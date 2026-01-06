use crate::components::Gem;
use crate::components::Player;
use crate::gameplay::audio::PlaySfxMessage;
use bevy::prelude::*;
use godot::classes::Area2D;
use godot_bevy::prelude::*;

/// Event fired when a gem is collected by the player
///
/// This event decouples gem collision detection from gem counting,
/// allowing these systems to run in parallel and improving modularity.
#[derive(Event, Debug, Clone)]
#[allow(dead_code)] // Fields provide useful API even if not currently used
pub struct GemCollectedMessage {
    pub player_entity: Entity,
    pub gem_entity: Entity,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default, Resource)]
pub struct GemsCollected(pub i64);

pub struct GemPlugin;

impl Plugin for GemPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GemsCollected>()
            .add_observer(on_gem_collected)
            .add_systems(Update, detect_gem_player_collision);
    }
}

/// System that detects gem-player collisions and fires events
///
/// This system only handles collision detection and event firing,
/// allowing it to run independently of gem counting logic.
fn detect_gem_player_collision(
    gems: Query<(Entity, &GodotNodeHandle), With<Gem>>,
    players: Query<Entity, With<Player>>,
    collisions: Collisions,
    mut commands: Commands,
    mut godot: GodotAccess,
) {
    for (gem_entity, handle) in gems.iter() {
        for &player_entity in collisions.colliding_with(gem_entity) {
            if players.get(player_entity).is_ok()
                && let Some(mut area) = godot.try_get::<Area2D>(*handle)
            {
                // Remove the gem from the scene
                area.queue_free();

                // Fire event for gem collection
                commands.trigger(GemCollectedMessage {
                    player_entity,
                    gem_entity,
                });
            }
        }
    }
}

/// Observer that handles gem collected events and updates game state
fn on_gem_collected(
    _trigger: On<GemCollectedMessage>,
    mut gems_collected: ResMut<GemsCollected>,
    mut commands: Commands,
) {
    // Update gem count
    gems_collected.0 += 1;

    // Trigger sound effect
    commands.trigger(PlaySfxMessage::GemCollected);

    debug!("Gem collected! Total: {}", gems_collected.0);
}
