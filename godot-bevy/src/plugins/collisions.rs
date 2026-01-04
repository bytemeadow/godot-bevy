use crate::interop::GodotNodeHandle;
use crate::plugins::core::PrePhysicsUpdate;
use crate::plugins::scene_tree::NodeEntityIndex;
use bevy_app::{App, Plugin};
use bevy_ecs::prelude::ReflectComponent;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    message::{Message, MessageReader, MessageWriter, message_update_system},
    prelude::Resource,
    schedule::IntoScheduleConfigs,
    system::{Query, Res},
};
use bevy_reflect::Reflect;
use godot::obj::InstanceId;
use godot::prelude::*;
use std::collections::HashMap;
use std::sync::Mutex;
use std::sync::mpsc::Receiver;
use tracing::trace;

#[derive(Default)]
pub struct GodotCollisionsPlugin;

// Collision signal constants
pub const BODY_ENTERED: &str = "body_entered";
pub const BODY_EXITED: &str = "body_exited";
pub const AREA_ENTERED: &str = "area_entered";
pub const AREA_EXITED: &str = "area_exited";

/// All collision signals that indicate collision start
pub const COLLISION_START_SIGNALS: &[&str] = &[BODY_ENTERED, AREA_ENTERED];

/// Resource for receiving collision messages from Godot.
/// Wrapped in Mutex to be Send+Sync, allowing it to be a regular Bevy Resource.
#[derive(Resource)]
pub struct CollisionMessageReader(pub Mutex<Receiver<CollisionMessage>>);

impl CollisionMessageReader {
    pub fn new(receiver: Receiver<CollisionMessage>) -> Self {
        Self(Mutex::new(receiver))
    }
}

#[derive(Debug, Message)]
pub struct CollisionMessage {
    pub event_type: CollisionMessageType,
    pub origin: GodotNodeHandle,
    pub target: GodotNodeHandle,
}

impl Plugin for GodotCollisionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            PrePhysicsUpdate,
            (
                write_godot_collision_events.before(message_update_system),
                update_godot_collisions,
            ),
        )
        .add_message::<CollisionMessage>();
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Component, Default, Reflect)]
#[reflect(Component)]
pub struct Collisions {
    colliding_entities: Vec<Entity>,
    recent_collisions: Vec<Entity>,
}

impl Collisions {
    pub fn colliding(&self) -> &[Entity] {
        &self.colliding_entities
    }

    pub fn recent_collisions(&self) -> &[Entity] {
        &self.recent_collisions
    }
}

#[doc(hidden)]
#[derive(Debug, GodotConvert)]
#[godot(via = GString)]
pub enum CollisionMessageType {
    Started,
    Ended,
}

fn update_godot_collisions(
    mut messages: MessageReader<CollisionMessage>,
    mut entities: Query<(Entity, &GodotNodeHandle, &mut Collisions)>,
    node_index: Res<NodeEntityIndex>,
) {
    // Build collision entity map (only entities with Collisions component)
    let collisions_by_instance: HashMap<InstanceId, Entity> = entities
        .iter()
        .map(|(entity, handle, _)| (handle.instance_id(), entity))
        .collect();

    for (_, _, mut collisions) in entities.iter_mut() {
        collisions.recent_collisions = vec![];
    }

    for event in messages.read() {
        trace!(target: "godot_collisions_update", event = ?event);

        // Use NodeEntityIndex for O(1) target lookup
        let target = node_index.get(event.target.instance_id());
        let origin_entity = collisions_by_instance
            .get(&event.origin.instance_id())
            .copied();

        let (target, origin_entity) = match (target, origin_entity) {
            (Some(target), Some(origin)) => (target, origin),
            _ => continue,
        };

        let Ok((_, _, mut collisions)) = entities.get_mut(origin_entity) else {
            continue;
        };

        match event.event_type {
            CollisionMessageType::Started => {
                collisions.colliding_entities.push(target);
                collisions.recent_collisions.push(target);
            }
            CollisionMessageType::Ended => collisions.colliding_entities.retain(|x| *x != target),
        };
    }
}

fn write_godot_collision_events(
    events: Res<CollisionMessageReader>,
    mut message_writer: MessageWriter<CollisionMessage>,
) {
    let receiver = events.0.lock().unwrap_or_else(|e| e.into_inner());
    message_writer.write_batch(receiver.try_iter());
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::world::World;

    /// Helper to create a mock InstanceId for testing
    fn mock_instance_id(id: i64) -> InstanceId {
        InstanceId::from_i64(id)
    }

    /// Helper to create a mock GodotNodeHandle for testing
    fn mock_handle(id: i64) -> GodotNodeHandle {
        GodotNodeHandle::from_instance_id(mock_instance_id(id))
    }

    #[test]
    fn test_hashmap_lookup_correctness() {
        // Test that HashMap-based lookup produces correct entity mappings
        let mut world = World::new();

        // Spawn entities with mock handles
        let entity1 = world.spawn(mock_handle(100)).id();
        let entity2 = world.spawn(mock_handle(200)).id();
        let entity3 = world.spawn(mock_handle(300)).id();

        // Build the HashMap (simulating what update_godot_collisions does)
        let instance_to_entity: HashMap<InstanceId, Entity> = world
            .query::<(Entity, &GodotNodeHandle)>()
            .iter(&world)
            .map(|(entity, handle)| (handle.instance_id(), entity))
            .collect();

        // Verify lookups
        assert_eq!(
            instance_to_entity.get(&mock_instance_id(100)),
            Some(&entity1)
        );
        assert_eq!(
            instance_to_entity.get(&mock_instance_id(200)),
            Some(&entity2)
        );
        assert_eq!(
            instance_to_entity.get(&mock_instance_id(300)),
            Some(&entity3)
        );
        assert_eq!(instance_to_entity.get(&mock_instance_id(999)), None);
    }

    #[test]
    fn test_collisions_component_started() {
        // Test that collision started events add to colliding_entities
        let mut collisions = Collisions::default();
        let target_entity = Entity::from_bits(42);

        // Simulate Started event
        collisions.colliding_entities.push(target_entity);
        collisions.recent_collisions.push(target_entity);

        assert_eq!(collisions.colliding().len(), 1);
        assert_eq!(collisions.colliding()[0], target_entity);
        assert_eq!(collisions.recent_collisions().len(), 1);
    }

    #[test]
    fn test_collisions_component_ended() {
        // Test that collision ended events remove from colliding_entities
        let mut collisions = Collisions::default();
        let target1 = Entity::from_bits(42);
        let target2 = Entity::from_bits(43);

        // Add two collisions
        collisions.colliding_entities.push(target1);
        collisions.colliding_entities.push(target2);

        // Remove first one (simulate Ended event)
        collisions.colliding_entities.retain(|x| *x != target1);

        assert_eq!(collisions.colliding().len(), 1);
        assert_eq!(collisions.colliding()[0], target2);
    }

    #[test]
    fn test_multiple_entities_with_collisions() {
        // Test that HashMap correctly maps multiple entities with Collisions component
        let mut world = World::new();

        // Spawn entities - some with Collisions, some without
        let origin1 = world.spawn((mock_handle(100), Collisions::default())).id();
        let origin2 = world.spawn((mock_handle(200), Collisions::default())).id();
        let target1 = world.spawn(mock_handle(300)).id();
        let target2 = world.spawn(mock_handle(400)).id();

        // Build collision entity map (only entities with Collisions component)
        let collisions_by_instance: HashMap<InstanceId, Entity> = world
            .query::<(Entity, &GodotNodeHandle, &Collisions)>()
            .iter(&world)
            .map(|(entity, handle, _)| (handle.instance_id(), entity))
            .collect();

        // Build all entities map
        let instance_to_entity: HashMap<InstanceId, Entity> = world
            .query::<(Entity, &GodotNodeHandle)>()
            .iter(&world)
            .map(|(entity, handle)| (handle.instance_id(), entity))
            .collect();

        // Verify collision entities map
        assert_eq!(collisions_by_instance.len(), 2);
        assert_eq!(
            collisions_by_instance.get(&mock_instance_id(100)),
            Some(&origin1)
        );
        assert_eq!(
            collisions_by_instance.get(&mock_instance_id(200)),
            Some(&origin2)
        );
        assert_eq!(collisions_by_instance.get(&mock_instance_id(300)), None);

        // Verify all entities map includes targets
        assert_eq!(instance_to_entity.len(), 4);
        assert_eq!(
            instance_to_entity.get(&mock_instance_id(300)),
            Some(&target1)
        );
        assert_eq!(
            instance_to_entity.get(&mock_instance_id(400)),
            Some(&target2)
        );
    }

    #[test]
    fn test_recent_collisions_cleared_each_frame() {
        // Test that recent_collisions is cleared properly
        let mut collisions = Collisions::default();
        let target = Entity::from_bits(42);

        // Frame 1: collision starts
        collisions.colliding_entities.push(target);
        collisions.recent_collisions.push(target);
        assert_eq!(collisions.recent_collisions().len(), 1);

        // Frame 2: clear recent (simulating start of update_godot_collisions)
        collisions.recent_collisions = vec![];
        assert_eq!(collisions.recent_collisions().len(), 0);
        // But colliding_entities should persist
        assert_eq!(collisions.colliding().len(), 1);
    }
}
