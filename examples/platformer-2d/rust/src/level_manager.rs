use bevy::prelude::*;
use godot::classes::Node;
use godot::prelude::*;
use godot_bevy::plugins::scene_tree::{SceneTreeEvent, SceneTreeEventType};
use godot_bevy::prelude::*;

use crate::scene_management::SceneOperationEvent;

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

/// Resource for tracking level loading state (internal to level manager)
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
#[derive(Event)]
pub struct LoadLevelEvent {
    pub level_id: LevelId,
}

/// Event fired when level loading is complete
#[derive(Event)]
pub struct LevelLoadedEvent {
    pub level_id: LevelId,
}

pub struct LevelManagerPlugin;

impl Plugin for LevelManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentLevel>()
            .init_resource::<PendingLevel>()
            .init_resource::<LevelLoadingState>()
            .add_event::<LoadLevelEvent>()
            .add_event::<LevelLoadedEvent>()
            .add_systems(
                Update,
                (
                    handle_level_load_requests,
                    (handle_level_scene_change, ApplyDeferred).chain(),
                    emit_level_loaded_event_when_scene_ready,
                ),
            );
    }
}

/// System that handles level loading requests - loads the asset
fn handle_level_load_requests(
    mut loading_state: ResMut<LevelLoadingState>,
    mut current_level: ResMut<CurrentLevel>,
    mut load_events: EventReader<LoadLevelEvent>,
    asset_server: Res<AssetServer>,
) {
    for event in load_events.read() {
        info!("Loading level asset: {:?}", event.level_id);

        // Load the level scene through Bevy's asset system
        let level_handle: Handle<GodotResource> = asset_server.load(event.level_id.scene_path());

        // Track loading state separately from current level
        loading_state.loading_handle = Some(level_handle);

        // Update current level
        current_level.set(event.level_id);

        info!("Level asset loading started for: {:?}", event.level_id);
    }
}

/// System that handles actual scene changing once assets are loaded
fn handle_level_scene_change(
    current_level: Res<CurrentLevel>,
    mut loading_state: ResMut<LevelLoadingState>,
    mut pending_level: ResMut<PendingLevel>,
    mut scene_events: EventWriter<SceneOperationEvent>,
    mut assets: ResMut<Assets<GodotResource>>,
) {
    if let (Some(level_id), Some(handle)) = (current_level.level_id, &loading_state.loading_handle)
    {
        // Check if the asset is loaded
        if let Some(_godot_resource) = assets.get_mut(handle) {
            info!("Requesting level scene change: {:?}", level_id);

            // Request scene change through centralized scene management
            scene_events.write(SceneOperationEvent::change_to_packed(handle.clone()));

            // Do NOT emit LevelLoadedEvent here!
            pending_level.level_id = Some(level_id);

            info!("Level scene change requested for: {:?}", level_id);

            // Clear the loading handle since we've used it
            loading_state.loading_handle = None;
        }
        // If asset isn't loaded yet, we'll try again next frame
    }
}

fn emit_level_loaded_event_when_scene_ready(
    mut pending_level: ResMut<PendingLevel>,
    mut scene_tree_events: EventReader<SceneTreeEvent>,
    mut loaded_events: EventWriter<LevelLoadedEvent>,
) {
    if let Some(level_id) = pending_level.level_id {
        let expected_path = match level_id {
            LevelId::Level1 => "/root/Level1",
            LevelId::Level2 => "/root/Level2",
            LevelId::Level3 => "/root/Level3",
        };
        for event in scene_tree_events.read() {
            if let SceneTreeEventType::NodeAdded = event.event_type {
                let node_path = event.node.clone().get::<Node>().get_path().to_string();
                if node_path == expected_path {
                    loaded_events.write(LevelLoadedEvent { level_id });
                    pending_level.level_id = None;
                    break;
                }
            }
        }
    }
}
