use crate::components::Gem;
use crate::components::Player;
use bevy::prelude::*;
use godot::{
    classes::{Area2D, IArea2D},
    prelude::*,
};
use godot_bevy::prelude::Collisions;
use godot_bevy::prelude::*;

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
        app.add_plugins(Gem2DBundleAutoSyncPlugin)
            .add_systems(Update, hide_gem_on_player_collision);
    }
}

/// System that hides the gem when the player enters its Area2D
fn hide_gem_on_player_collision(
    mut gems: Query<(&mut GodotNodeHandle, &Collisions), With<Gem>>,
    players: Query<Entity, With<Player>>,
) {
    for (mut handle, collisions) in gems.iter_mut() {
        for &entity in collisions.recent_collisions() {
            if players.get(entity).is_ok() {
                handle.get::<Area2D>().set_visible(false);
                // TODO: Play audio or send event to play audio
            }
        }
    }
}
