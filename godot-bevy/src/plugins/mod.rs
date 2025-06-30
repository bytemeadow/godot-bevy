use bevy::app::plugin_group;

pub mod assets;
pub mod audio;
pub mod core;
pub mod packed_scene;

// Re-export all plugins for convenience
pub use audio::GodotAudioPlugin;
pub use core::{
    BevyInputBridgePlugin, GodotCollisionsPlugin, GodotInputEventPlugin,
    GodotSceneTreeEventsPlugin, GodotSceneTreeMirroringPlugin, GodotSceneTreeRefPlugin,
    GodotSignalsPlugin, GodotTransformSyncPlugin,
};
pub use packed_scene::GodotPackedScenePlugin;

plugin_group! {
    /// Minimal core functionality required for Godot-Bevy integration.
    /// This includes scene tree management, asset loading, and basic bridge components.
    pub struct GodotCorePlugins {
        core:::GodotBaseCorePlugin,
        assets:::GodotAssetsPlugin
    }
}

plugin_group! {
    /// This plugin group will add all the default plugins for a *godot-bevy* application:
    pub struct GodotDefaultPlugins {
        core:::GodotCollisionsPlugin,
        core:::GodotSignalsPlugin,
        core:::GodotInputEventPlugin,
        core:::BevyInputBridgePlugin,
        audio:::GodotAudioPlugin,
        packed_scene:::GodotPackedScenePlugin
    }
}
