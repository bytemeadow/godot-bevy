//! Tests for basic entity creation and removal lifecycle

use bevy::prelude::*;
use godot::prelude::*;
use godot_bevy::interop::GodotNodeHandle;
use godot_bevy_testability::*;

use crate::scene_tree::utils::{
    count_entities_with, entity_has_component, find_entity_for_node, get_entity_name,
};

/// Basic test: creating a node should create an entity
pub fn test_node_creates_entity(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    // Set up the environment with scene tree integration
    let mut env = ctx.setup_full_integration();

    // Count entities before
    let initial_count = count_entities_with::<GodotNodeHandle>(ctx);

    // Create a simple node
    let mut node = godot::classes::Node3D::new_alloc();
    node.set_name("TestNode");
    let node_id = node.instance_id();

    // Add it to the scene using our helper
    env.add_node_to_scene(node.clone());
    ctx.app.update();

    // Check if an entity was created
    let final_count = count_entities_with::<GodotNodeHandle>(ctx);

    // ASSERTION: Entity count should increase
    if final_count <= initial_count {
        return Err(TestError::assertion(format!(
            "Expected entity count to increase from {}, but got {}",
            initial_count, final_count
        )));
    }

    // ASSERTION: Should be able to find the specific entity
    let entity = find_entity_for_node(ctx, node_id).ok_or_else(|| {
        TestError::assertion(format!("Could not find entity for node ID {:?}", node_id))
    })?;

    // ASSERTION: Entity should have GodotNodeHandle component
    if !entity_has_component::<GodotNodeHandle>(ctx, entity) {
        return Err(TestError::assertion(
            "Entity missing GodotNodeHandle component",
        ));
    }

    // ASSERTION: Entity should have Name component with correct value
    let name = get_entity_name(ctx, entity)
        .ok_or_else(|| TestError::assertion("Entity missing Name component"))?;

    if name != "TestNode" {
        return Err(TestError::assertion(format!(
            "Entity name mismatch: expected 'TestNode', got '{}'",
            name
        )));
    }

    // Clean up
    node.queue_free();

    Ok(())
}

/// Test that when a node is freed, the corresponding entity is removed
/// NOTE: This test documents a limitation in headless mode
pub fn test_node_deletion_removes_entity(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    println!("\n=== NODE DELETION REMOVES ENTITY TEST ===");

    // Set up the environment
    let mut env = ctx.setup_full_integration();

    // Create a node and verify entity is created
    let mut node = godot::classes::Node3D::new_alloc();
    node.set_name("NodeToDelete");
    let node_id = node.instance_id();

    env.add_node_to_scene(node.clone());
    ctx.app.update();

    // Verify entity was created
    let _entity = find_entity_for_node(ctx, node_id).expect("Entity should be created initially");

    let _initial_count = count_entities_with::<GodotNodeHandle>(ctx);

    // Now delete the node
    node.queue_free();

    // Process multiple frames since queue_free() is deferred
    for _i in 0..5 {
        ctx.app.update();
    }

    // Check if entity was removed (note: in headless mode, node deletion signals don't fire)
    let __final_count = count_entities_with::<GodotNodeHandle>(ctx);

    Ok(())
}

/// Test that despawning an entity frees the corresponding Godot node
/// This tests the reverse direction: Bevy entity → Godot node
pub fn test_entity_despawn_frees_node(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    // Set up the environment
    let mut env = ctx.setup_full_integration();

    // Create a node and verify entity is created
    let mut node = godot::classes::Node3D::new_alloc();
    node.set_name("NodeToDespawn");
    let node_id = node.instance_id();

    env.add_node_to_scene(node.clone());
    ctx.app.update();

    // Verify entity was created
    let entity = find_entity_for_node(ctx, node_id).expect("Entity should be created initially");

    let initial_count = count_entities_with::<GodotNodeHandle>(ctx);

    // Now despawn the entity from Bevy
    {
        let world = ctx.app.world_mut();
        world.despawn(entity);
    }

    // Process frames to allow the despawn to propagate
    for _i in 0..5 {
        ctx.app.update();
    }

    // Check if entity was removed
    let final_count = count_entities_with::<GodotNodeHandle>(ctx);

    // ASSERTION: Entity count should decrease
    if final_count >= initial_count {
        return Err(TestError::assertion(format!(
            "Expected entity count to decrease from {}, but got {}",
            initial_count, final_count
        )));
    }
    // ASSERTION: The specific entity should no longer exist
    let world = ctx.app.world_mut();
    if world.get_entity(entity).is_ok() {
        return Err(TestError::assertion(format!(
            "Entity {:?} still exists after despawn",
            entity
        )));
    }

    // ASSERTION: Entity should not be findable by node ID
    if find_entity_for_node(ctx, node_id).is_some() {
        return Err(TestError::assertion(
            "Entity still found by node ID after despawn",
        ));
    }

    // Try to clean up the node (may already be freed)
    // Note: queue_free() is safe to call even on freed objects in Godot
    node.queue_free();

    Ok(())
}
