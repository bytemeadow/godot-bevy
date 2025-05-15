use bevy::app::{App, Plugin};

pub mod app;
pub mod bridge;
pub mod node_tree_view;
pub mod plugins;
pub mod prelude;
pub mod watchers;

pub struct GodotPlugin;

impl Plugin for GodotPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(plugins::DefaultGodotPlugin);
    }
}
