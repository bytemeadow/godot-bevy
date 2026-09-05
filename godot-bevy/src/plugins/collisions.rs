//! Collision detection for godot-bevy.
//!
//! This module bridges Godot's collision detection with Bevy's ECS patterns,
//! providing multiple ways to detect and respond to collisions.
//!
//! # Accessing Collisions
//!
//! godot-bevy provides a [`Collisions`] system parameter for querying collision state.
//! This is the primary way to check what entities are currently colliding.
//!
//! ```no_run
//! # use godot_bevy::bevy_ecs::prelude::Entity;
//! # use godot_bevy::prelude::Collisions;
//! fn my_system(collisions: Collisions, player: Entity, enemy: Entity) {
//!     for (entity_a, entity_b) in collisions.iter() {
//!     }
//!
//!     if collisions.contains(player, enemy) {
//!     }
//!
//!     for other in collisions.colliding_with(player) {
//!     }
//! }
//! ```
//!
//! # Collision Events
//!
//! For reacting to collision start/end events, use [`CollisionStarted`] and
//! [`CollisionEnded`]. These can be read as Messages or observed as Events.
//!
//! ## Reading as Messages
//!
//! ```no_run
//! # use godot_bevy::bevy_ecs::prelude::MessageReader;
//! # use godot_bevy::prelude::CollisionStarted;
//! fn handle_hits(mut started: MessageReader<CollisionStarted>) {
//!     for event in started.read() {
//!         println!(
//!             "{:?} started colliding with {:?}",
//!             event.entity1, event.entity2
//!         );
//!     }
//! }
//! ```
//!
//! ## Using Observers
//!
//! ```no_run
//! # use godot_bevy::bevy_app::App;
//! # use godot_bevy::bevy_ecs::prelude::On;
//! # use godot_bevy::prelude::CollisionStarted;
//! # fn add_collision_observer(app: &mut App) {
//! app.add_observer(|event: On<CollisionStarted>| {
//!     println!(
//!         "{:?} started colliding with {:?}",
//!         event.event().entity1,
//!         event.event().entity2
//!     );
//! });
//! # }
//! ```

use crate::interop::GodotNodeHandle;
use crate::plugins::scene_tree::NodeEntityIndex;
use bevy_app::{App, FixedFirst, Plugin};
use bevy_ecs::{
    entity::Entity,
    event::{EntityEvent, Event},
    lifecycle::Remove,
    message::{Message, MessageReader, MessageWriter},
    observer::On,
    prelude::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Res, ResMut, SystemParam},
};
use crossbeam_channel::Receiver;
use godot::prelude::*;
use parking_lot::Mutex;
// foldhash, not SipHash: small Entity-tuple keys don't need DoS resistance,
// and SipHash's cost shows up in the collision burst benchmark.
use bevy_platform::collections::{HashMap, HashSet};
use tracing::trace;

pub const BODY_ENTERED: &str = "body_entered";
pub const BODY_EXITED: &str = "body_exited";
pub const AREA_ENTERED: &str = "area_entered";
pub const AREA_EXITED: &str = "area_exited";

/// All collision signals that indicate collision start
pub const COLLISION_START_SIGNALS: &[&str] = &[BODY_ENTERED, AREA_ENTERED];

/// Event-count cutoff for batch neighbor rebuild mode.
/// Batches at or above this size rebuild adjacency from `active_pairs`.
const COLLISION_NEIGHBOR_REBUILD_THRESHOLD: usize = 512;

/// Event fired when two entities start colliding.
///
/// Can be read as a [`Message`] with [`MessageReader`] or observed with
/// Bevy's observer system.
///
/// # Example
///
/// ```no_run
/// # use godot_bevy::bevy_app::App;
/// # use godot_bevy::bevy_ecs::prelude::{MessageReader, On};
/// # use godot_bevy::prelude::CollisionStarted;
/// fn handle_collision_start(mut events: MessageReader<CollisionStarted>) {
///     for event in events.read() {
///         println!("{:?} hit {:?}", event.entity1, event.entity2);
///     }
/// }
///
/// # fn add_collision_observer(app: &mut App) {
/// app.add_observer(|event: On<CollisionStarted>| {
///     let event = event.event();
///     println!("{:?} hit {:?}", event.entity1, event.entity2);
/// });
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Message, Event)]
pub struct CollisionStarted {
    /// The first entity in the collision.
    pub entity1: Entity,
    /// The second entity in the collision.
    pub entity2: Entity,
}

/// Event fired when two entities stop colliding.
///
/// Can be read as a [`Message`] with [`MessageReader`] or observed with
/// Bevy's observer system.
///
/// # Example
///
/// ```no_run
/// # use godot_bevy::bevy_app::App;
/// # use godot_bevy::bevy_ecs::prelude::{MessageReader, On};
/// # use godot_bevy::prelude::CollisionEnded;
/// fn handle_collision_end(mut events: MessageReader<CollisionEnded>) {
///     for event in events.read() {
///         println!("{:?} separated from {:?}", event.entity1, event.entity2);
///     }
/// }
///
/// # fn add_collision_observer(app: &mut App) {
/// app.add_observer(|event: On<CollisionEnded>| {
///     let event = event.event();
///     println!("{:?} separated from {:?}", event.entity1, event.entity2);
/// });
/// # }
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Message, Event)]
pub struct CollisionEnded {
    /// The first entity in the collision.
    pub entity1: Entity,
    /// The second entity in the collision.
    pub entity2: Entity,
}

/// Resource that tracks all current collision pairs.
///
/// This is automatically updated each frame based on Godot's collision events.
/// Use the [`Collisions`] system parameter for convenient access.
#[derive(Resource, Default, Debug)]
pub struct CollisionState {
    active_pairs: HashSet<(Entity, Entity)>,

    started_this_frame: Vec<(Entity, Entity)>,

    ended_this_frame: Vec<(Entity, Entity)>,

    entity_collisions: HashMap<Entity, Vec<Entity>>,
}

impl CollisionState {
    fn begin_frame(&mut self) {
        self.started_this_frame.clear();
        self.ended_this_frame.clear();
    }

    fn add_collision(&mut self, origin: Entity, target: Entity) -> bool {
        self.add_collision_internal(origin, target, true)
    }

    fn add_collision_without_neighbors(&mut self, origin: Entity, target: Entity) -> bool {
        self.add_collision_internal(origin, target, false)
    }

    fn add_collision_internal(
        &mut self,
        origin: Entity,
        target: Entity,
        update_neighbors: bool,
    ) -> bool {
        let pair = normalize_pair(origin, target);

        if self.active_pairs.insert(pair) {
            self.started_this_frame.push(pair);

            if update_neighbors {
                self.entity_collisions
                    .entry(origin)
                    .or_default()
                    .push(target);
                self.entity_collisions
                    .entry(target)
                    .or_default()
                    .push(origin);
            }
            true
        } else {
            false
        }
    }

    fn remove_collision(&mut self, origin: Entity, target: Entity) -> bool {
        self.remove_collision_internal(origin, target, true)
    }

    fn remove_collision_without_neighbors(&mut self, origin: Entity, target: Entity) -> bool {
        self.remove_collision_internal(origin, target, false)
    }

    fn remove_collision_internal(
        &mut self,
        origin: Entity,
        target: Entity,
        update_neighbors: bool,
    ) -> bool {
        let pair = normalize_pair(origin, target);

        if self.active_pairs.remove(&pair) {
            self.ended_this_frame.push(pair);

            if update_neighbors {
                if let Some(collisions) = self.entity_collisions.get_mut(&origin) {
                    collisions.retain(|&e| e != target);
                }
                if let Some(collisions) = self.entity_collisions.get_mut(&target) {
                    collisions.retain(|&e| e != origin);
                }
            }
            true
        } else {
            false
        }
    }

    /// Remove every pair involving `entity`, returning the entities it was
    /// colliding with (for `CollisionEnded` emission). Symmetric: also drops
    /// `entity` from each neighbor's adjacency list.
    fn purge_entity(&mut self, entity: Entity) -> Vec<Entity> {
        let neighbors = self.entity_collisions.remove(&entity).unwrap_or_default();
        for &other in &neighbors {
            self.active_pairs.remove(&normalize_pair(entity, other));
            if let Some(list) = self.entity_collisions.get_mut(&other) {
                list.retain(|&e| e != entity);
            }
        }
        neighbors
    }

    fn rebuild_entity_collisions(&mut self) {
        self.entity_collisions.clear();
        self.entity_collisions.reserve(self.active_pairs.len() * 2);
        for &(entity1, entity2) in &self.active_pairs {
            self.entity_collisions
                .entry(entity1)
                .or_default()
                .push(entity2);
            self.entity_collisions
                .entry(entity2)
                .or_default()
                .push(entity1);
        }
    }

    /// Check if two entities are currently colliding
    pub fn contains(&self, a: Entity, b: Entity) -> bool {
        self.active_pairs.contains(&normalize_pair(a, b))
    }

    /// Get all entities currently colliding with the given entity
    pub fn colliding_with(&self, entity: Entity) -> &[Entity] {
        self.entity_collisions
            .get(&entity)
            .map(|v| v.as_slice())
            .unwrap_or(&[])
    }

    /// Iterate over all currently active collision pairs
    pub fn iter(&self) -> impl Iterator<Item = (Entity, Entity)> + '_ {
        self.active_pairs.iter().copied()
    }

    /// Iterate over collision pairs that started this frame
    pub fn started(&self) -> impl Iterator<Item = (Entity, Entity)> + '_ {
        self.started_this_frame.iter().copied()
    }

    /// Iterate over collision pairs that ended this frame
    pub fn ended(&self) -> impl Iterator<Item = (Entity, Entity)> + '_ {
        self.ended_this_frame.iter().copied()
    }

    /// Returns true if there are no active collisions
    pub fn is_empty(&self) -> bool {
        self.active_pairs.is_empty()
    }

    /// Returns the number of active collision pairs
    pub fn len(&self) -> usize {
        self.active_pairs.len()
    }
}

#[inline]
fn normalize_pair(a: Entity, b: Entity) -> (Entity, Entity) {
    if a < b { (a, b) } else { (b, a) }
}

/// System parameter for querying collision state.
///
/// This provides a convenient API for checking collisions in systems.
///
/// # Example
///
/// ```no_run
/// # use godot_bevy::bevy_ecs::prelude::Entity;
/// # use godot_bevy::prelude::Collisions;
/// fn my_system(collisions: Collisions, player: Entity, enemy: Entity) {
///     for (a, b) in collisions.iter() {
///         println!("{a:?} is colliding with {b:?}");
///     }
///
///     if collisions.contains(player, enemy) {
///     }
///
///     for &other in collisions.colliding_with(player) {
///     }
/// }
/// ```
#[derive(SystemParam)]
pub struct Collisions<'w> {
    state: Res<'w, CollisionState>,
}

impl Collisions<'_> {
    /// Check if two entities are currently colliding.
    #[inline]
    pub fn contains(&self, a: Entity, b: Entity) -> bool {
        self.state.contains(a, b)
    }

    /// Get all entities currently colliding with the given entity.
    #[inline]
    pub fn colliding_with(&self, entity: Entity) -> &[Entity] {
        self.state.colliding_with(entity)
    }

    /// Iterate over all currently active collision pairs.
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = (Entity, Entity)> + '_ {
        self.state.iter()
    }

    /// Returns true if there are no active collisions.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.state.is_empty()
    }

    /// Returns the number of active collision pairs.
    #[inline]
    pub fn len(&self) -> usize {
        self.state.len()
    }
}

/// Internal message type for receiving collision events from Godot.
/// This is not part of the public API - use CollisionStarted/CollisionEnded instead.
#[doc(hidden)]
#[derive(Debug)]
pub struct RawCollisionMessage {
    pub event_type: CollisionMessageType,
    pub origin: GodotNodeHandle,
    pub target: GodotNodeHandle,
}

/// Resource for receiving collision messages from Godot.
/// Wrapped in Mutex to be Send+Sync, allowing it to be a regular Bevy Resource.
#[derive(Resource)]
pub struct CollisionMessageReader(pub Mutex<Receiver<RawCollisionMessage>>);

impl CollisionMessageReader {
    pub fn new(receiver: Receiver<RawCollisionMessage>) -> Self {
        Self(Mutex::new(receiver))
    }
}

#[doc(hidden)]
#[derive(Debug, GodotConvert)]
#[godot(via = GString)]
pub enum CollisionMessageType {
    Started,
    Ended,
}

/// Plugin that enables collision detection between Godot physics bodies and Bevy entities.
///
/// This plugin automatically tracks collisions for entities that have collision
/// signals (Area2D, Area3D, RigidBody2D, RigidBody3D, etc.).
///
/// # Usage
///
/// Add the plugin to your app:
///
/// ```no_run
/// # use godot_bevy::bevy_app::App;
/// # use godot_bevy::prelude::GodotCollisionsPlugin;
/// # let mut app = App::new();
/// app.add_plugins(GodotCollisionsPlugin);
/// ```
///
/// Then use the [`Collisions`] system parameter or collision events:
///
/// ```no_run
/// # use godot_bevy::bevy_ecs::prelude::{Entity, MessageReader};
/// # use godot_bevy::prelude::{CollisionStarted, Collisions};
/// fn detect_hits(
///     collisions: Collisions,
///     mut started: MessageReader<CollisionStarted>,
///     player: Entity,
///     enemy: Entity,
/// ) {
///     if collisions.contains(player, enemy) {
///     }
///
///     for event in started.read() {
///     }
/// }
/// ```
#[derive(Default)]
pub struct GodotCollisionsPlugin;

impl Plugin for GodotCollisionsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<CollisionState>()
            .add_message::<CollisionStarted>()
            .add_message::<CollisionEnded>()
            .add_observer(purge_collisions_on_node_removed)
            .add_systems(
                FixedFirst,
                (
                    process_godot_collisions,
                    trigger_collision_observers.after(process_godot_collisions),
                ),
            );
    }
}

fn process_godot_collisions(
    events: Option<Res<CollisionMessageReader>>,
    mut collision_state: ResMut<CollisionState>,
    mut started_writer: MessageWriter<CollisionStarted>,
    mut ended_writer: MessageWriter<CollisionEnded>,
    node_index: Res<NodeEntityIndex>,
) {
    collision_state.begin_frame();

    let Some(events) = events else {
        return;
    };

    let receiver = events.0.lock();
    let use_rebuild_path = receiver.len() >= COLLISION_NEIGHBOR_REBUILD_THRESHOLD;

    for event in receiver.try_iter() {
        trace!(target: "godot_collisions", event = ?event);

        let origin_entity = node_index.get(event.origin.instance_id());
        let target_entity = node_index.get(event.target.instance_id());

        let (origin, target) = match (origin_entity, target_entity) {
            (Some(o), Some(t)) => (o, t),
            _ => continue,
        };

        match event.event_type {
            CollisionMessageType::Started => {
                let changed = if use_rebuild_path {
                    collision_state.add_collision_without_neighbors(origin, target)
                } else {
                    collision_state.add_collision(origin, target)
                };
                if changed {
                    started_writer.write(CollisionStarted {
                        entity1: origin,
                        entity2: target,
                    });
                }
            }
            CollisionMessageType::Ended => {
                let changed = if use_rebuild_path {
                    collision_state.remove_collision_without_neighbors(origin, target)
                } else {
                    collision_state.remove_collision(origin, target)
                };
                if changed {
                    ended_writer.write(CollisionEnded {
                        entity1: origin,
                        entity2: target,
                    });
                }
            }
        }
    }

    if use_rebuild_path {
        collision_state.rebuild_entity_collisions();
    }
}

fn trigger_collision_observers(
    mut commands: Commands,
    mut started_reader: MessageReader<CollisionStarted>,
    mut ended_reader: MessageReader<CollisionEnded>,
) {
    for &event in started_reader.read() {
        commands.trigger(event);
    }
    for &event in ended_reader.read() {
        commands.trigger(event);
    }
}

/// Purge collision pairs when a node's `GodotNodeHandle` is removed. `Remove` fires
/// on despawn and on the protected-node strip path but not on re-association (that is
/// a `Discard`), so a freed collider's pairs are cleared without a spurious end for a
/// reparent. Emitted through the message tier so `trigger_collision_observers` re-pumps
/// it to observers, matching `process_godot_collisions`.
fn purge_collisions_on_node_removed(
    trigger: On<Remove, GodotNodeHandle>,
    mut state: ResMut<CollisionState>,
    mut ended: MessageWriter<CollisionEnded>,
) {
    let entity = trigger.event_target();
    for other in state.purge_entity(entity) {
        ended.write(CollisionEnded {
            entity1: entity,
            entity2: other,
        });
    }
}

#[cfg(test)]
include!("collisions_tests.rs");
