use bevy::app::{App, Plugin};

pub mod assets;
pub mod audio;
pub mod core;
pub mod packed_scene;

// Re-export all plugins for convenience
pub use audio::GodotAudioPlugin;
pub use core::{
    BevyInputBridgePlugin, GodotCollisionsPlugin, GodotInputEventPlugin, GodotSignalsPlugin,
    GodotTransformsPlugin,
};
pub use packed_scene::GodotPackedScenePlugin;

/// Minimal core functionality required for Godot-Bevy integration.
/// This includes scene tree management, asset loading, and basic bridge components.
pub struct GodotCorePlugins;

impl Plugin for GodotCorePlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins(core::GodotBaseCorePlugin)
            .add_plugins(assets::GodotAssetsPlugin);
    }
}

/// All plugins bundled together for convenience - equivalent to the old DefaultGodotPlugin.
/// Use this if you want all functionality enabled by default.
pub struct GodotDefaultPlugins;

impl Plugin for GodotDefaultPlugins {
    fn build(&self, app: &mut App) {
        app.add_plugins(GodotCorePlugins)
            .add_plugins(GodotTransformsPlugin)
            .add_plugins(GodotCollisionsPlugin)
            .add_plugins(GodotSignalsPlugin)
            .add_plugins(GodotInputEventPlugin)
            .add_plugins(BevyInputBridgePlugin)
            .add_plugins(GodotAudioPlugin)
            .add_plugins(GodotPackedScenePlugin);
    }
}

/// @deprecated Use GodotDefaultPlugins instead
pub struct DefaultGodotPlugin;

impl Plugin for DefaultGodotPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(GodotDefaultPlugins);
    }
}
