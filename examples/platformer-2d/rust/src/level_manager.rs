use bevy::prelude::*;
use godot_bevy::prelude::*;

/// Simple level identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LevelId {
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

/// Resource that tracks the current level and loaded handles
#[derive(Resource, Default)]
pub struct CurrentLevel {
    pub level_id: Option<LevelId>,
    pub level_handle: Option<Handle<GodotResource>>,
}

/// Component marking entities that belong to the current level
/// Useful for cleanup when switching levels
#[derive(Component)]
pub struct LevelEntity;

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
            .add_event::<LoadLevelEvent>()
            .add_event::<LevelLoadedEvent>()
            .add_systems(
                Update,
                (handle_level_load_requests, handle_level_scene_change),
            );
    }
}

/// System that handles level loading requests - loads the asset
fn handle_level_load_requests(
    mut current_level: ResMut<CurrentLevel>,
    mut load_events: EventReader<LoadLevelEvent>,
    asset_server: Res<AssetServer>,
) {
    for event in load_events.read() {
        info!("Loading level asset: {:?}", event.level_id);
        
        // Load the level scene through Bevy's asset system
        let level_handle: Handle<GodotResource> = asset_server.load(event.level_id.scene_path());
        
        current_level.level_id = Some(event.level_id);
        current_level.level_handle = Some(level_handle);
        
        info!("Level asset loading started for: {:?}", event.level_id);
    }
}

/// System that handles actual scene changing once assets are loaded
fn handle_level_scene_change(
    mut current_level: ResMut<CurrentLevel>,
    mut loaded_events: EventWriter<LevelLoadedEvent>,
    mut scene_tree: SceneTreeRef,
    mut assets: ResMut<Assets<GodotResource>>,
) {
    if let (Some(level_id), Some(ref handle)) = (current_level.level_id, &current_level.level_handle) {
        // Check if the asset is loaded
        if let Some(godot_resource) = assets.get_mut(handle) {
            if let Some(packed_scene) = godot_resource.try_cast::<godot::classes::PackedScene>() {
                info!("Changing to level scene: {:?}", level_id);
                
                // Use change_scene_to_packed instead of change_scene_to_file
                let mut tree = scene_tree.get();
                tree.change_scene_to_packed(&packed_scene);
                
                loaded_events.write(LevelLoadedEvent {
                    level_id,
                });
                
                info!("Successfully changed to level: {:?}", level_id);
                
                // Clear the handle since we've used it
                current_level.level_handle = None;
            } else {
                warn!("Loaded resource is not a PackedScene for level: {:?}", level_id);
            }
        }
        // If asset isn't loaded yet, we'll try again next frame
    }
}
