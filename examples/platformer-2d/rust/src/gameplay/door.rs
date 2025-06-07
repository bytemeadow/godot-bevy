use crate::components::{Door, Player};
use crate::level_manager::{LevelId, LoadLevelEvent};
use bevy::prelude::*;
use godot::{
    classes::{Area2D, IArea2D},
    prelude::*,
};
use godot_bevy::prelude::Collisions;
use godot_bevy::prelude::*;

#[derive(GodotClass, BevyBundle)]
#[class(base=Area2D)]
#[bevy_bundle((Door: level))]
pub struct Door2D {
    base: Base<Area2D>,
    #[export]
    level: LevelId,
}

#[godot_api]
impl IArea2D for Door2D {
    fn init(base: Base<Area2D>) -> Self {
        Self {
            base,
            level: LevelId::Level1,
        }
    }
}

pub struct DoorPlugin;

impl Plugin for DoorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Door2DBundleAutoSyncPlugin)
            .add_systems(Update, handle_player_door_collision);
    }
}

fn handle_player_door_collision(
    doors: Query<(&Door, &Collisions)>,
    players: Query<Entity, With<Player>>,
    mut load_level_events: EventWriter<LoadLevelEvent>,
) {
    for (door, collisions) in doors.iter() {
        for &entity in collisions.recent_collisions() {
            if players.get(entity).is_ok() {
                load_level_events.write(LoadLevelEvent { level_id: door.0 });
            }
        }
    }
}
