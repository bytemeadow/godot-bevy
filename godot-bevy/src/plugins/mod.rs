use bevy::app::{App, Plugin};

pub mod assets;
pub mod audio;
pub mod core;
pub mod packed_scene;

// Re-export for convenience
pub use audio::GodotAudioPlugin;

pub struct DefaultGodotPlugin;

impl Plugin for DefaultGodotPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(core::GodotCorePlugin)
            .add_plugins(assets::GodotAssetsPlugin)
            .add_plugins(audio::GodotAudioPlugin)
            .add_plugins(packed_scene::PackedScenePlugin);
    }
}
