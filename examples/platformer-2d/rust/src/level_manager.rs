use bevy::prelude::*;
use godot::classes::Node;
use godot::prelude::*;
use godot_bevy::interop::signal_names::SceneTreeSignals;
use godot_bevy::plugins::scene_tree::{SceneTreeMessage, SceneTreeMessageType};
use godot_bevy::prelude::*;

use crate::scene_management::SceneOperationMessage;

#[derive(Event, Debug, Clone)]
struct SceneChanged;

/// Simple level identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, GodotConvert, Var, Export)]
#[godot(via = GString)]
pub enum LevelId {
    #[default]
    Level1,
    Level2,
    Level3,
}

impl LevelId {
    /// Get the Godot scene path for this level
    pub fn scene_path(&self) -> &'static str {
        match self {
            LevelId::Level1 => "scenes/levels/level_1.tscn",
            LevelId::Level2 => "scenes/levels/level_2.tscn",
            LevelId::Level3 => "scenes/levels/level_3.tscn",
        }
    }

    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            LevelId::Level1 => "Level 1",
            LevelId::Level2 => "Level 2",
            LevelId::Level3 => "Level 3",
        }
    }
}

/// Resource that tracks the current active level (read-mostly for game state)
#[derive(Resource, Default)]
pub struct CurrentLevel {
    pub level_id: Option<LevelId>,
}

impl CurrentLevel {
    /// Set the current level
    pub fn set(&mut self, level_id: LevelId) {
        self.level_id = Some(level_id);
    }

    /// Clear the current level state
    pub fn clear(&mut self) {
        self.level_id = None;
    }
}

#[derive(Resource, Default)]
struct LevelLoadingState {
    pub loading_handle: Option<Handle<GodotResource>>,
}

/// Resource that tracks the pending level
#[derive(Resource, Default)]
pub struct PendingLevel {
    pub level_id: Option<LevelId>,
}

/// Event fired when a level load is requested
#[derive(Event, Debug, Clone)]
pub struct LoadLevelMessage {
    pub level_id: LevelId,
}

/// Event fired when level loading is complete
#[derive(Event, Debug, Clone)]
pub struct LevelLoadedMessage {
    pub level_id: LevelId,
}

pub struct LevelManagerPlugin;

impl Plugin for LevelManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentLevel>()
            .init_resource::<PendingLevel>()
            .init_resource::<LevelLoadingState>()
            .init_resource::<SceneTreeSignalConnected>()
            .add_plugins(GodotSignalsPlugin::<SceneChanged>::default())
            .add_observer(on_load_level_request)
            .add_observer(on_scene_changed)
            .add_systems(Startup, connect_scene_tree_signal)
            .add_systems(
                Update,
                (
                    (handle_level_scene_change, ApplyDeferred).chain(),
                    emit_level_loaded_event_when_scene_ready,
                ),
            );
    }
}

#[derive(Resource, Default)]
struct SceneTreeSignalConnected(bool);

fn connect_scene_tree_signal(
    mut connected: ResMut<SceneTreeSignalConnected>,
    signals: GodotSignals<SceneChanged>,
    mut scene_tree: SceneTreeRef,
) {
    if connected.0 {
        return;
    }

    let tree = scene_tree.get().clone();
    signals.connect_object(tree, SceneTreeSignals::SCENE_CHANGED, |_args| {
        Some(SceneChanged)
    });
    connected.0 = true;

    info!("Connected to SceneTree.scene_changed signal");
}

fn on_scene_changed(_trigger: On<SceneChanged>) {
    info!("Scene changed!");
}

fn on_load_level_request(
    trigger: On<LoadLevelMessage>,
    mut loading_state: ResMut<LevelLoadingState>,
    mut current_level: ResMut<CurrentLevel>,
    asset_server: Res<AssetServer>,
) {
    let event = trigger.event();
    info!("Loading level asset: {:?}", event.level_id);

    let level_handle: Handle<GodotResource> = asset_server.load(event.level_id.scene_path());

    loading_state.loading_handle = Some(level_handle);

    current_level.set(event.level_id);

    info!("Level asset loading started for: {:?}", event.level_id);
}

fn handle_level_scene_change(
    current_level: Res<CurrentLevel>,
    mut loading_state: ResMut<LevelLoadingState>,
    mut pending_level: ResMut<PendingLevel>,
    mut scene_events: MessageWriter<SceneOperationMessage>,
    mut assets: ResMut<Assets<GodotResource>>,
) {
    if let (Some(level_id), Some(handle)) = (current_level.level_id, &loading_state.loading_handle)
        && assets.get_mut(handle).is_some()
    {
        info!("Requesting level scene change: {:?}", level_id);

        scene_events.write(SceneOperationMessage::change_to_packed(handle.clone()));

        pending_level.level_id = Some(level_id);

        info!("Level scene change requested for: {:?}", level_id);

        loading_state.loading_handle = None;
    }
}

fn emit_level_loaded_event_when_scene_ready(
    mut pending_level: ResMut<PendingLevel>,
    mut scene_tree_events: MessageReader<SceneTreeMessage>,
    mut commands: Commands,
    mut godot: GodotAccess,
) {
    if let Some(level_id) = pending_level.level_id {
        let expected_path = match level_id {
            LevelId::Level1 => "/root/Level1",
            LevelId::Level2 => "/root/Level2",
            LevelId::Level3 => "/root/Level3",
        };
        for event in scene_tree_events.read() {
            if let SceneTreeMessageType::NodeAdded = event.message_type
                && let Some(node) = godot.try_get::<Node>(event.node_id)
                && node.is_inside_tree()
            {
                let node_path = node.get_path().to_string();
                if node_path == expected_path {
                    commands.trigger(LevelLoadedMessage { level_id });
                    pending_level.level_id = None;
                    break;
                }
            }
        }
    }
}
