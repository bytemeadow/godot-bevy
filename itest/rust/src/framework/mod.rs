/*
 * Test framework for godot-bevy integration tests
 * All tests are async and wait for actual Godot frame progression
 */

use godot::classes::Node;
use godot::obj::{EngineBitfield, Gd};
use godot::register::{GodotClass, godot_api};
use std::time::Instant;

use godot::builtin::Signal;
use godot::classes::Engine;

// Plugin registry for async tests only
godot::sys::plugin_registry!(pub(crate) __GODOT_ASYNC_ITEST: AsyncRustTestCase);

/// Context passed to each test
#[derive(Clone)]
pub struct TestContext {
    pub scene_tree: Gd<Node>,
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

/// Main test runner class exposed to Godot
#[derive(GodotClass, Debug)]
#[class(init)]
pub struct IntegrationTests {}

#[godot_api]
impl IntegrationTests {
    /// Run all registered async tests
    /// This starts the async test execution and returns immediately
    /// Tests will complete asynchronously and call get_tree().quit() when done
    #[func]
    fn run_all_tests(&mut self, scene_tree: Gd<Node>) {
        println!(
            "\n{}Run{} godot-bevy async integration tests...",
            FMT_CYAN_BOLD, FMT_END
        );

        let tests = self.collect_tests();

        if tests.focus_run {
            println!(
                "  {}Focused run{} -- execute only selected tests.",
                FMT_CYAN, FMT_END
            );
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
}

impl IntegrationTests {
    fn collect_tests(&self) -> CollectedTests {
        let mut all_files = std::collections::HashSet::new();
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

    fn run_async_tests(
        &self,
        tests: Vec<AsyncRustTestCase>,
        ctx: TestContext,
        start_time: Instant,
    ) {
        use std::cell::RefCell;
        use std::rc::Rc;

        // Shared state for test execution
        let state = Rc::new(RefCell::new(TestRunState {
            passed: 0,
            skipped: 0,
            failed_list: Vec::new(),
        }));

        // Start with the first test
        run_next_test(0, tests, ctx, state, start_time);
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
    state: std::rc::Rc<std::cell::RefCell<TestRunState>>,
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

            println!("{}FAILED{}", FMT_RED, FMT_END);
            println!("    {}", msg);
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
    state: std::rc::Rc<std::cell::RefCell<TestRunState>>,
    start_time: Instant,
) {
    use godot::builtin::Callable;
    use godot::classes::object::ConnectFlags;
    use godot::task::has_godot_task_panicked;

    if !task_handle.is_pending() {
        // Task completed
        if has_godot_task_panicked(task_handle) {
            println!("{}FAILED{}", FMT_RED, FMT_END);
            state.borrow_mut().failed_list.push(test_name);
        } else {
            println!("{}ok{}", FMT_GREEN, FMT_END);
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

    let deferred = Callable::from_local_fn("check_async_test", move |_| {
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
        Ok(godot::builtin::Variant::nil())
    });

    let mut tree = ctx.scene_tree.get_tree().expect("Scene tree should exist");
    tree.connect_ex("process_frame", &deferred)
        .flags(ConnectFlags::ONE_SHOT.ord() as u32)
        .done();
}

fn finish_test_run(
    total: usize,
    state: std::rc::Rc<std::cell::RefCell<TestRunState>>,
    start_time: Instant,
    ctx: &TestContext,
) {
    let state = state.borrow();
    let elapsed = start_time.elapsed();
    let failed_count = total - state.passed - state.skipped;

    println!();
    println!("{}Test result:{}", FMT_CYAN_BOLD, FMT_END);
    print!("  ");

    if state.passed > 0 {
        print!("{}{} passed{}", FMT_GREEN, state.passed, FMT_END);
    }

    if failed_count > 0 {
        if state.passed > 0 {
            print!(", ");
        }
        print!("{}{} failed{}", FMT_RED, failed_count, FMT_END);
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
        println!("{}Failed tests:{}", FMT_RED, FMT_END);
        for name in &state.failed_list {
            println!("  - {}", name);
        }
    }

    let success = failed_count == 0;

    if success {
        println!("{}All tests passed!{}", FMT_GREEN, FMT_END);
    }

    // Exit with appropriate code
    let exit_code: i32 = if success { 0 } else { 1 };

    // Write exit code to a file for the wrapper script to read
    if let Err(e) = std::fs::write("/tmp/godot_test_exit_code", exit_code.to_string()) {
        eprintln!("Warning: Failed to write exit code file: {}", e);
    }

    // Now quit
    ctx.scene_tree.get_tree().expect("tree").quit();
}

// ANSI color codes for terminal output
const FMT_CYAN_BOLD: &str = "\x1b[36;1m";
const FMT_CYAN: &str = "\x1b[36m";
const FMT_GREEN: &str = "\x1b[32m";
const FMT_YELLOW: &str = "\x1b[33m";
const FMT_RED: &str = "\x1b[31m";
const FMT_END: &str = "\x1b[0m";

/// Helper function to wait for the next Godot process frame
/// This allows async tests to yield control back to Godot's frame loop
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

pub mod test_helpers;
pub use test_helpers::*;

pub mod test_app;
pub use test_app::TestApp;
