use crate::components::{Door, Player};
use crate::level_manager::LoadLevelEvent;
use bevy::prelude::*;
use godot_bevy::prelude::Collisions;

pub struct DoorPlugin;

impl Plugin for DoorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                // Collision detection runs first and writes events
                detect_door_collisions,
            ),
        );
    }
}

/// System that detects door-player collisions and fires events
///
/// This system only handles collision detection and event firing,
/// allowing it to run in parallel with other collision detection systems.
fn detect_door_collisions(
    doors: Query<(&Door, &Collisions)>,
    players: Query<Entity, With<Player>>,
    mut load_level_events: EventWriter<LoadLevelEvent>,
) {
    for (door, collisions) in doors.iter() {
        for &player_entity in collisions.recent_collisions() {
            if players.get(player_entity).is_ok() {
                load_level_events.write(LoadLevelEvent {
                    level_id: door.level_id,
                });
            }
        }
    }
}
