use bevy::app::plugin_group;

pub mod assets;
pub mod audio;
pub mod collisions;
pub mod core;
pub mod input;
pub mod input_bridge;
pub mod packed_scene;
pub mod scene_tree;
pub mod signals;
pub mod transforms;

// Re-export all plugins for convenience
pub use audio::GodotAudioPlugin;
pub use collisions::GodotCollisionsPlugin;
pub use core::GodotBaseCorePlugin;
pub use input::GodotInputPlugin;
pub use input_bridge::BevyInputBridgePlugin;
pub use packed_scene::GodotPackedScenePlugin;
pub use scene_tree::GodotSceneTreePlugin;
pub use signals::GodotSignalsPlugin;
pub use transforms::GodotTransformSyncPlugin;

plugin_group! {
    /// Minimal core functionality required for Godot-Bevy integration.
    /// This includes scene tree management, asset loading, and basic bridge components.
    pub struct GodotCorePlugins {
        :GodotBaseCorePlugin,
        assets:::GodotAssetsPlugin
    }
}

plugin_group! {
    /// This plugin group will add all the default plugins for a *godot-bevy* application:
    pub struct GodotDefaultPlugins {
        :GodotCollisionsPlugin,
        :GodotSignalsPlugin,
        :GodotInputPlugin,
        :BevyInputBridgePlugin,
        audio:::GodotAudioPlugin,
        packed_scene:::GodotPackedScenePlugin
    }
}
