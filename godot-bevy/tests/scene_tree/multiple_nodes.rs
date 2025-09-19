//! Tests for multiple node creation and different node types

use bevy::prelude::*;
use godot::prelude::*;
use godot_bevy::interop::GodotNodeHandle;
use godot_bevy_testability::*;

use crate::scene_tree::utils::{
    count_entities_with, entity_has_component, find_entity_for_node, get_entity_name,
};

/// Test that multiple nodes create multiple entities with unique components
pub fn test_multiple_nodes_create_multiple_entities(
    ctx: &mut BevyGodotTestContext,
) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    // Set up the environment
    let mut env = ctx.setup_full_integration();

    let initial_count = count_entities_with::<GodotNodeHandle>(ctx);

    // Create multiple different types of nodes
    let mut node1 = godot::classes::Node3D::new_alloc();
    node1.set_name("Node3D_1");
    let node1_id = node1.instance_id();

    let mut node2 = godot::classes::Node3D::new_alloc();
    node2.set_name("Node3D_2");
    let node2_id = node2.instance_id();

    let mut node3 = godot::classes::Node2D::new_alloc();
    node3.set_name("Node2D_1");
    let node3_id = node3.instance_id();

    // Add them to the scene
    env.add_node_to_scene(node1.clone());
    env.add_node_to_scene(node2.clone());
    env.add_node_to_scene(node3.clone().upcast::<godot::classes::Node>());

    ctx.app.update();

    let final_count = count_entities_with::<GodotNodeHandle>(ctx);

    // ASSERTION: Entity count should increase by at least 3
    if final_count < initial_count + 3 {
        return Err(TestError::assertion(format!(
            "Entity count did not increase sufficiently (expected +3, got +{})",
            final_count - initial_count
        )));
    }

    // ASSERTION: Should be able to find each specific entity
    let entity1 = find_entity_for_node(ctx, node1_id)
        .ok_or_else(|| TestError::assertion("Could not find entity for node1"))?;
    let entity2 = find_entity_for_node(ctx, node2_id)
        .ok_or_else(|| TestError::assertion("Could not find entity for node2"))?;
    let entity3 = find_entity_for_node(ctx, node3_id)
        .ok_or_else(|| TestError::assertion("Could not find entity for node3"))?;

    // ASSERTION: All entities should be unique
    if entity1 == entity2 || entity2 == entity3 || entity1 == entity3 {
        return Err(TestError::assertion(format!(
            "Some entities are not unique: {:?} {:?} {:?}",
            entity1, entity2, entity3
        )));
    }

    // ASSERTION: Each entity should have the correct name
    let name1 = get_entity_name(ctx, entity1)
        .ok_or_else(|| TestError::assertion("Entity1 missing Name component"))?;
    if name1 != "Node3D_1" {
        return Err(TestError::assertion(format!(
            "Entity1 name mismatch: expected 'Node3D_1', got '{}'",
            name1
        )));
    }

    let name2 = get_entity_name(ctx, entity2)
        .ok_or_else(|| TestError::assertion("Entity2 missing Name component"))?;
    if name2 != "Node3D_2" {
        return Err(TestError::assertion(format!(
            "Entity2 name mismatch: expected 'Node3D_2', got '{}'",
            name2
        )));
    }

    let name3 = get_entity_name(ctx, entity3)
        .ok_or_else(|| TestError::assertion("Entity3 missing Name component"))?;
    if name3 != "Node2D_1" {
        return Err(TestError::assertion(format!(
            "Entity3 name mismatch: expected 'Node2D_1', got '{}'",
            name3
        )));
    }

    // ASSERTION: All entities should have GodotNodeHandle
    if !entity_has_component::<GodotNodeHandle>(ctx, entity1) {
        return Err(TestError::assertion(
            "Entity1 missing GodotNodeHandle component",
        ));
    }
    if !entity_has_component::<GodotNodeHandle>(ctx, entity2) {
        return Err(TestError::assertion(
            "Entity2 missing GodotNodeHandle component",
        ));
    }
    if !entity_has_component::<GodotNodeHandle>(ctx, entity3) {
        return Err(TestError::assertion(
            "Entity3 missing GodotNodeHandle component",
        ));
    }
    // Clean up
    node1.queue_free();
    node2.queue_free();
    node3.queue_free();

    Ok(())
}

/// Test that different node types work correctly
pub fn test_different_node_types_create_entities(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    // Set up the environment
    let mut env = ctx.setup_full_integration();

    let _initial_count = count_entities_with::<GodotNodeHandle>(ctx);

    // Create different node types
    let mut node3d = godot::classes::Node3D::new_alloc();
    node3d.set_name("TestNode3D");
    let node3d_id = node3d.instance_id();

    let mut node2d = godot::classes::Node2D::new_alloc();
    node2d.set_name("TestNode2D");
    let node2d_id = node2d.instance_id();

    let mut control = godot::classes::Control::new_alloc();
    control.set_name("TestControl");
    let control_id = control.instance_id();

    let mut rigidbody = godot::classes::RigidBody3D::new_alloc();
    rigidbody.set_name("TestRigidBody");
    let rigidbody_id = rigidbody.instance_id();

    // Add them to the scene
    env.add_node_to_scene(node3d.clone());
    env.add_node_to_scene(node2d.clone().upcast::<godot::classes::Node>());
    env.add_node_to_scene(control.clone().upcast::<godot::classes::Node>());
    env.add_node_to_scene(rigidbody.clone());

    ctx.app.update();

    let _final_count = count_entities_with::<GodotNodeHandle>(ctx);

    // ASSERTION: All entities should be created
    let entity_node3d = find_entity_for_node(ctx, node3d_id)
        .ok_or_else(|| TestError::assertion("Node3D did not create entity"))?;

    let entity_node2d = find_entity_for_node(ctx, node2d_id)
        .ok_or_else(|| TestError::assertion("Node2D did not create entity"))?;

    let entity_control = find_entity_for_node(ctx, control_id)
        .ok_or_else(|| TestError::assertion("Control did not create entity"))?;

    let entity_rigidbody = find_entity_for_node(ctx, rigidbody_id)
        .ok_or_else(|| TestError::assertion("RigidBody3D did not create entity"))?;

    // ASSERTION: All entities should have GodotNodeHandle
    let entities_to_check = [
        ("Node3D", entity_node3d),
        ("Node2D", entity_node2d),
        ("Control", entity_control),
        ("RigidBody3D", entity_rigidbody),
    ];

    for (node_type, entity) in &entities_to_check {
        if !entity_has_component::<GodotNodeHandle>(ctx, *entity) {
            return Err(TestError::assertion(format!(
                "{} entity missing GodotNodeHandle",
                node_type
            )));
        }

        let name = get_entity_name(ctx, *entity).ok_or_else(|| {
            TestError::assertion(format!("{} entity missing Name component", node_type))
        })?;

        let expected_name = match *node_type {
            "Node3D" => "TestNode3D",
            "Node2D" => "TestNode2D",
            "Control" => "TestControl",
            "RigidBody3D" => "TestRigidBody",
            _ => unreachable!(),
        };

        if name != expected_name {
            return Err(TestError::assertion(format!(
                "{} name mismatch: expected '{}', got '{}'",
                node_type, expected_name, name
            )));
        }
    }

    // Clean up
    node3d.queue_free();
    node2d.queue_free();
    control.queue_free();
    rigidbody.queue_free();

    Ok(())
}
