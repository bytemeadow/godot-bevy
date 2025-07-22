use bevy::app::plugin_group;
#[cfg(feature = "bevy_gamepad")]
use bevy::gilrs::GilrsPlugin;

pub mod assets;
pub mod audio;
pub mod collisions;
pub mod core;
pub mod input;
pub mod packed_scene;
pub mod scene_tree;
pub mod signals;
pub mod transforms;

// Re-export all plugins for convenience
pub use assets::GodotAssetsPlugin;
pub use audio::GodotAudioPlugin;
pub use collisions::GodotCollisionsPlugin;
pub use core::GodotBaseCorePlugin;
pub use input::{BevyInputBridgePlugin, GodotInputEventPlugin};
pub use packed_scene::GodotPackedScenePlugin;
pub use scene_tree::GodotSceneTreePlugin;
pub use signals::GodotSignalsPlugin;
pub use transforms::GodotTransformSyncPlugin;

// Re-export for backwards compatibility
#[deprecated(note = "Use GodotInputEventPlugin instead")]
pub use input::GodotInputEventPlugin as GodotInputPlugin;

plugin_group! {
    /// Minimal core functionality required for Godot-Bevy integration.
    /// This includes scene tree management and basic bridge components.
    pub struct GodotCorePlugins {
        :GodotBaseCorePlugin,
        :GodotSceneTreePlugin,
    }
}

plugin_group! {
    /// This plugin group will add all the default plugins for a *godot-bevy* application:
    pub struct GodotDefaultPlugins {
        :GodotAssetsPlugin,
        :GodotCollisionsPlugin,
        :GodotSignalsPlugin,
        :BevyInputBridgePlugin,
        :GodotAudioPlugin,
        :GodotPackedScenePlugin,
        :GodotTransformSyncPlugin,
        #[cfg(feature = "bevy_gamepad")]
        :GilrsPlugin,
    }
}
