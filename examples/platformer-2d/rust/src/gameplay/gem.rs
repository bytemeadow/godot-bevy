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
#[derive(Message, Debug)]
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
            .add_message::<GemCollectedMessage>()
            .add_systems(
                Update,
                (
                    // Collision detection runs first and writes events
                    detect_gem_player_collision,
                    // State updates run after and handle events
                    handle_gem_collected_events,
                )
                    .chain(), // Ensure collision detection runs before state updates
            );
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
    mut gem_collected_events: MessageWriter<GemCollectedMessage>,
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
                gem_collected_events.write(GemCollectedMessage {
                    player_entity,
                    gem_entity,
                });
            }
        }
    }
}

/// System that handles gem collected events and updates game state
///
/// This system runs after collision detection and can run in parallel
/// with other event-handling systems that don't modify GemsCollected.
fn handle_gem_collected_events(
    mut gem_events: MessageReader<GemCollectedMessage>,
    mut gems_collected: ResMut<GemsCollected>,
    mut sfx_events: MessageWriter<PlaySfxMessage>,
) {
    for _event in gem_events.read() {
        // Update gem count
        gems_collected.0 += 1;

        // Trigger sound effect
        sfx_events.write(PlaySfxMessage::GemCollected);

        debug!("Gem collected! Total: {}", gems_collected.0);
    }
}
