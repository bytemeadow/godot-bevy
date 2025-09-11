pub use crate::GodotPlugin;
pub use crate::interop::*;
pub use crate::node_tree_view::NodeTreeView;
#[cfg(feature = "godot_bevy_log")]
pub use crate::plugins::godot_bevy_logger::GodotBevyLogPlugin;
pub use crate::plugins::{
    // Signals
    GodotCorePlugins,
    // Transforms
    GodotDefaultPlugins,
    assets::{GodotAssetsPlugin, GodotResource},
    audio::{
        Audio, AudioApp, AudioChannel, AudioChannelMarker, AudioEasing, AudioError, AudioOutput,
        AudioPlayerType, AudioSettings, AudioTween, GodotAudioChannels, GodotAudioPlugin,
        MainAudioTrack, PlayAudioCommand, SoundId,
    },
    collisions::{
        AREA_ENTERED, AREA_EXITED, BODY_ENTERED, BODY_EXITED, COLLISION_START_SIGNALS,
        CollisionEvent, CollisionEventType, Collisions, GodotCollisionsPlugin,
    },
    core::{FindEntityByNameExt, MainThreadMarker, PhysicsDelta, PhysicsUpdate},
    // Collisions
    input::{
        ActionInput, BevyInputBridgePlugin, GodotInputEventPlugin, KeyboardInput, MouseButtonInput,
        MouseMotion,
    },
    // Core functionality
    packed_scene::{GodotPackedScenePlugin, GodotScene},
    // Input
    scene_tree::{
        AutoSyncBundleRegistry, GodotSceneTreePlugin, Groups, SceneTreeConfig, SceneTreeRef,
    },
    signals::{GodotTypedSignalsPlugin, TypedGodotSignals},
    // Scene tree
    transforms::{
        GodotTransformConfig, GodotTransformSyncPlugin, GodotTransformSyncPluginExt,
        TransformSyncMetadata, TransformSyncMode, add_transform_sync_systems,
    },
};

// Legacy re-exports (deprecated). Keep available for downstreams while avoiding warnings here.
#[allow(deprecated)]
pub use crate::plugins::signals::{
    GodotSignal, GodotSignals, GodotSignalsPlugin, connect_godot_signal,
};
pub use bevy::prelude as bevy_prelude;
pub use godot::prelude as godot_prelude;
pub use godot_bevy_macros::*;
