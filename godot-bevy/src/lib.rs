#![allow(clippy::type_complexity)]
#![allow(clippy::needless_lifetimes)]

use bevy::app::{App, Plugin};

pub mod app;
pub mod bridge;
pub mod node_tree_view;
pub mod plugins;
pub mod prelude {
    pub use crate::bridge::*;
    pub use crate::node_tree_view::NodeTreeView;
    pub use crate::plugins::{
        assets::{GodotAssetsPlugin, GodotResource},
        audio::{
            AudioError, AudioManager, AudioManagerExt, GodotAudio, GodotAudioPlugin, SoundId,
            SoundSettings,
        },
        core::{
            ActionInput, CollisionEventReader, CollisionEventType, Collisions, FindEntityByNameExt,
            GodotCorePlugin, GodotSignal, Groups, KeyboardInput, MouseButtonInput, MouseMotion,
            PhysicsUpdate, SceneTreeEventReader, SceneTreeRef, SystemDeltaTimer, Transform2D,
            Transform3D, connect_godot_signal,
        },
        packed_scene::{GodotScene, PackedScenePlugin},
    };
    pub use godot::prelude as godot_prelude;
    pub use godot_bevy_macros::*;
}
pub mod utils;
pub mod watchers;

pub struct GodotPlugin;

impl Plugin for GodotPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(plugins::DefaultGodotPlugin);
    }
}
