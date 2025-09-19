//! Common utilities for transform tests

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

/// Assert that two Vec3 values are approximately equal
pub fn assert_vec3_eq(actual: Vec3, expected: Vec3, message: &str) {
    const TOLERANCE: f32 = 0.01;
    let diff = actual - expected;
    assert!(
        diff.length() < TOLERANCE,
        "{}: expected {:?}, got {:?} (diff: {:.3})",
        message,
        expected,
        actual,
        diff.length()
    );
}

/// Assert that two Vec3 values are approximately equal with custom tolerance
pub fn assert_vec3_eq_with_tolerance(actual: Vec3, expected: Vec3, tolerance: f32, message: &str) {
    let diff = actual - expected;
    let distance = diff.length();
    assert!(
        distance < tolerance,
        "{}: expected {:?}, got {:?} (difference: {})",
        message,
        expected,
        actual,
        distance
    );
}
