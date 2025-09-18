//! Full integration tests for transform syncing with complete scene tree setup
//!
//! Run with: `cargo test --features api-4-3 --test full_integration_transform_tests`

use bevy::math::Vec3;
use bevy::prelude::*;
use godot::classes::Node;
use godot::prelude::*;
use godot_bevy::interop::node_markers::Node3DMarker;
use godot_bevy::interop::{GodotNodeHandle, NodeMarker};
use godot_bevy::plugins::transforms::{GodotTransformSyncPlugin, TransformSyncMode};
use godot_bevy_testability::*;

use godot_bevy_testability::BevyGodotTestContextExt;

// Test that nodes added to the scene tree automatically get Bevy entities with transforms
fn test_scene_tree_node_creates_entity_with_transform(
    ctx: &mut BevyGodotTestContext,
) -> TestResult<()> {
    // Set up full integration environment
    let mut env = ctx.setup_full_integration();

    // Add transform plugin
    ctx.app.add_plugins(GodotTransformSyncPlugin::default());

    // Create and add a node to the scene tree
    let mut node = Node3D::new_alloc();
    node.set_name("TestNode3D");
    node.set_position(Vector3::new(10.0, 20.0, 30.0));
    node.set_scale(Vector3::new(2.0, 2.0, 2.0));

    // Add to scene tree - this should trigger scene tree events
    env.add_node_to_scene(node.clone());

    // Update the app to process scene tree events
    ctx.app.update();

    // Query for the entity that should have been created
    let world = ctx.app.world_mut();
    let mut query =
        world.query_filtered::<(&GodotNodeHandle, &Transform, &Node3DMarker), With<NodeMarker>>();

    let mut found = false;
    for (handle, transform, _) in query.iter(&world) {
        if handle.instance_id() == node.instance_id() {
            found = true;

            // Verify transform was initialized from Godot node
            assert!((transform.translation.x - 10.0).abs() < 0.01);
            assert!((transform.translation.y - 20.0).abs() < 0.01);
            assert!((transform.translation.z - 30.0).abs() < 0.01);
            assert!((transform.scale.x - 2.0).abs() < 0.01);
        }
    }

    assert!(found, "Entity with Node3D should have been created");

    // Clean up
    node.queue_free();

    Ok(())
}

// Test that modifying Bevy transform syncs to Godot with full scene tree
fn test_bevy_to_godot_sync_with_scene_tree(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    // Set up full integration
    let mut env = ctx.setup_full_integration();

    // Add transform plugin
    ctx.app.add_plugins(GodotTransformSyncPlugin::default());

    // Create a node
    let mut node = Node3D::new_alloc();
    node.set_name("SyncTest");
    node.set_position(Vector3::new(0.0, 0.0, 0.0));

    // Add to scene tree
    env.add_node_to_scene(node.clone());

    // Update to process initial events
    ctx.app.update();

    // Find the entity and modify its transform
    let entity_to_modify = {
        let mut world = ctx.app.world_mut();
        let mut query = world.query_filtered::<(Entity, &GodotNodeHandle), With<Node3DMarker>>();
        let mut found_entity = None;
        for (entity, handle) in query.iter(&world) {
            if handle.instance_id() == node.instance_id() {
                found_entity = Some(entity);
                break;
            }
        }
        found_entity.expect("Should find entity for node")
    };

    // Modify the transform
    ctx.app
        .world_mut()
        .entity_mut(entity_to_modify)
        .insert(Transform::from_xyz(100.0, 200.0, 300.0));

    // Update to trigger sync
    ctx.app.update();

    // Verify Godot node was updated
    let pos = node.get_position();
    assert!(
        (pos.x - 100.0).abs() < 0.01,
        "X should be 100.0, got {}",
        pos.x
    );
    assert!(
        (pos.y - 200.0).abs() < 0.01,
        "Y should be 200.0, got {}",
        pos.y
    );
    assert!(
        (pos.z - 300.0).abs() < 0.01,
        "Z should be 300.0, got {}",
        pos.z
    );

    // Clean up
    node.queue_free();

    Ok(())
}

// Test two-way sync with full scene tree integration
fn test_two_way_sync_with_scene_tree(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    // Set up full integration
    let mut env = ctx.setup_full_integration();

    // Add transform plugin with two-way sync
    ctx.app.add_plugins(GodotTransformSyncPlugin {
        sync_mode: TransformSyncMode::TwoWay,
        auto_sync: true,
    });

    // Create a node
    let mut node = Node3D::new_alloc();
    node.set_name("TwoWayTest");
    node.set_position(Vector3::new(10.0, 10.0, 10.0));

    // Add to scene tree
    env.add_node_to_scene(node.clone());

    // Initial update
    ctx.app.update();

    // Test 1: Modify Godot and verify Bevy updates
    node.set_position(Vector3::new(50.0, 60.0, 70.0));
    ctx.app.update();

    let world = ctx.app.world_mut();
    let mut query = world.query_filtered::<(&GodotNodeHandle, &Transform), With<Node3DMarker>>();

    let mut verified_bevy_update = false;
    for (handle, transform) in query.iter(&world) {
        if handle.instance_id() == node.instance_id() {
            assert!((transform.translation.x - 50.0).abs() < 0.01);
            assert!((transform.translation.y - 60.0).abs() < 0.01);
            assert!((transform.translation.z - 70.0).abs() < 0.01);
            verified_bevy_update = true;
        }
    }
    assert!(verified_bevy_update, "Bevy transform should have updated");

    // Test 2: Modify Bevy and verify Godot updates
    let entity_to_modify = {
        let mut world = ctx.app.world_mut();
        let mut query = world.query_filtered::<(Entity, &GodotNodeHandle), With<Node3DMarker>>();
        let mut found_entity = None;
        for (entity, handle) in query.iter(&world) {
            if handle.instance_id() == node.instance_id() {
                found_entity = Some(entity);
                break;
            }
        }
        found_entity.expect("Should find entity")
    };

    ctx.app
        .world_mut()
        .entity_mut(entity_to_modify)
        .get_mut::<Transform>()
        .unwrap()
        .translation = Vec3::new(150.0, 160.0, 170.0);

    ctx.app.update();

    let pos = node.get_position();
    assert!((pos.x - 150.0).abs() < 0.01);
    assert!((pos.y - 160.0).abs() < 0.01);
    assert!((pos.z - 170.0).abs() < 0.01);

    // Clean up
    node.queue_free();

    Ok(())
}

// Test parent-child hierarchy with scene tree
fn test_hierarchy_transform_sync(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    // Set up full integration
    let mut env = ctx.setup_full_integration();

    // Add transform plugin
    ctx.app.add_plugins(GodotTransformSyncPlugin::default());

    // Create parent and child nodes
    let mut parent = Node3D::new_alloc();
    parent.set_name("ParentNode");
    parent.set_position(Vector3::new(100.0, 0.0, 0.0));

    let mut child = Node3D::new_alloc();
    child.set_name("ChildNode");
    child.set_position(Vector3::new(50.0, 0.0, 0.0)); // Local to parent

    // Set up hierarchy
    parent.add_child(&child.clone().upcast::<Node>());

    // Add to scene tree
    env.add_node_to_scene(parent.clone());

    // Update to process scene tree events
    ctx.app.update();

    // Both entities should exist
    let mut world = ctx.app.world_mut();
    let mut query = world.query::<&GodotNodeHandle>();
    let count = query.iter(&world).count();
    assert!(count >= 2, "Should have at least parent and child entities");

    // Find parent entity and move it
    let parent_entity = {
        let mut world = ctx.app.world_mut();
        let mut query = world.query_filtered::<(Entity, &GodotNodeHandle), With<Node3DMarker>>();
        let mut found_entity = None;
        for (entity, handle) in query.iter(&world) {
            if handle.instance_id() == parent.instance_id() {
                found_entity = Some(entity);
                break;
            }
        }
        found_entity.expect("Should find parent entity")
    };

    ctx.app
        .world_mut()
        .entity_mut(parent_entity)
        .get_mut::<Transform>()
        .unwrap()
        .translation = Vec3::new(200.0, 100.0, 0.0);

    ctx.app.update();

    // Verify parent moved
    let parent_pos = parent.get_position();
    assert!((parent_pos.x - 200.0).abs() < 0.01);
    assert!((parent_pos.y - 100.0).abs() < 0.01);

    // Child's local position should remain the same
    let child_pos = child.get_position();
    assert!(
        (child_pos.x - 50.0).abs() < 0.01,
        "Child local X should stay 50.0"
    );

    // Clean up
    parent.queue_free(); // This also frees the child

    Ok(())
}

// Generate the test main function
bevy_godot_test_main! {
    test_scene_tree_node_creates_entity_with_transform,
    test_bevy_to_godot_sync_with_scene_tree,
    test_two_way_sync_with_scene_tree,
    test_hierarchy_transform_sync,
}
