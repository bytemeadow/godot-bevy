use bevy::{
    app::{App, Plugin, PreUpdate},
    ecs::{component::Component, entity::Entity, event::EventReader, system::Query},
    log::trace,
};
use godot::prelude::*;

use super::GodotSignal;
use crate::bridge::GodotNodeHandle;

pub struct GodotCollisionsPlugin;

// Collision signal constants
pub const BODY_ENTERED: &str = "body_entered";
pub const BODY_EXITED: &str = "body_exited";
pub const AREA_ENTERED: &str = "area_entered";
pub const AREA_EXITED: &str = "area_exited";

/// All collision signals that indicate collision start
pub const COLLISION_START_SIGNALS: &[&str] = &[BODY_ENTERED, AREA_ENTERED];

/// All collision signals that indicate collision end
pub const COLLISION_END_SIGNALS: &[&str] = &[BODY_EXITED, AREA_EXITED];

/// All collision signals (both start and end)
pub const ALL_COLLISION_SIGNALS: &[&str] = &[BODY_ENTERED, BODY_EXITED, AREA_ENTERED, AREA_EXITED];

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
        let signal_name = signal.name.as_str();
        let event_type = if COLLISION_START_SIGNALS.contains(&signal_name) {
            CollisionEventType::Started
        } else if COLLISION_END_SIGNALS.contains(&signal_name) {
            CollisionEventType::Ended
        } else {
            continue; // Skip non-collision signals
        };

        // The colliding body/area is passed as the first argument to collision signals
        let target_node_handle = match signal.arguments.first() {
            Some(arg) => match &arg.instance_id {
                Some(instance_id) => GodotNodeHandle::from_instance_id(*instance_id),
                None => continue, // Skip if first argument is not an object with instance ID
            },
            None => continue, // Skip if no arguments
        };

        trace!(target: "godot_collisions_update", signal = ?signal, event_type = ?event_type);

        let target_entity = all_entities.iter().find_map(|(ent, reference)| {
            if *reference == target_node_handle {
                Some(ent)
            } else {
                None
            }
        });

        let collisions = entities.iter_mut().find_map(|(reference, collisions)| {
            if reference == &signal.origin {
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
            CollisionEventType::Ended => collisions
                .colliding_entities
                .retain(|x| *x != target_entity),
        };
    }
}
