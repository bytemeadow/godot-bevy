use bevy::{
    app::{App, First, Plugin, PreUpdate},
    ecs::{
        component::Component,
        entity::Entity,
        event::{Event, EventReader, EventWriter, event_update_system},
        schedule::IntoScheduleConfigs,
        system::{NonSendMut, Query},
    },
    log::trace,
};
use godot::prelude::*;
use std::sync::mpsc::Receiver;

use crate::bridge::GodotNodeHandle;

pub struct GodotCollisionsPlugin;

impl Plugin for GodotCollisionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, update_godot_collisions)
            .add_systems(
                First,
                write_godot_collision_events.before(event_update_system),
            )
            .add_event::<CollisionEvent>();
    }
}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Component, Default)]
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
pub struct CollisionEventReader(pub Receiver<CollisionEvent>);

#[doc(hidden)]
#[derive(Debug, GodotConvert)]
#[godot(via = GString)]
pub enum CollisionEventType {
    Started,
    Ended,
}

#[derive(Debug, Event)]
pub struct CollisionEvent {
    pub event_type: CollisionEventType,
    pub origin: InstanceId,
    pub target: InstanceId,
}

fn update_godot_collisions(
    mut events: EventReader<CollisionEvent>,
    mut entities: Query<(&GodotNodeHandle, &mut Collisions)>,
    all_entities: Query<(Entity, &GodotNodeHandle)>,
) {
    for (_, mut collisions) in entities.iter_mut() {
        collisions.recent_collisions = vec![];
    }

    for event in events.read() {
        trace!(target: "godot_collisions_update", event = ?event);

        let target = all_entities.iter().find_map(|(ent, reference)| {
            if reference.instance_id() == event.target {
                Some(ent)
            } else {
                None
            }
        });

        let collisions = entities.iter_mut().find_map(|(reference, collisions)| {
            if reference.instance_id() == event.origin {
                Some(collisions)
            } else {
                None
            }
        });

        let (target, mut collisions) = match (target, collisions) {
            (Some(target), Some(collisions)) => (target, collisions),
            _ => return,
        };

        match event.event_type {
            CollisionEventType::Started => {
                collisions.colliding_entities.push(target);
                collisions.recent_collisions.push(target);
            }
            CollisionEventType::Ended => collisions.colliding_entities.retain(|x| *x != target),
        };
    }
}

fn write_godot_collision_events(
    events: NonSendMut<CollisionEventReader>,
    mut event_writer: EventWriter<CollisionEvent>,
) {
    event_writer.write_batch(events.0.try_iter());
}
