//! Integration tests for godot-bevy projects.
//!
//! Tests live in the game crate behind an `itest` feature and run in that
//! crate's Godot project. The runner needs the godot-bevy addon and the
//! `BevyAppSingleton` autoload already used by the game.
//!
//! # Quick Start
//!
//! Add the optional dependency and feature to the game crate's `Cargo.toml`:
//! ```toml
//! [dependencies]
//! godot = "0.5"
//! godot-bevy = "0.11"
//! godot-bevy-test = { version = "0.11", optional = true }
//! bevy = { version = "0.19", default-features = false }
//!
//! [features]
//! itest = ["dep:godot-bevy-test", "godot-bevy-test/test-frame-signal"]
//! ```
//!
//! Register the test runner alongside the normal `#[bevy_app]` entry point:
//! ```no_run
//! #[cfg(feature = "itest")]
//! godot_bevy_test::declare_test_runner!();
//!
//! #[cfg(feature = "itest")]
//! mod itests;
//! # fn main() {}
//! ```
//!
//! Write asynchronous tests with an owned [`TestContext`]:
//! ```no_run
//! use godot_bevy_test::prelude::*;
//!
//! #[itest]
//! async fn test_player_spawns(ctx: TestContext) {
//!     let mut app = TestApp::new(&ctx, |_app| {}).await;
//!     app.update().await;
//!     app.cleanup().await;
//! }
//! # fn main() {}
//! ```
//!
//! The explicit alternative is `#[itest(async)] fn test(ctx: &TestContext) ->
//! godot::task::TaskHandle`, returning `godot::task::spawn(async move { ... })`.

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
/// Must be called once in your game crate's lib.rs.
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
