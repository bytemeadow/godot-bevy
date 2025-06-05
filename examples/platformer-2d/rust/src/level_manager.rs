use bevy::prelude::*;

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
            LevelId::Level1 => "res://scenes/levels/level_1.tscn",
            LevelId::Level2 => "res://scenes/levels/level_2.tscn", 
            LevelId::Level3 => "res://scenes/levels/level_3.tscn",
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

/// Resource that tracks the current level
#[derive(Resource, Default)]
pub struct CurrentLevel {
    pub level_id: Option<LevelId>,
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

/// Event fired when level unloading is complete
#[derive(Event)]
pub struct LevelUnloadedEvent {
    pub level_id: LevelId,
}

pub struct LevelManagerPlugin;

impl Plugin for LevelManagerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CurrentLevel>()
            .add_event::<LoadLevelEvent>()
            .add_event::<LevelLoadedEvent>()
            .add_event::<LevelUnloadedEvent>()
            .add_systems(
                Update,
                handle_level_load_requests,
            );
    }
}

/// System that handles level loading requests
fn handle_level_load_requests(
    mut current_level: ResMut<CurrentLevel>,
    mut load_events: EventReader<LoadLevelEvent>,
    mut loaded_events: EventWriter<LevelLoadedEvent>,
) {
    for event in load_events.read() {
        info!("Received request to load level: {:?}", event.level_id);
        info!("Scene path would be: {}", event.level_id.scene_path());
        
        // TODO: Implement actual scene loading here
        // For now just track the level change

        
        
        current_level.level_id = Some(event.level_id);
        
        loaded_events.send(LevelLoadedEvent {
            level_id: event.level_id,
        });
    }
}

/// System that handles level unloading requests
fn handle_level_unloading_requests(
    mut current_level: ResMut<CurrentLevel>,
    mut unload_events: EventReader<LevelUnloadedEvent>,
) {
    for event in unload_events.read() {
        info!("Unloading level: {:?}", event.level_id);
        current_level.level_id = None;
    }
} 