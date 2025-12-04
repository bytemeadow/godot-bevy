#![allow(clippy::type_complexity)]
#![allow(clippy::needless_lifetimes)]

// Allow the macro to reference the crate externally even from within itself
extern crate self as godot_bevy;

use bevy_app::{App, Plugin};

pub mod app;
pub mod interop;
pub mod node_tree_view;
pub mod plugins;
pub mod prelude;
pub mod profiling;
pub mod utils;
pub mod watchers;

#[cfg(test)]
mod tests;

// Re-export BevyApp for testing and advanced usage
pub use app::{BEVY_INIT_FUNC, BevyApp};

// Re-export inventory to avoid requiring users to add it as a dependency
pub use inventory;

// Re-export paste to avoid requiring users to add it as a dependency for transform sync macros
pub use paste;

// Re-export bevy sub-crates for macro-generated code
// This allows macros to use $crate::bevy_ecs:: paths that work for both
// users who depend on individual sub-crates and users who depend on the main bevy crate
pub use bevy_app;
pub use bevy_ecs;
pub use bevy_transform;

pub struct GodotPlugin;

impl Plugin for GodotPlugin {
    fn build(&self, app: &mut App) {
        // Only add minimal core functionality by default
        // Users must explicitly opt-in to additional features
        app.add_plugins(plugins::GodotCorePlugins);
    }
}
