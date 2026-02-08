//! Integration testing framework for godot-bevy projects
//!
//! This crate provides a testing framework for writing integration tests
//! that run inside Godot with full access to both Bevy ECS and Godot's runtime.
//!
//! # Quick Start
//!
//! 1. Add dependencies to your test crate's `Cargo.toml`:
//! ```toml
//! [package]
//! name = "my-game-tests"
//! edition = "2024"
//!
//! [lib]
//! crate-type = ["cdylib"]
//!
//! [dependencies]
//! godot = "0.4"
//! godot-bevy = "0.9"
//! godot-bevy-test = "0.9"
//! ```
//!
//! 2. Set up your test entry point in `src/lib.rs`:
//! ```ignore
//! use godot::init::{ExtensionLibrary, gdextension};
//! use godot_bevy_test::prelude::*;
//!
//! // Declare the test runner class for Godot
//! godot_bevy_test::declare_test_runner!();
//!
//! // Include your test modules
//! mod my_tests;
//!
//! #[gdextension(entry_symbol = my_game_tests)]
//! unsafe impl ExtensionLibrary for IntegrationTests {}
//! ```
//!
//! 3. Write tests using the `#[itest]` macro:
//! ```ignore
//! use godot_bevy_test::prelude::*;
//!
//! #[itest(async)]
//! fn test_player_spawns(ctx: &TestContext) -> godot::task::TaskHandle {
//!     godot::task::spawn(async move {
//!         let mut app = TestApp::new(&ctx, |app| {
//!             // Add your plugins
//!         }).await;
//!
//!         app.update().await;
//!         // assertions...
//!     })
//! }
//! ```
//!
//! 4. Set up a Godot project with `TestRunner.gd` and run tests headlessly.

pub mod bencher;
pub mod exit_code;
pub mod runner;
pub mod test_app;
pub mod test_helpers;

// Re-export plugin registries from runner module for macro access
#[doc(hidden)]
pub use runner::__GODOT_ASYNC_ITEST;
#[doc(hidden)]
pub use runner::__GODOT_BENCH;
#[doc(hidden)]
pub use runner::__GODOT_ITEST;

// Re-export core types
pub use runner::{AsyncRustTestCase, RustBenchmark, RustTestCase, TestRunnerImpl};
pub use runner::{await_frame, await_frames, await_physics_frame};
pub use test_app::TestApp;
pub use test_helpers::Counter;

// Re-export bencher types
pub use bencher::{BenchResult, metrics, run_benchmark};

// Re-export the macros
pub use godot_bevy_test_macros::{bench, itest};

/// Context passed to each test function
#[derive(Clone)]
pub struct TestContext {
    pub scene_tree: godot::obj::Gd<godot::classes::Node>,
}

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::test_app::TestApp;
    pub use crate::test_helpers::Counter;
    pub use crate::{TestContext, await_frame, await_frames, bench, itest};
}

/// Macro to declare the test runner GodotClass in user's crate
///
/// This creates the `IntegrationTests` class (or custom name) that Godot will instantiate.
/// Must be called once in your test crate's lib.rs.
///
/// # Example
/// ```ignore
/// // Default name (IntegrationTests)
/// godot_bevy_test::declare_test_runner!();
///
/// // Custom name
/// godot_bevy_test::declare_test_runner!(MyTestRunner);
/// ```
#[macro_export]
macro_rules! declare_test_runner {
    () => {
        $crate::declare_test_runner!(IntegrationTests);
    };
    ($name:ident) => {
        #[derive(::godot::register::GodotClass, Debug)]
        #[class(init)]
        pub struct $name {
            runner: $crate::TestRunnerImpl,
        }

        #[::godot::register::godot_api]
        impl $name {
            #[func]
            fn run_all_tests(&mut self, scene_tree: ::godot::obj::Gd<::godot::classes::Node>) {
                self.runner.run_all_tests(scene_tree);
            }

            #[func]
            fn run_all_benchmarks(&mut self, scene_tree: ::godot::obj::Gd<::godot::classes::Node>) {
                self.runner.run_all_benchmarks(scene_tree);
            }
        }
    };
}
