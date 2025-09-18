//! Integration tests for godot-bevy functionality.
//!
//! Run with: `cargo test --features api-4-3 --test bevy_godot_integration_tests`

use bevy::prelude::*;
use godot::prelude::*;
use godot::classes::Node;
use godot_bevy::interop::{GodotNodeHandle, NodeMarker};
use godot_bevy_testability::*;

// Test GodotNodeHandle functionality
fn test_godot_node_handle(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy::plugins::core::GodotBaseCorePlugin;

    // Add core plugin
    ctx.app.add_plugins(GodotBaseCorePlugin);

    // Create a node
    let mut node = Node2D::new_alloc();
    node.set_name("TestNode2D");
    node.set_position(Vector2::new(100.0, 200.0));

    let instance_id = node.instance_id();

    // Create entity with GodotNodeHandle reference
    let entity = ctx
        .app
        .world_mut()
        .spawn((GodotNodeHandle::from_instance_id(instance_id), NodeMarker))
        .id();

    // Update app
    ctx.app.update();

    // Verify we can retrieve the GodotNodeHandle component
    let world = ctx.app.world();
    let godot_node = world.entity(entity).get::<GodotNodeHandle>();
    assert!(
        godot_node.is_some(),
        "GodotNodeHandle component should exist"
    );

    if let Some(godot_node) = godot_node {
        assert_eq!(
            godot_node.instance_id(),
            instance_id,
            "Instance ID should match"
        );
    }

    // Clean up
    node.queue_free();

    Ok(())
}

// Test that we can create Bevy entities with Godot node references
fn test_entity_with_node_marker(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy::plugins::core::GodotBaseCorePlugin;

    // Add core plugin
    ctx.app.add_plugins(GodotBaseCorePlugin);

    // Create multiple nodes and entities
    let mut nodes = Vec::new();
    let mut entities = Vec::new();

    for i in 0..3 {
        let mut node = Node3D::new_alloc();
        node.set_name(format!("TestNode_{}", i).as_str());

        let instance_id = node.instance_id();
        let entity = ctx
            .app
            .world_mut()
            .spawn((
                GodotNodeHandle::from_instance_id(instance_id),
                NodeMarker,
                Transform::from_xyz(i as f32 * 10.0, 0.0, 0.0),
            ))
            .id();

        nodes.push(node);
        entities.push(entity);
    }

    // Update app
    ctx.app.update();

    // Verify all entities exist with their components
    for (i, entity) in entities.iter().enumerate() {
        let world = ctx.app.world();
        let entity_ref = world.entity(*entity);

        assert!(
            entity_ref.get::<GodotNodeHandle>().is_some(),
            "Entity {} should have GodotNodeHandle",
            i
        );
        assert!(
            entity_ref.get::<NodeMarker>().is_some(),
            "Entity {} should have NodeMarker",
            i
        );
        assert!(
            entity_ref.get::<Transform>().is_some(),
            "Entity {} should have Transform",
            i
        );
    }

    // Clean up
    for mut node in nodes {
        node.queue_free();
    }

    Ok(())
}

// Test transform components with Godot nodes
fn test_transform_sync_basic(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy::plugins::core::GodotBaseCorePlugin;

    // Add core plugin
    ctx.app.add_plugins(GodotBaseCorePlugin);

    // Create a Godot node
    let mut node = Node3D::new_alloc();
    node.set_name("TransformNode");
    node.set_position(Vector3::new(10.0, 20.0, 30.0));

    let instance_id = node.instance_id();

    // Create entity with transform and node handle
    let entity = ctx
        .app
        .world_mut()
        .spawn((
            GodotNodeHandle::from_instance_id(instance_id),
            NodeMarker,
            Transform::from_xyz(5.0, 10.0, 15.0),
        ))
        .id();

    // Update app
    ctx.app.update();

    // Verify entity still has its transform
    let world = ctx.app.world();
    let transform = world.entity(entity).get::<Transform>();
    assert!(
        transform.is_some(),
        "Entity should have Transform component"
    );

    if let Some(transform) = transform {
        // Check that transform exists and has expected values
        assert_eq!(transform.translation.x, 5.0);
        assert_eq!(transform.translation.y, 10.0);
        assert_eq!(transform.translation.z, 15.0);
    }

    // Clean up
    node.queue_free();

    Ok(())
}

// Test basic scene tree operations
fn test_scene_tree_operations(_ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    // Create some nodes and verify Godot's scene tree operations
    let mut parent = Node3D::new_alloc();
    parent.set_name("ParentNode");

    let mut child1 = Node3D::new_alloc();
    child1.set_name("Child1");

    let mut child2 = Node3D::new_alloc();
    child2.set_name("Child2");

    // Add children to parent
    parent.add_child(&child1);
    parent.add_child(&child2);

    // Verify parent-child relationships
    assert_eq!(parent.get_child_count(), 2, "Parent should have 2 children");

    let child1_path = NodePath::from("Child1");
    assert!(parent.has_node(&child1_path), "Parent should have Child1");

    let child2_path = NodePath::from("Child2");
    assert!(parent.has_node(&child2_path), "Parent should have Child2");

    // Test node retrieval - get_node_as returns Option<Gd<T>>
    let retrieved_child = parent.try_get_node_as::<Node3D>(&child1_path);
    assert!(
        retrieved_child.is_some(),
        "Should be able to retrieve Child1"
    );

    // Clean up (freeing parent also frees children)
    parent.queue_free();

    Ok(())
}

// Test that creating a Godot node and entity with Transform initializes correctly
fn test_godot_node_creates_entity_with_transform(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy::plugins::core::GodotBaseCorePlugin;
    use godot_bevy::plugins::transforms::{GodotTransformSyncPlugin, IntoBevyTransform};
    use godot_bevy::interop::node_markers::Node3DMarker;

    // Initialize resources and add necessary plugins
    ctx.initialize_godot_bevy_resources();
    ctx.app.add_plugins(GodotBaseCorePlugin);
    ctx.app.add_plugins(GodotTransformSyncPlugin::default());

    // Create a test node
    let mut parent_node = Node3D::new_alloc();
    parent_node.set_name("TestParent");
    parent_node.set_position(Vector3::new(100.0, 200.0, 300.0));
    parent_node.set_rotation_degrees(Vector3::new(45.0, 90.0, 0.0));
    parent_node.set_scale(Vector3::new(2.0, 3.0, 4.0));

    // Get the Godot transform and convert to Bevy
    let godot_transform = parent_node.get_transform();
    let bevy_transform = godot_transform.to_bevy_transform();

    // Manually create entity with the transform from Godot
    let entity = ctx
        .app
        .world_mut()
        .spawn((
            GodotNodeHandle::from_instance_id(parent_node.instance_id()),
            Node3DMarker,
            bevy_transform,
        ))
        .id();

    // Process once
    ctx.app.update();

    // Query for the entity and verify transform
    let world = ctx.app.world();
    let transform = world.entity(entity).get::<Transform>().unwrap();

    // Verify the transform matches what we set on the Godot node
    assert!(
        (transform.translation.x - 100.0).abs() < 0.01,
        "X position should be 100.0, got {}",
        transform.translation.x
    );
    assert!(
        (transform.translation.y - 200.0).abs() < 0.01,
        "Y position should be 200.0, got {}",
        transform.translation.y
    );
    assert!(
        (transform.translation.z - 300.0).abs() < 0.01,
        "Z position should be 300.0, got {}",
        transform.translation.z
    );

    // Scale should also match
    assert!(
        (transform.scale.x - 2.0).abs() < 0.01,
        "X scale should be 2.0, got {}",
        transform.scale.x
    );
    assert!(
        (transform.scale.y - 3.0).abs() < 0.01,
        "Y scale should be 3.0, got {}",
        transform.scale.y
    );
    assert!(
        (transform.scale.z - 4.0).abs() < 0.01,
        "Z scale should be 4.0, got {}",
        transform.scale.z
    );

    // Clean up
    parent_node.queue_free();

    Ok(())
}

// Test that modifying a Bevy transform syncs to Godot
fn test_bevy_transform_syncs_to_godot(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy::plugins::core::GodotBaseCorePlugin;
    use godot_bevy::plugins::transforms::{GodotTransformSyncPlugin, TransformSyncMetadata};
    use godot_bevy::interop::node_markers::Node3DMarker;
    use bevy::math::Vec3;

    // Initialize and add plugins
    ctx.initialize_godot_bevy_resources();
    ctx.app.add_plugins(GodotBaseCorePlugin);
    ctx.app.add_plugins(GodotTransformSyncPlugin::default());

    // Create a Godot node with initial position
    let mut node = Node3D::new_alloc();
    node.set_name("SyncTestNode");
    node.set_position(Vector3::new(0.0, 0.0, 0.0));

    let instance_id = node.instance_id();

    // Create entity with node handle and transform
    let entity = ctx
        .app
        .world_mut()
        .spawn((
            GodotNodeHandle::from_instance_id(instance_id),
            Node3DMarker,
            Transform::from_xyz(0.0, 0.0, 0.0),
            TransformSyncMetadata::default(),
        ))
        .id();

    // First update to establish baseline
    ctx.app.update();

    // Modify the Bevy transform
    ctx.app.world_mut().entity_mut(entity)
        .get_mut::<Transform>().unwrap()
        .translation = Vec3::new(50.0, 75.0, 100.0);

    // Update to trigger sync
    ctx.app.update();

    // Check that the Godot node's position was updated
    let godot_position = node.get_position();
    assert!(
        (godot_position.x - 50.0).abs() < 0.01,
        "Godot X position should be 50.0, got {}",
        godot_position.x
    );
    assert!(
        (godot_position.y - 75.0).abs() < 0.01,
        "Godot Y position should be 75.0, got {}",
        godot_position.y
    );
    assert!(
        (godot_position.z - 100.0).abs() < 0.01,
        "Godot Z position should be 100.0, got {}",
        godot_position.z
    );

    // Clean up
    node.queue_free();

    Ok(())
}

// Test two-way transform sync
fn test_two_way_transform_sync(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy::plugins::core::GodotBaseCorePlugin;
    use godot_bevy::plugins::transforms::{GodotTransformSyncPlugin, TransformSyncMetadata, TransformSyncMode};
    use godot_bevy::interop::node_markers::Node3DMarker;

    // Initialize and add plugins with two-way sync
    ctx.initialize_godot_bevy_resources();
    ctx.app.add_plugins(GodotBaseCorePlugin);
    ctx.app.add_plugins(GodotTransformSyncPlugin {
        sync_mode: TransformSyncMode::TwoWay,
        auto_sync: true,
    });

    // Create a Godot node
    let mut node = Node3D::new_alloc();
    node.set_name("TwoWayNode");
    node.set_position(Vector3::new(10.0, 20.0, 30.0));

    let instance_id = node.instance_id();

    // Create entity
    let entity = ctx
        .app
        .world_mut()
        .spawn((
            GodotNodeHandle::from_instance_id(instance_id),
            Node3DMarker,
            Transform::from_xyz(10.0, 20.0, 30.0),
            TransformSyncMetadata::default(),
        ))
        .id();

    // Update once to establish baseline
    ctx.app.update();

    // Test 1: Modify Godot position and check if Bevy updates
    node.set_position(Vector3::new(100.0, 200.0, 300.0));
    ctx.app.update();

    let world = ctx.app.world();
    let bevy_transform = world.entity(entity).get::<Transform>().unwrap();
    assert!(
        (bevy_transform.translation.x - 100.0).abs() < 0.01,
        "Bevy X should update to 100.0, got {}",
        bevy_transform.translation.x
    );
    assert!(
        (bevy_transform.translation.y - 200.0).abs() < 0.01,
        "Bevy Y should update to 200.0, got {}",
        bevy_transform.translation.y
    );
    assert!(
        (bevy_transform.translation.z - 300.0).abs() < 0.01,
        "Bevy Z should update to 300.0, got {}",
        bevy_transform.translation.z
    );

    // Test 2: Modify Bevy transform and check if Godot updates
    ctx.app.world_mut().entity_mut(entity)
        .get_mut::<Transform>().unwrap()
        .translation = Vec3::new(500.0, 600.0, 700.0);

    ctx.app.update();

    let godot_position = node.get_position();
    assert!(
        (godot_position.x - 500.0).abs() < 0.01,
        "Godot X should update to 500.0, got {}",
        godot_position.x
    );
    assert!(
        (godot_position.y - 600.0).abs() < 0.01,
        "Godot Y should update to 600.0, got {}",
        godot_position.y
    );
    assert!(
        (godot_position.z - 700.0).abs() < 0.01,
        "Godot Z should update to 700.0, got {}",
        godot_position.z
    );

    // Clean up
    node.queue_free();

    Ok(())
}

// Test transform sync with parent-child hierarchy
fn test_transform_sync_hierarchy(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy::plugins::core::GodotBaseCorePlugin;
    use godot_bevy::plugins::scene_tree::GodotSceneTreePlugin;
    use godot_bevy::plugins::transforms::{GodotTransformSyncPlugin, TransformSyncMetadata};
    use godot_bevy::interop::node_markers::Node3DMarker;
    use std::sync::mpsc::channel;

    // Initialize and add plugins
    ctx.initialize_godot_bevy_resources();
    ctx.app.add_plugins(GodotBaseCorePlugin);
    ctx.app.add_plugins(GodotSceneTreePlugin::default());
    ctx.app.add_plugins(GodotTransformSyncPlugin::default());

    // Set up scene tree event channel
    let (_sender, receiver) = channel();
    ctx.app.insert_non_send_resource(godot_bevy::plugins::scene_tree::SceneTreeEventReader(receiver));

    // Create parent and child nodes
    let mut parent = Node3D::new_alloc();
    parent.set_name("Parent");
    parent.set_position(Vector3::new(100.0, 0.0, 0.0));

    let mut child = Node3D::new_alloc();
    child.set_name("Child");
    child.set_position(Vector3::new(50.0, 0.0, 0.0)); // Local position relative to parent

    parent.add_child(&child.clone().upcast::<Node>());

    let parent_id = parent.instance_id();
    let child_id = child.instance_id();

    // Manually create entities for parent and child
    let parent_entity = ctx
        .app
        .world_mut()
        .spawn((
            GodotNodeHandle::from_instance_id(parent_id),
            Node3DMarker,
            Transform::from_xyz(100.0, 0.0, 0.0),
            TransformSyncMetadata::default(),
        ))
        .id();

    let _child_entity = ctx
        .app
        .world_mut()
        .spawn((
            GodotNodeHandle::from_instance_id(child_id),
            Node3DMarker,
            Transform::from_xyz(50.0, 0.0, 0.0),
            TransformSyncMetadata::default(),
        ))
        .id();

    // Update once
    ctx.app.update();

    // Move parent in Bevy and check both nodes in Godot
    ctx.app.world_mut().entity_mut(parent_entity)
        .get_mut::<Transform>().unwrap()
        .translation = Vec3::new(200.0, 100.0, 50.0);

    ctx.app.update();

    // Parent should have moved
    let parent_pos = parent.get_position();
    assert!(
        (parent_pos.x - 200.0).abs() < 0.01,
        "Parent X should be 200.0, got {}",
        parent_pos.x
    );
    assert!(
        (parent_pos.y - 100.0).abs() < 0.01,
        "Parent Y should be 100.0, got {}",
        parent_pos.y
    );
    assert!(
        (parent_pos.z - 50.0).abs() < 0.01,
        "Parent Z should be 50.0, got {}",
        parent_pos.z
    );

    // Child's local position should remain unchanged
    let child_local_pos = child.get_position();
    assert!(
        (child_local_pos.x - 50.0).abs() < 0.01,
        "Child local X should remain 50.0, got {}",
        child_local_pos.x
    );

    // Clean up
    parent.queue_free(); // This also frees the child

    Ok(())
}

// Generate the test main function
bevy_godot_test_main! {
    test_godot_node_handle,
    test_entity_with_node_marker,
    test_transform_sync_basic,
    test_scene_tree_operations,
    test_godot_node_creates_entity_with_transform,
    test_bevy_transform_syncs_to_godot,
    test_two_way_transform_sync,
    test_transform_sync_hierarchy,
}
