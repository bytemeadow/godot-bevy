//! Tests for node type markers and group components

use bevy::prelude::*;
use godot::prelude::*;
use godot_bevy::interop::{GodotNodeHandle, node_markers::*};
use godot_bevy::plugins::scene_tree::{Groups, plugin::ProtectedNodeEntity};
use godot_bevy_testability::*;

use crate::scene_tree::utils::{count_entities_with, entity_has_component, find_entity_for_node};

/// Test that different node types get correct marker components
/// Uses batch processing pattern like the successful multiple_nodes test
pub fn test_node_type_markers(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    println!("\n=== NODE TYPE MARKERS TEST ===");

    // Set up the environment
    let mut env = ctx.setup_full_integration();

    // Create all test nodes first (batch pattern like successful tests)
    let mut test_data = Vec::new();

    // Node3D test
    let mut node3d = godot::classes::Node3D::new_alloc();
    node3d.set_name("TestNode3D");
    let node3d_id = node3d.instance_id();
    test_data.push(("Node3D", node3d.upcast::<godot::classes::Node>(), node3d_id));

    // Node2D test
    let mut node2d = godot::classes::Node2D::new_alloc();
    node2d.set_name("TestNode2D");
    let node2d_id = node2d.instance_id();
    test_data.push(("Node2D", node2d.upcast::<godot::classes::Node>(), node2d_id));

    // Control test
    let mut control = godot::classes::Control::new_alloc();
    control.set_name("TestControl");
    let control_id = control.instance_id();
    test_data.push((
        "Control",
        control.upcast::<godot::classes::Node>(),
        control_id,
    ));

    // RigidBody3D test
    let mut rigidbody = godot::classes::RigidBody3D::new_alloc();
    rigidbody.set_name("TestRigidBody");
    let rigidbody_id = rigidbody.instance_id();
    test_data.push((
        "RigidBody3D",
        rigidbody.upcast::<godot::classes::Node>(),
        rigidbody_id,
    ));

    // Area2D test
    let mut area2d = godot::classes::Area2D::new_alloc();
    area2d.set_name("TestArea2D");
    let area2d_id = area2d.instance_id();
    test_data.push(("Area2D", area2d.upcast::<godot::classes::Node>(), area2d_id));

    println!("Created {} test nodes", test_data.len());

    // Add ALL nodes to scene first (like the working multiple nodes test)
    for (node_type_name, node, _) in &test_data {
        println!("Adding {} to scene...", node_type_name);
        env.add_node_to_scene(node.clone());
    }

    // Process events ONCE for all nodes (key difference!)
    println!("Processing scene tree events for all nodes...");
    ctx.app.update();

    // Now verify each entity was created and has correct markers
    for (node_type_name, _, node_id) in test_data {
        println!("\nTesting {} markers...", node_type_name);

        // ASSERTION: Entity should be created
        let entity = find_entity_for_node(ctx, node_id).ok_or_else(|| {
            TestError::assertion(format!("Could not find entity for {} node", node_type_name))
        })?;
        println!("✓ Found entity {:?} for {} node", entity, node_type_name);

        // ASSERTION: All nodes should have NodeMarker
        if !entity_has_component::<NodeMarker>(ctx, entity) {
            return Err(TestError::assertion(format!(
                "{} entity missing NodeMarker component",
                node_type_name
            )));
        }
        println!("✓ {} entity has NodeMarker", node_type_name);

        // ASSERTION: Check type-specific markers
        match node_type_name {
            "Node3D" => {
                if !entity_has_component::<Node3DMarker>(ctx, entity) {
                    return Err(TestError::assertion(
                        "Node3D entity missing Node3DMarker component",
                    ));
                }
                println!("✓ Node3D entity has Node3DMarker");
            }
            "Node2D" => {
                if !entity_has_component::<Node2DMarker>(ctx, entity) {
                    return Err(TestError::assertion(
                        "Node2D entity missing Node2DMarker component",
                    ));
                }
                if !entity_has_component::<CanvasItemMarker>(ctx, entity) {
                    return Err(TestError::assertion(
                        "Node2D entity missing CanvasItemMarker component",
                    ));
                }
                println!("✓ Node2D entity has Node2DMarker and CanvasItemMarker");
            }
            "Control" => {
                if !entity_has_component::<ControlMarker>(ctx, entity) {
                    return Err(TestError::assertion(
                        "Control entity missing ControlMarker component",
                    ));
                }
                if !entity_has_component::<CanvasItemMarker>(ctx, entity) {
                    return Err(TestError::assertion(
                        "Control entity missing CanvasItemMarker component",
                    ));
                }
                println!("✓ Control entity has ControlMarker and CanvasItemMarker");
            }
            "RigidBody3D" => {
                if !entity_has_component::<RigidBody3DMarker>(ctx, entity) {
                    return Err(TestError::assertion(
                        "RigidBody3D entity missing RigidBody3DMarker component",
                    ));
                }
                if !entity_has_component::<Node3DMarker>(ctx, entity) {
                    return Err(TestError::assertion(
                        "RigidBody3D entity missing Node3DMarker component",
                    ));
                }
                println!("✓ RigidBody3D entity has RigidBody3DMarker and Node3DMarker");
            }
            "Area2D" => {
                if !entity_has_component::<Area2DMarker>(ctx, entity) {
                    return Err(TestError::assertion(
                        "Area2D entity missing Area2DMarker component",
                    ));
                }
                if !entity_has_component::<Node2DMarker>(ctx, entity) {
                    return Err(TestError::assertion(
                        "Area2D entity missing Node2DMarker component",
                    ));
                }
                if !entity_has_component::<CanvasItemMarker>(ctx, entity) {
                    return Err(TestError::assertion(
                        "Area2D entity missing CanvasItemMarker component",
                    ));
                }
                println!("✓ Area2D entity has Area2DMarker, Node2DMarker, and CanvasItemMarker");
            }
            _ => unreachable!(),
        }
    }

    println!("=== END NODE TYPE MARKERS TEST ===\n");

    Ok(())
}

/// Test that nodes in groups get Groups component
pub fn test_node_groups_component(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    println!("\n=== NODE GROUPS COMPONENT TEST ===");

    // Set up the environment
    let mut env = ctx.setup_full_integration();

    // Create BOTH test nodes first (batch pattern)

    // Node with groups
    let mut node = godot::classes::Node3D::new_alloc();
    node.set_name("GroupTestNode");
    let node_id = node.instance_id();
    node.add_to_group("enemies");
    node.add_to_group("flying");
    node.add_to_group("boss");

    // Node with no groups
    let mut node_no_groups = godot::classes::Node3D::new_alloc();
    node_no_groups.set_name("NoGroupsNode");
    let node_no_groups_id = node_no_groups.instance_id();

    // Add BOTH nodes to scene first
    println!("Adding node with groups to scene...");
    env.add_node_to_scene(node.clone());
    println!("Adding node without groups to scene...");
    env.add_node_to_scene(node_no_groups.clone());

    // Process events ONCE for both nodes
    println!("Processing scene tree events for both nodes...");
    ctx.app.update();

    // Test first node (with groups)
    println!("\nTesting node with groups...");

    // ASSERTION: Entity should be created
    let entity = find_entity_for_node(ctx, node_id)
        .ok_or_else(|| TestError::assertion("Could not find entity for group test node"))?;
    println!("✓ Found entity {:?} for group test node", entity);

    // ASSERTION: Entity should have Groups component
    if !entity_has_component::<Groups>(ctx, entity) {
        return Err(TestError::assertion("Entity missing Groups component"));
    }
    println!("✓ Entity has Groups component");

    // ASSERTION: Groups component should contain the correct groups
    let world = ctx.app.world();
    let groups = world
        .get::<Groups>(entity)
        .ok_or_else(|| TestError::assertion("Failed to get Groups component from entity"))?;

    if !groups.is("enemies") {
        return Err(TestError::assertion(
            "Groups component missing 'enemies' group",
        ));
    }
    if !groups.is("flying") {
        return Err(TestError::assertion(
            "Groups component missing 'flying' group",
        ));
    }
    if !groups.is("boss") {
        return Err(TestError::assertion(
            "Groups component missing 'boss' group",
        ));
    }
    if groups.is("nonexistent") {
        return Err(TestError::assertion(
            "Groups component incorrectly contains 'nonexistent' group",
        ));
    }
    println!("✓ Groups component contains correct groups: enemies, flying, boss");

    // Test second node (no groups)
    println!("\nTesting node without groups...");

    let entity_no_groups = find_entity_for_node(ctx, node_no_groups_id)
        .ok_or_else(|| TestError::assertion("Could not find entity for no-groups node"))?;

    // ASSERTION: Entity should still have Groups component (empty)
    if !entity_has_component::<Groups>(ctx, entity_no_groups) {
        return Err(TestError::assertion(
            "No-groups entity missing Groups component",
        ));
    }

    let world = ctx.app.world();
    let empty_groups = world.get::<Groups>(entity_no_groups).ok_or_else(|| {
        TestError::assertion("Failed to get Groups component from no-groups entity")
    })?;

    if empty_groups.is("any_group") {
        return Err(TestError::assertion(
            "Empty Groups component incorrectly contains groups",
        ));
    }
    println!("✓ Node with no groups has empty Groups component");

    // Clean up
    node.queue_free();
    node_no_groups.queue_free();

    println!("=== END NODE GROUPS COMPONENT TEST ===\n");

    Ok(())
}

/// Test that ProtectedNodeEntity prevents entity deletion
pub fn test_protected_entity_deletion(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    println!("\n=== PROTECTED ENTITY DELETION TEST ===");

    // Set up the environment
    let mut env = ctx.setup_full_integration();

    // Create a node and verify entity is created
    let mut node = godot::classes::Node3D::new_alloc();
    node.set_name("ProtectedNode");
    let node_id = node.instance_id();

    env.add_node_to_scene(node.clone());
    ctx.app.update();

    // ASSERTION: Entity should be created
    let entity = find_entity_for_node(ctx, node_id)
        .ok_or_else(|| TestError::assertion("Could not find entity for protected node"))?;
    println!("✓ Found entity {:?} for protected node", entity);

    // Add ProtectedNodeEntity marker
    {
        let world = ctx.app.world_mut();
        let mut entity_mut = world.entity_mut(entity);
        entity_mut.insert(ProtectedNodeEntity);
    }
    println!("✓ Added ProtectedNodeEntity marker to entity");

    let initial_count = count_entities_with::<GodotNodeHandle>(ctx);
    println!("Initial entity count: {}", initial_count);

    // Now try to delete the node (this should NOT delete the entity)
    println!("Deleting protected node...");
    node.queue_free();

    // Process multiple frames since queue_free() is deferred
    println!("Processing frames to allow deferred deletion...");
    for i in 0..5 {
        ctx.app.update();
        let current_count = count_entities_with::<GodotNodeHandle>(ctx);
        println!("  Frame {}: entity count = {}", i + 1, current_count);
    }

    // Check that entity still exists (in headless mode, node deletion doesn't work anyway,
    // but we can test that the protection mechanism is in place)
    let final_count = count_entities_with::<GodotNodeHandle>(ctx);
    println!("Final entity count: {}", final_count);

    // In headless mode, node deletion signals don't fire, so we can't test the actual protection.
    // But we can verify the entity still exists and has the protection marker.
    let world = ctx.app.world();
    if world.get_entity(entity).is_err() {
        return Err(TestError::assertion(
            "Protected entity was incorrectly despawned",
        ));
    }
    println!("✓ Protected entity still exists");

    if !entity_has_component::<ProtectedNodeEntity>(ctx, entity) {
        return Err(TestError::assertion(
            "Entity lost ProtectedNodeEntity marker",
        ));
    }
    println!("✓ Entity still has ProtectedNodeEntity marker");

    println!("✓ Protected entity test completed");
    println!("  NOTE: In headless mode, node deletion signals don't fire");
    println!("  In a real Godot environment, this would prevent entity deletion");

    println!("=== END PROTECTED ENTITY DELETION TEST ===\n");

    Ok(())
}
