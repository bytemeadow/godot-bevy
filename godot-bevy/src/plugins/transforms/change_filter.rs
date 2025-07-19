use bevy::ecs::component::Tick;
use bevy::prelude::{Entity, Resource};
use std::collections::HashMap;

/// Resource that tracks entities synced from Godot and their sync ticks
/// This prevents post_update systems from writing back transforms that
/// we just synced from Godot, and enables tick-based change detection
#[derive(Resource, Default)]
pub struct GodotSyncedEntities {
    pub synced_entities: HashMap<Entity, Tick>,
}
