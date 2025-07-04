#![allow(clippy::type_complexity)]
#![allow(clippy::needless_lifetimes)]

// Allow the macro to reference the crate externally even from within itself
extern crate self as godot_bevy;

use bevy::app::{App, Plugin};

pub mod app;
pub mod interop;
pub mod node_tree_view;
pub mod plugins;
pub mod prelude;
pub mod utils;
pub mod watchers;

// Re-export inventory to avoid requiring users to add it as a dependency
pub use inventory;

pub struct GodotPlugin;

impl Plugin for GodotPlugin {
    fn build(&self, app: &mut App) {
        // Only add minimal core functionality by default
        // Users must explicitly opt-in to additional features
        app.add_plugins(plugins::GodotCorePlugins);
    }
}
