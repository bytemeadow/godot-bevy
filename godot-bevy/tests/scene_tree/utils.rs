//! Common utilities for scene tree tests

use bevy::prelude::*;
use godot::prelude::*;
use godot_bevy::interop::GodotNodeHandle;
use godot_bevy_testability::*;

/// Find the Bevy entity corresponding to a Godot node
pub fn find_entity_for_node(ctx: &mut BevyGodotTestContext, node_id: InstanceId) -> Option<Entity> {
    let world = ctx.app.world_mut();
    let mut query = world.query::<(Entity, &GodotNodeHandle)>();
    for (entity, handle) in query.iter(world) {
        if handle.instance_id() == node_id {
            return Some(entity);
        }
    }
    None
}

/// Count entities with a specific component
pub fn count_entities_with<T: Component>(ctx: &mut BevyGodotTestContext) -> usize {
    let world = ctx.app.world_mut();
    let mut query = world.query::<&T>();
    query.iter(world).count()
}

/// Get the name of an entity
pub fn get_entity_name(ctx: &mut BevyGodotTestContext, entity: Entity) -> Option<String> {
    let world = ctx.app.world_mut();
    world
        .entity(entity)
        .get::<Name>()
        .map(|name| name.as_str().to_string())
}

/// Check if entity has a specific component
pub fn entity_has_component<T: Component>(ctx: &mut BevyGodotTestContext, entity: Entity) -> bool {
    let world = ctx.app.world_mut();
    world.entity(entity).contains::<T>()
}
