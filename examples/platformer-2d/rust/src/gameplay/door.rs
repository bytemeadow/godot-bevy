use crate::components::{Door, Player};
use crate::level_manager::LoadLevelMessage;
use bevy::prelude::*;
use godot_bevy::prelude::*;

pub struct DoorPlugin;

impl Plugin for DoorPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, detect_door_collisions);
    }
}

fn detect_door_collisions(
    doors: Query<(Entity, &Door)>,
    players: Query<Entity, With<Player>>,
    collisions: Collisions,
    mut commands: Commands,
) {
    for (door_entity, door) in doors.iter() {
        for &player_entity in collisions.colliding_with(door_entity) {
            if players.get(player_entity).is_ok() {
                commands.trigger(LoadLevelMessage {
                    level_id: door.level_id,
                });
            }
        }
    }
}
