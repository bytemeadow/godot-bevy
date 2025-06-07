use crate::components::Gem;
use crate::components::Player;
use crate::gameplay::audio::PlaySfxEvent;
use bevy::prelude::*;
use godot::{
    classes::{Area2D, IArea2D},
    prelude::*,
};
use godot_bevy::prelude::Collisions;
use godot_bevy::prelude::*;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default, Resource)]
pub struct GemsCollected(pub i64);

#[derive(GodotClass, BevyBundle)]
#[class(base=Area2D)]
#[bevy_bundle((Gem))]
pub struct Gem2D {
    base: Base<Area2D>,
}

#[godot_api]
impl IArea2D for Gem2D {
    fn init(base: Base<Area2D>) -> Self {
        Self { base }
    }
}

pub struct GemPlugin;

impl Plugin for GemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GemsCollected>()
            .add_plugins(Gem2DBundleAutoSyncPlugin)
            .add_systems(Update, hide_gem_on_player_collision);
    }
}

fn hide_gem_on_player_collision(
    mut gems: Query<(&mut GodotNodeHandle, &Collisions), With<Gem>>,
    players: Query<Entity, With<Player>>,
    mut gems_collected: ResMut<GemsCollected>,
    mut sfx_events: EventWriter<PlaySfxEvent>,
) {
    for (mut handle, collisions) in gems.iter_mut() {
        for &entity in collisions.recent_collisions() {
            if players.get(entity).is_ok() {
                if let Some(mut area) = handle.try_get::<Area2D>() {
                    area.queue_free();
                    gems_collected.0 += 1;
                    sfx_events.write(PlaySfxEvent::GemCollected);
                }
            }
        }
    }
}
