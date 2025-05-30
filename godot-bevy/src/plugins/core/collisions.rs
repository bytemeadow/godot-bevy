use bevy::{
    app::{App, Plugin, PreUpdate},
    ecs::{
        component::Component,
        entity::Entity,
        event::EventReader,
        system::{Query},
    },
    log::trace,
};
use godot::prelude::*;

use crate::bridge::GodotNodeHandle;
use super::GodotSignal;

pub struct GodotCollisionsPlugin;

impl Plugin for GodotCollisionsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreUpdate, update_godot_collisions);
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
#[derive(Debug, GodotConvert)]
#[godot(via = GString)]
pub enum CollisionEventType {
    Started,
    Ended,
}

fn update_godot_collisions(
    mut signal_events: EventReader<GodotSignal>,
    mut entities: Query<(&GodotNodeHandle, &mut Collisions)>,
    all_entities: Query<(Entity, &GodotNodeHandle)>,
) {
    // Clear recent collisions for all entities
    for (_, mut collisions) in entities.iter_mut() {
        collisions.recent_collisions = vec![];
    }

    // Process collision signals
    for signal in signal_events.read() {
        let (event_type, origin, target) = match signal.name.as_str() {
            "body_entered" => (CollisionEventType::Started, &signal.origin, &signal.target),
            "body_exited" => (CollisionEventType::Ended, &signal.origin, &signal.target),
            _ => continue, // Skip non-collision signals
        };

        trace!(target: "godot_collisions_update", signal = ?signal, event_type = ?event_type);

        let target_entity = all_entities.iter().find_map(|(ent, reference)| {
            if reference == target {
                Some(ent)
            } else {
                None
            }
        });

        let collisions = entities.iter_mut().find_map(|(reference, collisions)| {
            if reference == origin {
                Some(collisions)
            } else {
                None
            }
        });

        let (target_entity, mut collisions) = match (target_entity, collisions) {
            (Some(target), Some(collisions)) => (target, collisions),
            _ => continue,
        };

        match event_type {
            CollisionEventType::Started => {
                collisions.colliding_entities.push(target_entity);
                collisions.recent_collisions.push(target_entity);
            }
            CollisionEventType::Ended => collisions.colliding_entities.retain(|x| *x != target_entity),
        };
    }
}
