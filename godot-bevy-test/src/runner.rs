//! Test runner implementation for godot-bevy integration tests

use godot::builtin::{Callable, Signal};
use godot::classes::object::ConnectFlags;
use godot::classes::{Engine, Node};
use godot::obj::{Gd, Singleton};
use godot::task::has_godot_task_panicked;
use std::cell::RefCell;
use std::collections::HashSet;
use std::rc::Rc;
use std::time::Instant;

use crate::TestContext;
use crate::bencher;
use crate::exit_code::write_exit_code;

// Plugin registries - defined here so plugin_foreach! can access them as simple identifiers
godot::sys::plugin_registry!(pub __GODOT_ITEST: RustTestCase);
godot::sys::plugin_registry!(pub __GODOT_ASYNC_ITEST: AsyncRustTestCase);
godot::sys::plugin_registry!(pub __GODOT_BENCH: RustBenchmark);

/// Represents a single sync test case
#[derive(Copy, Clone)]
pub struct RustTestCase {
    pub name: &'static str,
    pub file: &'static str,
    pub skipped: bool,
    pub focused: bool,
    pub line: u32,
    pub function: fn(&TestContext),
}

/// Represents a single async test case
#[derive(Copy, Clone)]
pub struct AsyncRustTestCase {
    pub name: &'static str,
    pub file: &'static str,
    pub skipped: bool,
    pub focused: bool,
    pub line: u32,
    pub function: fn(&TestContext) -> godot::task::TaskHandle,
}

/// Represents a single benchmark
#[derive(Copy, Clone)]
pub struct RustBenchmark {
    pub name: &'static str,
    pub file: &'static str,
    pub line: u32,
    pub function: fn(),
    pub repetitions: usize,
}

/// The test runner implementation that does the actual work
/// This is used by the `declare_test_runner!` macro
#[derive(Default, Debug)]
pub struct TestRunnerImpl {}

impl TestRunnerImpl {
    pub fn new() -> Self {
        Self {}
    }

    /// Run all registered async tests
    pub fn run_all_tests(&mut self, scene_tree: Gd<Node>) {
        println!("\n{FMT_CYAN_BOLD}Run{FMT_END} godot-bevy async integration tests...");

        let tests = self.collect_tests();

        if tests.focus_run {
            println!("  {FMT_CYAN}Focused run{FMT_END} -- execute only selected tests.");
        }

        println!(
            "  Found {} async tests in {} files.",
            tests.test_count, tests.file_count
        );

        let clock = Instant::now();
        let ctx = TestContext { scene_tree };

        // Start async test execution - will call quit when done
        self.run_async_tests(tests.tests, ctx, clock);
    }

    /// Run all registered benchmarks
    pub fn run_all_benchmarks(&mut self, _scene_tree: Gd<Node>) {
        println!("\n\n{FMT_CYAN_BOLD}Run{FMT_END} godot-bevy benchmarks...");

        // Check for debug builds and warn
        let rust_debug = cfg!(debug_assertions);
        let godot_debug = godot::classes::Os::singleton().is_debug_build();

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

        let (benchmarks, file_count) = self.collect_benchmarks();
        println!(
            "  Rust: found {} benchmarks in {} files.",
            benchmarks.len(),
            file_count
        );

        // Print header
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

    fn collect_tests(&self) -> CollectedTests {
        let mut all_files = HashSet::new();
        let mut tests = Vec::new();
        let mut is_focus_run = false;

        godot::sys::plugin_foreach!(__GODOT_ASYNC_ITEST; |test: &AsyncRustTestCase| {
            // Switch to focused mode if we encounter a focused test
            if !is_focus_run && test.focused {
                tests.clear();
                all_files.clear();
                is_focus_run = true;
            }

            // Only collect if normal mode or (focus mode and test is focused)
            if !is_focus_run || test.focused {
                all_files.insert(test.file);
                tests.push(*test);
            }
        });

        // Sort for deterministic order
        tests.sort_by_key(|test| (test.file, test.line));

        let test_count = tests.len();
        let file_count = all_files.len();

        CollectedTests {
            tests,
            test_count,
            file_count,
            focus_run: is_focus_run,
        }
    }

    fn collect_benchmarks(&self) -> (Vec<RustBenchmark>, usize) {
        let mut all_files = HashSet::new();
        let mut benchmarks = Vec::new();

        godot::sys::plugin_foreach!(__GODOT_BENCH; |bench: &RustBenchmark| {
            benchmarks.push(*bench);
            all_files.insert(bench.file);
        });

        // Sort for deterministic order
        benchmarks.sort_by_key(|bench| (bench.file, bench.line));

        (benchmarks, all_files.len())
    }

    fn run_async_tests(
        &self,
        tests: Vec<AsyncRustTestCase>,
        ctx: TestContext,
        start_time: Instant,
    ) {
        // Shared state for test execution
        let state = Rc::new(RefCell::new(TestRunState {
            passed: 0,
            skipped: 0,
            failed_list: Vec::new(),
        }));

        // Start with the first test
        run_next_test(0, tests, ctx, state, start_time);
    }

    fn run_rust_benchmarks(&self, benchmarks: Vec<RustBenchmark>) {
        // Check if we should output JSON (for CI)
        let output_json = std::env::var("BENCHMARK_JSON").is_ok();

        let mut results = Vec::new();
        let mut last_file = None;

        for bench in benchmarks {
            // Print file header if different from last (human-readable mode)
            if !output_json && last_file.as_deref() != Some(bench.file) {
                if last_file.is_some() {
                    println!();
                }
                println!("{}:{}", bench.file, bench.line);
                last_file = Some(bench.file.to_string());
            }

            // Print benchmark name (human-readable mode)
            if !output_json {
                print!("  {:58}", bench.name);
                std::io::Write::flush(&mut std::io::stdout()).ok();
            }

            // Run the benchmark
            let result = bencher::run_benchmark(bench.function, bench.repetitions);

            // Store result for JSON output
            results.push((bench.name, result.stats[0], result.stats[1])); // min, median

            // Print results (human-readable mode)
            if !output_json {
                for stat in result.stats {
                    print!(" {stat:>12.2?}");
                }
                println!();
            }
        }

        // Output JSON if requested
        if output_json {
            output_json_results(results);
        }
    }
}

struct CollectedTests {
    tests: Vec<AsyncRustTestCase>,
    test_count: usize,
    file_count: usize,
    focus_run: bool,
}

#[derive(Clone)]
struct TestRunState {
    passed: usize,
    skipped: usize,
    failed_list: Vec<String>,
}

// Free functions for async test execution
fn run_next_test(
    index: usize,
    tests: Vec<AsyncRustTestCase>,
    ctx: TestContext,
    state: Rc<RefCell<TestRunState>>,
    start_time: Instant,
) {
    // All tests done?
    if index >= tests.len() {
        finish_test_run(tests.len(), state, start_time, &ctx);
        return;
    }

    let test = &tests[index];

    // Skip test?
    if test.skipped {
        println!("  {} ... {}[SKIP]{}", test.name, FMT_YELLOW, FMT_END);
        state.borrow_mut().skipped += 1;
        run_next_test(index + 1, tests, ctx, state, start_time);
        return;
    }

    print!("  {} ... ", test.name);
    std::io::Write::flush(&mut std::io::stdout()).ok();

    // Run the test
    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| (test.function)(&ctx)));

    match result {
        Ok(task_handle) => {
            // Wait for task to complete
            check_async_test(
                task_handle,
                test.name.to_string(),
                index,
                tests,
                ctx,
                state,
                start_time,
            );
        }
        Err(e) => {
            let msg = if let Some(s) = e.downcast_ref::<String>() {
                s.clone()
            } else if let Some(s) = e.downcast_ref::<&str>() {
                s.to_string()
            } else {
                "unknown panic".to_string()
            };

            println!("{FMT_RED}FAILED{FMT_END}");
            println!("    {msg}");
            state.borrow_mut().failed_list.push(test.name.to_string());
            run_next_test(index + 1, tests, ctx, state, start_time);
        }
    }
}

fn check_async_test(
    task_handle: godot::task::TaskHandle,
    test_name: String,
    index: usize,
    tests: Vec<AsyncRustTestCase>,
    ctx: TestContext,
    state: Rc<RefCell<TestRunState>>,
    start_time: Instant,
) {
    if !task_handle.is_pending() {
        // Task completed
        if has_godot_task_panicked(task_handle) {
            println!("{FMT_RED}FAILED{FMT_END}");
            state.borrow_mut().failed_list.push(test_name);
        } else {
            println!("{FMT_GREEN}ok{FMT_END}");
            state.borrow_mut().passed += 1;
        }

        // Continue to next test
        run_next_test(index + 1, tests, ctx, state, start_time);
        return;
    }

    // Still pending - check again next frame
    // Need to wrap in Option to move out of FnMut closure
    let mut task_opt = Some(task_handle);
    let next_ctx = ctx.clone();

    let deferred = Callable::from_fn("check_async_test", move |_| {
        check_async_test(
            task_opt
                .take()
                .expect("Callable should only be called once"),
            test_name.clone(),
            index,
            tests.clone(),
            next_ctx.clone(),
            state.clone(),
            start_time,
        );
        godot::builtin::Variant::nil()
    });

    let mut tree = ctx.scene_tree.get_tree().expect("Scene tree should exist");
    tree.connect_flags("process_frame", &deferred, ConnectFlags::ONE_SHOT);
}

fn finish_test_run(
    total: usize,
    state: Rc<RefCell<TestRunState>>,
    start_time: Instant,
    ctx: &TestContext,
) {
    let state = state.borrow();
    let elapsed = start_time.elapsed();
    let failed_count = total - state.passed - state.skipped;

    println!();
    println!("{FMT_CYAN_BOLD}Test result:{FMT_END}");
    print!("  ");

    if state.passed > 0 {
        print!("{}{} passed{}", FMT_GREEN, state.passed, FMT_END);
    }

    if failed_count > 0 {
        if state.passed > 0 {
            print!(", ");
        }
        print!("{FMT_RED}{failed_count} failed{FMT_END}");
    }

    if state.skipped > 0 {
        if state.passed > 0 || failed_count > 0 {
            print!(", ");
        }
        print!("{} skipped", state.skipped);
    }

    println!(" in {:.2}s", elapsed.as_secs_f32());

    if !state.failed_list.is_empty() {
        println!();
        println!("{FMT_RED}Failed tests:{FMT_END}");
        for name in &state.failed_list {
            println!("  - {name}");
        }
    }

    let success = failed_count == 0;

    if success {
        println!("{FMT_GREEN}All tests passed!{FMT_END}");
    }

    // Exit with appropriate code (cross-platform)
    let exit_code: i32 = if success { 0 } else { 1 };
    write_exit_code(exit_code);

    // Now quit
    ctx.scene_tree.get_tree().expect("tree").quit();
}

fn output_json_results(results: Vec<(&str, std::time::Duration, std::time::Duration)>) {
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
            "godot_debug": godot::classes::Os::singleton().is_debug_build(),
        }
    });

    // Write to file
    if let Ok(path) = std::env::var("BENCHMARK_JSON_PATH")
        && let Ok(file) = std::fs::File::create(path)
    {
        let _ = serde_json::to_writer_pretty(file, &output);
    }

    // Also output to stdout with special markers for parsing
    println!("===BENCHMARK_JSON_START===");
    println!(
        "{}",
        serde_json::to_string_pretty(&output).unwrap_or_default()
    );
    println!("===BENCHMARK_JSON_END===");
}

// ANSI color codes for terminal output
const FMT_CYAN_BOLD: &str = "\x1b[36;1m";
const FMT_CYAN: &str = "\x1b[36m";
const FMT_GREEN: &str = "\x1b[32m";
const FMT_YELLOW: &str = "\x1b[33m";
const FMT_RED: &str = "\x1b[31m";
const FMT_END: &str = "\x1b[0m";

/// Helper function to wait for the next Godot process frame
pub async fn await_frame() {
    let tree = Engine::singleton()
        .get_main_loop()
        .expect("Main loop should exist")
        .cast::<godot::classes::SceneTree>();

    let signal = Signal::from_object_signal(&tree, "process_frame");
    let _: () = signal.to_future().await;
}

/// Helper function to wait for multiple frames
pub async fn await_frames(count: u32) {
    for _ in 0..count {
        await_frame().await;
    }
}
