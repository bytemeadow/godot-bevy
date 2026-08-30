use crate::components::Gem;
use crate::components::Player;
use crate::gameplay::audio::PlaySfxMessage;
use crate::gameplay::hud::HudUpdateMessage;
use bevy::prelude::*;
use godot::classes::Area2D;
use godot_bevy::prelude::*;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default, Resource)]
pub struct GemsCollected(pub i64);

pub struct GemPlugin;

impl Plugin for GemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GemsCollected>()
            .add_observer(on_collision_started);
    }
}

fn on_collision_started(
    trigger: On<CollisionStarted>,
    gems: Query<&GodotNodeHandle, With<Gem>>,
    players: Query<(), With<Player>>,
    mut gems_collected: ResMut<GemsCollected>,
    mut commands: Commands,
    mut godot: GodotAccess,
) {
    let event = trigger.event();

    let (gem_entity, gem_handle) = if let Ok(handle) = gems.get(event.entity1) {
        if players.get(event.entity2).is_ok() {
            (event.entity1, handle)
        } else {
            return;
        }
    } else if let Ok(handle) = gems.get(event.entity2) {
        if players.get(event.entity1).is_ok() {
            (event.entity2, handle)
        } else {
            return;
        }
    } else {
        return;
    };

    if let Some(mut area) = godot.try_get::<Area2D>(*gem_handle) {
        area.queue_free();
    }

    // Despawn the entity to prevent duplicate processing
    commands.entity(gem_entity).despawn();

    gems_collected.0 += 1;

    commands.trigger(PlaySfxMessage::GemCollected);
    commands.trigger(HudUpdateMessage::GemsChanged(gems_collected.0));

    debug!("Gem collected! Total: {}", gems_collected.0);
}
