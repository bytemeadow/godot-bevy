#![allow(clippy::type_complexity)]
#![allow(clippy::needless_lifetimes)]

use bevy::app::{App, Plugin};

pub mod app;
pub mod autosync;
pub mod bridge;
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
        app.add_plugins(plugins::DefaultGodotPlugin);

        // Auto-register all discovered AutoSyncBundle plugins
        autosync::register_all_autosync_bundles(app);
    }
}
