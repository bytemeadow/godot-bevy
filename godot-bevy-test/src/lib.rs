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
//! ```no_run
//! use godot::init::{ExtensionLibrary, gdextension};
//! use godot_bevy_test::prelude::*;
//!
//! godot_bevy_test::declare_test_runner!();
//!
//! mod my_tests {}
//!
//! #[gdextension(entry_symbol = my_game_tests)]
//! unsafe impl ExtensionLibrary for IntegrationTests {}
//! # fn main() {}
//! ```
//!
//! 3. Write tests using the `#[itest]` macro:
//! ```no_run
//! use godot_bevy_test::prelude::*;
//!
//! #[itest(async)]
//! fn test_player_spawns(ctx: &TestContext) -> godot::task::TaskHandle {
//!     let ctx = ctx.clone();
//!     godot::task::spawn(async move {
//!         let mut app = TestApp::new(&ctx, |_app| {}).await;
//!
//!         app.update().await;
//!     })
//! }
//! # fn main() {}
//! ```
//!
//! 4. Set up a Godot project with `TestRunner.gd` and run tests headlessly.

pub mod bencher;
mod config;
pub mod exit_code;
#[cfg(feature = "profile-tracy")]
#[doc(hidden)]
pub mod profiling;
mod report;
pub mod runner;
mod selection;
pub mod test_app;
pub mod test_helpers;

#[doc(hidden)]
pub use runner::__GODOT_ASYNC_ITEST;
#[doc(hidden)]
pub use runner::__GODOT_BENCH;
#[doc(hidden)]
pub use runner::__GODOT_ITEST;

#[cfg(feature = "test-frame-signal")]
pub use runner::await_bevy_frame;
pub use runner::{AsyncRustTestCase, RustBenchmark, RustTestCase, TestRunnerImpl};
pub use runner::{await_frame, await_frames, await_physics_frame};
pub use test_app::TestApp;
pub use test_helpers::Counter;

pub use bencher::{BenchResult, measured, metrics, run_benchmark};

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
/// ```no_run
/// # mod default_name {
/// godot_bevy_test::declare_test_runner!();
///
/// # }
/// # mod custom_name {
/// godot_bevy_test::declare_test_runner!(MyTestRunner);
/// # }
/// # fn main() {}
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
            fn run_all_benchmarks(
                &mut self,
                scene_tree: ::godot::obj::Gd<::godot::classes::Node>,
            ) -> i32 {
                self.runner.run_all_benchmarks(scene_tree)
            }
        }
    };
}
