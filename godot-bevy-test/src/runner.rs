//! Test runner implementation for godot-bevy integration tests

use godot::builtin::{Callable, Signal};
use godot::classes::object::ConnectFlags;
use godot::classes::{Engine, Node, Os};
use godot::obj::{Gd, Singleton};
use godot::task::has_godot_task_panicked;
use std::any::Any;
use std::cell::RefCell;
use std::collections::HashSet;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::rc::Rc;
use std::sync::Once;
use std::time::{Duration, Instant};

use crate::TestContext;
use crate::bencher;
use crate::config::{Filter, TestConfig, filter_from_env, report_path_from_env};
use crate::exit_code::write_exit_code;
use crate::report::{
    Attempt, AttemptOutcome, Failure, LogicalTest, ReportWriter, RunOutcome, Selection,
    TestOutcome, TestReport, test_id,
};

godot::sys::shard_registry!(pub __GODOT_ITEST: RustTestCase);
godot::sys::shard_registry!(pub __GODOT_ASYNC_ITEST: AsyncRustTestCase);
godot::sys::shard_registry!(pub __GODOT_BENCH: RustBenchmark);

#[derive(Copy, Clone)]
pub struct RustTestCase {
    pub name: &'static str,
    pub file: &'static str,
    pub skipped: bool,
    pub focused: bool,
    pub line: u32,
    pub function: fn(&TestContext),
}

#[derive(Copy, Clone)]
pub struct AsyncRustTestCase {
    pub name: &'static str,
    pub file: &'static str,
    pub skipped: bool,
    pub focused: bool,
    pub line: u32,
    pub function: fn(&TestContext) -> godot::task::TaskHandle,
}

#[derive(Copy, Clone)]
pub struct RustBenchmark {
    pub name: &'static str,
    pub file: &'static str,
    pub line: u32,
    pub function: fn(),
    pub repetitions: usize,
}

#[derive(Copy, Clone)]
enum TestFunction {
    Sync(fn(&TestContext)),
    Async(fn(&TestContext) -> godot::task::TaskHandle),
}

#[derive(Copy, Clone)]
struct RegisteredTest {
    name: &'static str,
    file: &'static str,
    skipped: bool,
    focused: bool,
    line: u32,
    function: TestFunction,
}

#[derive(Default, Debug)]
pub struct TestRunnerImpl {}

impl TestRunnerImpl {
    pub fn new() -> Self {
        Self {}
    }

    pub fn run_all_tests(&mut self, scene_tree: Gd<Node>) {
        println!("\n{FMT_CYAN_BOLD}Run{FMT_END} godot-bevy integration tests...");
        install_panic_capture_hook();

        let ctx = TestContext { scene_tree };
        let report_path = match report_path_from_env() {
            Ok(path) => path,
            Err(error) => {
                eprintln!("{FMT_RED}Configuration error: {error}{FMT_END}");
                terminate(&ctx, 2);
                return;
            }
        };

        let all_tests = self.collect_registered_tests();
        let config = match TestConfig::from_env() {
            Ok(config) => config,
            Err(error) => {
                let fallback = TestConfig::fallback();
                let selection = Selection {
                    registered: all_tests.len(),
                    selected: 0,
                    focus_run: all_tests.iter().any(|test| test.focused),
                    filter: std::env::var("ITEST_FILTER").ok(),
                    patterns: Vec::new(),
                };
                finish_configuration_error(&ctx, &fallback, selection, report_path, error);
                return;
            }
        };

        let collected = select_registered_tests(all_tests, config.filter.as_ref());
        print_test_configuration(&config, &collected);

        let selection = Selection {
            registered: collected.registered,
            selected: collected.tests.len(),
            focus_run: collected.focus_run,
            filter: config
                .filter
                .as_ref()
                .map(|filter| filter.normalized.clone()),
            patterns: config
                .filter
                .as_ref()
                .map(|filter| filter.patterns.clone())
                .unwrap_or_default(),
        };
        let mut report = TestReport::new(&config, selection, godot_version());
        let writer = ReportWriter::new(config.json_path.clone());

        if let Err(error) = writer.write(&report) {
            eprintln!("{FMT_RED}Failed to initialize integration test report: {error}{FMT_END}");
            terminate(&ctx, 2);
            return;
        }

        if collected.focus_run && config.deny_focus {
            report.finish_error(
                "configuration",
                "focused integration tests are forbidden by ITEST_DENY_FOCUS".to_string(),
            );
            finish_early_report(&ctx, &writer, &report);
            return;
        }

        if collected.tests.is_empty() {
            report.finish_error(
                "configuration",
                "integration test selection matched zero tests".to_string(),
            );
            finish_early_report(&ctx, &writer, &report);
            return;
        }

        let state = Rc::new(RefCell::new(TestRunState {
            tests: collected.tests,
            ctx,
            config,
            report,
            writer,
            started: Instant::now(),
            test_index: 0,
            current_attempts: Vec::new(),
        }));
        run_next_attempt(state);
    }

    pub fn run_all_benchmarks(&mut self, scene_tree: Gd<Node>) {
        println!("\n\n{FMT_CYAN_BOLD}Run{FMT_END} godot-bevy benchmarks...");

        let rust_debug = cfg!(debug_assertions);
        let godot_debug = Os::singleton().is_debug_build();

        println!(
            "  Rust build: {}",
            if rust_debug {
                format!("{FMT_YELLOW}debug{FMT_END}")
            } else {
                format!("{FMT_GREEN}release{FMT_END}")
            }
        );

        if rust_debug || godot_debug {
            print!("  {FMT_YELLOW}Warning: ");
            match (rust_debug, godot_debug) {
                (true, true) => println!("Both Rust and Godot are debug builds"),
                (true, false) => println!("Rust is a debug build"),
                (false, true) => println!("Godot is a debug build"),
                _ => {}
            }
            println!("  For accurate benchmarks, use release builds{FMT_END}");
        }

        let (benchmarks, file_count) = match self.collect_benchmarks() {
            Ok(collected) => collected,
            Err(error) => {
                godot::global::godot_error!("{error}");
                scene_tree.get_tree().quit_ex().exit_code(2).done();
                return;
            }
        };
        println!(
            "  Rust: found {} benchmarks in {} files.",
            benchmarks.len(),
            file_count
        );

        if let Ok(filter) = std::env::var("BENCHMARK_FILTER") {
            println!("  Filter: {FMT_CYAN}{filter}{FMT_END}");
        }

        print!("\n{FMT_CYAN}");
        print!("{:60}", "");
        for metric in bencher::metrics() {
            print!("{metric:>13}");
        }
        println!("{FMT_END}");

        let clock = Instant::now();
        self.run_rust_benchmarks(benchmarks);
        let elapsed = clock.elapsed();

        println!("\nBenchmarks completed in {:.2}s.", elapsed.as_secs_f32());
    }

    fn collect_registered_tests(&self) -> Vec<RegisteredTest> {
        let mut tests = Vec::new();

        godot::sys::shard_foreach!(__GODOT_ITEST; |test: &RustTestCase| {
            tests.push(RegisteredTest {
                name: test.name,
                file: test.file,
                skipped: test.skipped,
                focused: test.focused,
                line: test.line,
                function: TestFunction::Sync(test.function),
            });
        });
        godot::sys::shard_foreach!(__GODOT_ASYNC_ITEST; |test: &AsyncRustTestCase| {
            tests.push(RegisteredTest {
                name: test.name,
                file: test.file,
                skipped: test.skipped,
                focused: test.focused,
                line: test.line,
                function: TestFunction::Async(test.function),
            });
        });

        tests
    }

    fn collect_benchmarks(&self) -> Result<(Vec<RustBenchmark>, usize), String> {
        let filter = filter_from_env("BENCHMARK_FILTER")?;
        let mut all_files = HashSet::new();
        let mut benchmarks = Vec::new();

        godot::sys::shard_foreach!(__GODOT_BENCH; |bench: &RustBenchmark| {
            let matches = filter.as_ref().is_none_or(|filter| {
                filter
                    .patterns
                    .iter()
                    .any(|pattern| bench.name.contains(pattern))
            });
            if matches {
                benchmarks.push(*bench);
                all_files.insert(bench.file);
            }
        });

        benchmarks.sort_by_key(|bench| (bench.file, bench.line));
        Ok((benchmarks, all_files.len()))
    }

    fn run_rust_benchmarks(&self, benchmarks: Vec<RustBenchmark>) {
        let output_json = std::env::var("BENCHMARK_JSON").is_ok();

        let mut results = Vec::new();
        let mut last_file = None;

        for bench in benchmarks {
            if !output_json && last_file.as_deref() != Some(bench.file) {
                if last_file.is_some() {
                    println!();
                }
                println!("{}:{}", bench.file, bench.line);
                last_file = Some(bench.file.to_string());
            }

            if !output_json {
                print!("  {:58}", bench.name);
                std::io::Write::flush(&mut std::io::stdout()).ok();
            }

            let result = bencher::run_benchmark(bench.function, bench.repetitions);
            results.push((bench.name, result.stats[0], result.stats[1]));

            if !output_json {
                for stat in result.stats {
                    print!(" {stat:>12.2?}");
                }
                println!();
            }
        }

        if output_json {
            output_json_results(results);
        }
    }
}

struct CollectedTests {
    tests: Vec<RegisteredTest>,
    registered: usize,
    file_count: usize,
    focus_run: bool,
}

fn select_registered_tests(
    mut tests: Vec<RegisteredTest>,
    filter: Option<&Filter>,
) -> CollectedTests {
    let registered = tests.len();
    let focus_run = tests.iter().any(|test| test.focused);

    tests.retain(|test| {
        (!focus_run || test.focused)
            && filter.is_none_or(|filter| {
                filter
                    .patterns
                    .iter()
                    .any(|pattern| test.name.contains(pattern))
            })
    });
    tests.sort_by_key(|test| (test.file, test.line));

    let file_count = tests
        .iter()
        .map(|test| test.file)
        .collect::<HashSet<_>>()
        .len();

    CollectedTests {
        tests,
        registered,
        file_count,
        focus_run,
    }
}

fn print_test_configuration(config: &TestConfig, tests: &CollectedTests) {
    println!(
        "  Rust build: {}",
        if cfg!(debug_assertions) {
            format!("{FMT_YELLOW}debug{FMT_END}")
        } else {
            format!("{FMT_GREEN}release{FMT_END}")
        }
    );
    if tests.focus_run {
        println!("  {FMT_CYAN}Focused run{FMT_END} -- execute only selected tests.");
    }
    if let Some(filter) = &config.filter {
        println!("  Filter: {}", filter.normalized);
    }
    println!("  Repeat: {}", config.repeat);
    println!("  Timeout: {} frames", config.timeout_frames);
    println!(
        "  Found {} selected tests in {} files ({} registered).",
        tests.tests.len(),
        tests.file_count,
        tests.registered
    );
}

struct TestRunState {
    tests: Vec<RegisteredTest>,
    ctx: TestContext,
    config: TestConfig,
    report: TestReport,
    writer: ReportWriter,
    started: Instant,
    test_index: usize,
    current_attempts: Vec<Attempt>,
}

enum NextAction {
    Finish,
    Checkpoint,
    Run {
        test: RegisteredTest,
        attempt_index: u32,
        repeat: u32,
        ctx: TestContext,
    },
}

fn run_next_attempt(state: Rc<RefCell<TestRunState>>) {
    let action = {
        let mut state = state.borrow_mut();

        if state.test_index >= state.tests.len() {
            NextAction::Finish
        } else {
            let test = state.tests[state.test_index];
            if test.skipped {
                println!("  {} ... {}[SKIP]{}", test.name, FMT_YELLOW, FMT_END);
                let result = LogicalTest::skipped(test.name, test.file, test.line);
                let elapsed = state.started.elapsed();
                state.report.push_test(result, elapsed);
                state.test_index += 1;
                NextAction::Checkpoint
            } else if state.current_attempts.len() == state.config.repeat as usize {
                let attempts = std::mem::take(&mut state.current_attempts);
                let result = LogicalTest::from_attempts(test.name, test.file, test.line, attempts);
                if result.outcome == TestOutcome::Flaky {
                    println!("    {FMT_RED}flaky across repeated attempts{FMT_END}");
                }
                let elapsed = state.started.elapsed();
                state.report.push_test(result, elapsed);
                state.test_index += 1;
                NextAction::Checkpoint
            } else {
                NextAction::Run {
                    test,
                    attempt_index: state.current_attempts.len() as u32 + 1,
                    repeat: state.config.repeat,
                    ctx: state.ctx.clone(),
                }
            }
        }
    };

    match action {
        NextAction::Finish => finish_test_run(state),
        NextAction::Checkpoint => checkpoint_and_continue(state),
        NextAction::Run {
            test,
            attempt_index,
            repeat,
            ctx,
        } => run_attempt(state, test, attempt_index, repeat, ctx),
    }
}

fn run_attempt(
    state: Rc<RefCell<TestRunState>>,
    test: RegisteredTest,
    attempt_index: u32,
    repeat: u32,
    ctx: TestContext,
) {
    begin_attempt_capture();
    if repeat == 1 {
        print!("  {} ... ", test.name);
    } else {
        print!("  {} [{attempt_index}/{repeat}] ... ", test.name);
    }
    std::io::Write::flush(&mut std::io::stdout()).ok();
    let started = Instant::now();

    match test.function {
        TestFunction::Sync(function) => {
            let result = catch_unwind(AssertUnwindSafe(|| function(&ctx)));
            let captured = end_attempt_capture();
            let mut failures = drain_frame_failures();
            if let Err(payload) = result {
                failures.insert(0, failure_from_payload(payload.as_ref(), captured));
            }
            finish_attempt(state, test, attempt_index, started.elapsed(), failures);
        }
        TestFunction::Async(function) => match catch_unwind(AssertUnwindSafe(|| function(&ctx))) {
            Ok(task) => check_async_attempt(task, state, test, attempt_index, started, 0),
            Err(payload) => {
                let captured = end_attempt_capture();
                let mut failures = drain_frame_failures();
                failures.insert(0, failure_from_payload(payload.as_ref(), captured));
                finish_attempt(state, test, attempt_index, started.elapsed(), failures);
            }
        },
    }
}

fn check_async_attempt(
    task: godot::task::TaskHandle,
    state: Rc<RefCell<TestRunState>>,
    test: RegisteredTest,
    attempt_index: u32,
    started: Instant,
    frames: u32,
) {
    if !task.is_pending() {
        let task_panicked = has_godot_task_panicked(task);
        let captured = end_attempt_capture();
        let mut failures = drain_frame_failures();

        if task_panicked {
            if captured.is_empty() && failures.is_empty() {
                failures.push(Failure {
                    kind: "panic".to_string(),
                    message: "async task panicked; payload unavailable".to_string(),
                    location: None,
                    gdext_context: None,
                    callback: None,
                });
            } else {
                let mut captured: Vec<_> = captured.into_iter().map(failure_from_capture).collect();
                captured.append(&mut failures);
                failures = captured;
            }
        }

        finish_attempt(state, test, attempt_index, started.elapsed(), failures);
        return;
    }

    let timeout_frames = state.borrow().config.timeout_frames;
    if frames >= timeout_frames {
        task.cancel();
        drop(end_attempt_capture());
        let mut failures = drain_frame_failures();
        failures.push(Failure {
            kind: "timeout".to_string(),
            message: format!("attempt exceeded {timeout_frames} frames"),
            location: None,
            gdext_context: None,
            callback: None,
        });
        finish_attempt(state, test, attempt_index, started.elapsed(), failures);
        return;
    }

    let mut task = Some(task);
    let ctx = state.borrow().ctx.clone();
    let deferred = Callable::from_fn("check_async_attempt", move |_| {
        check_async_attempt(
            task.take().expect("Callable should only be called once"),
            state.clone(),
            test,
            attempt_index,
            started,
            frames + 1,
        );
        godot::builtin::Variant::nil()
    });

    ctx.scene_tree
        .get_tree()
        .connect_flags("process_frame", &deferred, ConnectFlags::ONE_SHOT);
}

fn finish_attempt(
    state: Rc<RefCell<TestRunState>>,
    test: RegisteredTest,
    attempt_index: u32,
    duration: Duration,
    failures: Vec<Failure>,
) {
    let attempt = Attempt::new(
        &test_id(test.file, test.name),
        attempt_index,
        duration,
        failures,
    );
    if attempt.outcome == AttemptOutcome::Pass {
        println!("{FMT_GREEN}ok{FMT_END}");
    } else {
        println!("{FMT_RED}FAILED{FMT_END}");
        for failure in &attempt.failures {
            println!("    {}: {}", failure.kind, failure.message);
        }
    }
    state.borrow_mut().current_attempts.push(attempt);
    schedule_next_attempt(state);
}

fn checkpoint_and_continue(state: Rc<RefCell<TestRunState>>) {
    let result = {
        let state = state.borrow();
        state.writer.write(&state.report)
    };
    if let Err(error) = result {
        let ctx = state.borrow().ctx.clone();
        eprintln!("{FMT_RED}Failed to checkpoint integration test report: {error}{FMT_END}");
        terminate(&ctx, 2);
        return;
    }
    schedule_next_attempt(state);
}

fn schedule_next_attempt(state: Rc<RefCell<TestRunState>>) {
    let ctx = state.borrow().ctx.clone();
    let mut state = Some(state);
    let callable = Callable::from_fn("run_next_attempt", move |_| {
        run_next_attempt(state.take().expect("Callable should only be called once"));
        godot::builtin::Variant::nil()
    });
    ctx.scene_tree
        .get_tree()
        .connect_flags("process_frame", &callable, ConnectFlags::ONE_SHOT);
}

fn finish_test_run(state: Rc<RefCell<TestRunState>>) {
    let (ctx, exit_code, output) = {
        let mut state = state.borrow_mut();
        let elapsed = state.started.elapsed();
        state.report.finish(elapsed);
        print_test_summary(&state.report, elapsed);
        let exit_code = if state.report.outcome == RunOutcome::Pass {
            0
        } else {
            1
        };
        let output = state.writer.write(&state.report);
        (state.ctx.clone(), exit_code, output)
    };

    match output {
        Ok(Some(json)) => print_json_report(&json),
        Ok(None) => {}
        Err(error) => {
            eprintln!("{FMT_RED}Failed to finalize integration test report: {error}{FMT_END}");
            terminate(&ctx, 2);
            return;
        }
    }
    terminate(&ctx, exit_code);
}

fn print_test_summary(report: &TestReport, elapsed: Duration) {
    println!();
    println!("{FMT_CYAN_BOLD}Test result:{FMT_END}");
    println!(
        "  {} passed, {} failed, {} flaky, {} skipped in {:.2}s",
        report.summary.passed,
        report.summary.failed,
        report.summary.flaky,
        report.summary.skipped,
        elapsed.as_secs_f32()
    );

    let mut failed = report
        .tests
        .iter()
        .filter(|test| matches!(test.outcome, TestOutcome::Fail | TestOutcome::Flaky))
        .peekable();
    if failed.peek().is_some() {
        println!();
        println!("{FMT_RED}Failed tests:{FMT_END}");
        for test in failed {
            println!("  - {} ({:?})", test.name, test.outcome);
        }
    } else {
        println!("{FMT_GREEN}All tests passed!{FMT_END}");
    }
}

fn finish_configuration_error(
    ctx: &TestContext,
    config: &TestConfig,
    selection: Selection,
    report_path: Option<std::path::PathBuf>,
    error: String,
) {
    eprintln!("{FMT_RED}Configuration error: {error}{FMT_END}");
    let mut report = TestReport::new(config, selection, godot_version());
    let writer = ReportWriter::new(report_path);
    if let Err(write_error) = writer.write(&report) {
        eprintln!("{FMT_RED}Failed to initialize integration test report: {write_error}{FMT_END}");
        terminate(ctx, 2);
        return;
    }
    report.finish_error("configuration", error);
    finish_early_report(ctx, &writer, &report);
}

fn finish_early_report(ctx: &TestContext, writer: &ReportWriter, report: &TestReport) {
    match writer.write(report) {
        Ok(Some(json)) => print_json_report(&json),
        Ok(None) => {}
        Err(error) => {
            eprintln!("{FMT_RED}Failed to finalize integration test report: {error}{FMT_END}");
        }
    }
    terminate(ctx, 2);
}

fn print_json_report(json: &str) {
    println!("===ITEST_JSON_START===");
    println!("{json}");
    println!("===ITEST_JSON_END===");
}

fn terminate(ctx: &TestContext, exit_code: i32) {
    write_exit_code(exit_code);
    ctx.scene_tree.get_tree().quit();
}

fn godot_version() -> String {
    let version = Engine::singleton().get_version_info();
    let major: i64 = version.at("major").to();
    let minor: i64 = version.at("minor").to();
    let patch: i64 = version.at("patch").to();
    let status: godot::builtin::GString = version.at("status").to();
    format!("{major}.{minor}.{patch}.{status}")
}

struct CapturedPanic {
    message: String,
    location: Option<String>,
    gdext_context: Option<String>,
}

thread_local! {
    static ACTIVE_PANICS: RefCell<Option<Vec<CapturedPanic>>> = const { RefCell::new(None) };
}

fn install_panic_capture_hook() {
    static INSTALL: Once = Once::new();
    INSTALL.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            ACTIVE_PANICS.with(|active| {
                if let Ok(mut active) = active.try_borrow_mut()
                    && let Some(records) = active.as_mut()
                {
                    records.push(CapturedPanic {
                        message: panic_message(info.payload()),
                        location: info.location().map(|location| {
                            format!(
                                "{}:{}:{}",
                                location.file(),
                                location.line(),
                                location.column()
                            )
                        }),
                        gdext_context: godot::private::fetch_last_panic_context(),
                    });
                }
            });
            previous(info);
        }));
    });
}

fn begin_attempt_capture() {
    ACTIVE_PANICS.with(|active| *active.borrow_mut() = Some(Vec::new()));
    drop(drain_frame_failures());
}

fn end_attempt_capture() -> Vec<CapturedPanic> {
    ACTIVE_PANICS.with(|active| active.borrow_mut().take().unwrap_or_default())
}

fn failure_from_payload(payload: &(dyn Any + Send), captured: Vec<CapturedPanic>) -> Failure {
    let message = panic_message(payload);
    let (location, gdext_context) = captured
        .into_iter()
        .rev()
        .find(|record| record.message == message)
        .map(|record| (record.location, record.gdext_context))
        .unwrap_or_default();
    Failure {
        kind: "panic".to_string(),
        message,
        location,
        gdext_context,
        callback: None,
    }
}

fn failure_from_capture(captured: CapturedPanic) -> Failure {
    Failure {
        kind: "panic".to_string(),
        message: captured.message,
        location: captured.location,
        gdext_context: captured.gdext_context,
        callback: None,
    }
}

fn panic_message(payload: &(dyn Any + Send)) -> String {
    if let Some(message) = payload.downcast_ref::<String>() {
        message.clone()
    } else if let Some(message) = payload.downcast_ref::<&str>() {
        message.to_string()
    } else {
        "unknown panic payload".to_string()
    }
}

#[cfg(feature = "test-frame-signal")]
fn drain_frame_failures() -> Vec<Failure> {
    godot_bevy::app::drain_test_frame_panics()
        .into_iter()
        .map(|(callback, message)| Failure {
            kind: "panic".to_string(),
            message,
            location: None,
            gdext_context: None,
            callback: Some(callback.to_string()),
        })
        .collect()
}

#[cfg(not(feature = "test-frame-signal"))]
fn drain_frame_failures() -> Vec<Failure> {
    Vec::new()
}

fn output_json_results(results: Vec<(&str, Duration, Duration)>) {
    use std::collections::HashMap;

    let mut benchmarks = HashMap::new();

    for (name, min, median) in results {
        let mut entry = HashMap::new();
        entry.insert("min_ns", min.as_nanos().to_string());
        entry.insert("median_ns", median.as_nanos().to_string());
        entry.insert("min_display", format!("{min:.2?}"));
        entry.insert("median_display", format!("{median:.2?}"));

        benchmarks.insert(name.to_string(), entry);
    }

    let output = serde_json::json!({
        "benchmarks": benchmarks,
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "environment": {
            "rust_debug": cfg!(debug_assertions),
            "godot_debug": Os::singleton().is_debug_build(),
        }
    });

    if let Ok(path) = std::env::var("BENCHMARK_JSON_PATH")
        && let Ok(file) = std::fs::File::create(path)
    {
        let _ = serde_json::to_writer_pretty(file, &output);
    }

    println!("===BENCHMARK_JSON_START===");
    println!(
        "{}",
        serde_json::to_string_pretty(&output).unwrap_or_default()
    );
    println!("===BENCHMARK_JSON_END===");
}

const FMT_CYAN_BOLD: &str = "\x1b[36;1m";
const FMT_CYAN: &str = "\x1b[36m";
const FMT_GREEN: &str = "\x1b[32m";
const FMT_YELLOW: &str = "\x1b[33m";
const FMT_RED: &str = "\x1b[31m";
const FMT_END: &str = "\x1b[0m";

/// Helper function to wait for the next Godot process frame.
///
/// The `process_frame` signal fires after all `_physics_process()` calls but
/// before `_process()` for that frame, so the suffix (Update/PostUpdate/Last)
/// has not yet run when this resolves. The Main prefix
/// (First/PreUpdate/StateTransition) + FixedMain have run too -- except on a
/// 0-physics-step frame, where the prefix runs in the `_process` fallback after
/// this fires. The itest harness pins `--fixed-fps 60` (one step per frame), so
/// the prefix has always run there.
pub async fn await_frame() {
    let tree = Engine::singleton()
        .get_main_loop()
        .expect("Main loop should exist")
        .cast::<godot::classes::SceneTree>();

    let signal = Signal::from_object_signal(&tree, "process_frame");
    let _: () = signal.to_future().await;
}

/// Helper function to wait for the next Godot physics frame.
///
/// Waits for the `physics_frame` signal, which fires immediately before
/// `_physics_process()` runs. Use this when you need to guarantee that
/// a physics tick has occurred (e.g., collision processing).
pub async fn await_physics_frame() {
    let tree = Engine::singleton()
        .get_main_loop()
        .expect("Main loop should exist")
        .cast::<godot::classes::SceneTree>();

    let signal = Signal::from_object_signal(&tree, "physics_frame");
    let _: () = signal.to_future().await;
}

/// Helper function to wait for multiple frames
pub async fn await_frames(count: u32) {
    for _ in 0..count {
        await_frame().await;
    }
}

/// Wait for the BevyApp to finish a full render frame (suffix + clear_trackers).
/// Returns the number of physics steps that ran that frame. Requires the
/// `test-frame-signal` feature.
#[cfg(feature = "test-frame-signal")]
pub async fn await_bevy_frame(app: &Gd<godot_bevy::BevyApp>) -> i64 {
    let signal = Signal::from_object_signal(app, "bevy_frame_complete");
    let args = signal.to_future::<(i64,)>().await;
    args.0
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sync_test(_: &TestContext) {}

    fn async_test(_: &TestContext) -> godot::task::TaskHandle {
        unreachable!()
    }

    fn registered(
        name: &'static str,
        file: &'static str,
        line: u32,
        focused: bool,
        skipped: bool,
        function: TestFunction,
    ) -> RegisteredTest {
        RegisteredTest {
            name,
            file,
            skipped,
            focused,
            line,
            function,
        }
    }

    #[test]
    fn selection_contract() {
        let mixed = select_registered_tests(
            vec![
                registered(
                    "async_second",
                    "src/b.rs",
                    10,
                    false,
                    false,
                    TestFunction::Async(async_test),
                ),
                registered(
                    "sync_first",
                    "src/a.rs",
                    20,
                    false,
                    false,
                    TestFunction::Sync(sync_test),
                ),
            ],
            None,
        );
        assert_eq!(
            mixed.tests.iter().map(|test| test.name).collect::<Vec<_>>(),
            ["sync_first", "async_second"]
        );

        let all = vec![
            registered(
                "normal_sync",
                "src/z.rs",
                4,
                false,
                false,
                TestFunction::Sync(sync_test),
            ),
            registered(
                "focused_async_first",
                "src/a.rs",
                20,
                true,
                false,
                TestFunction::Async(async_test),
            ),
            registered(
                "focused_async_skipped",
                "src/a.rs",
                30,
                true,
                true,
                TestFunction::Async(async_test),
            ),
        ];
        let filter = Filter {
            normalized: "focused_async".to_string(),
            patterns: vec!["focused_async".to_string()],
        };
        let selected = select_registered_tests(all, Some(&filter));

        assert_eq!(selected.registered, 3);
        assert!(selected.focus_run);
        assert_eq!(selected.tests.len(), 2);
        assert_eq!(selected.tests[0].name, "focused_async_first");
        assert_eq!(selected.tests[1].name, "focused_async_skipped");
        assert!(selected.tests[1].skipped);

        let no_match = Filter {
            normalized: "FOCUSED".to_string(),
            patterns: vec!["FOCUSED".to_string()],
        };
        let selected = select_registered_tests(
            vec![registered(
                "focused_async_first",
                "src/a.rs",
                20,
                true,
                false,
                TestFunction::Async(async_test),
            )],
            Some(&no_match),
        );
        assert!(selected.tests.is_empty());
        assert!(selected.focus_run);
    }
}
