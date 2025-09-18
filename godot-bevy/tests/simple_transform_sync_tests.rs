//! Simple transform sync tests that work within the existing test framework
//!
//! Run with: `cargo test --features api-4-3 --test simple_transform_sync_tests`

use bevy::math::Vec3;
use bevy::prelude::*;
use godot::prelude::*;
use godot_bevy::interop::node_markers::Node3DMarker;
use godot_bevy::interop::{GodotNodeHandle, NodeMarker};
use godot_bevy::plugins::transforms::{
    GodotTransformSyncPlugin, TransformSyncMetadata, TransformSyncMode,
};
use godot_bevy_testability::*;

// Test basic transform initialization from Godot node
fn test_transform_initialization(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy::plugins::core::GodotBaseCorePlugin;
    use godot_bevy::plugins::transforms::IntoBevyTransform;

    // Add core plugin
    ctx.app.add_plugins(GodotBaseCorePlugin);

    // Create a Godot node with specific transform
    let mut node = Node3D::new_alloc();
    node.set_name("TestNode");
    node.set_position(Vector3::new(10.0, 20.0, 30.0));
    node.set_scale(Vector3::new(2.0, 3.0, 4.0));

    // Get the Godot transform and convert to Bevy
    let godot_transform = node.get_transform();
    let bevy_transform = godot_transform.to_bevy_transform();

    // Create entity with the transform
    let entity = ctx
        .app
        .world_mut()
        .spawn((
            GodotNodeHandle::from_instance_id(node.instance_id()),
            Node3DMarker,
            NodeMarker,
            bevy_transform,
        ))
        .id();

    // Verify the transform matches
    let world = ctx.app.world();
    let transform = world.entity(entity).get::<Transform>().unwrap();

    assert!((transform.translation.x - 10.0).abs() < 0.01);
    assert!((transform.translation.y - 20.0).abs() < 0.01);
    assert!((transform.translation.z - 30.0).abs() < 0.01);
    assert!((transform.scale.x - 2.0).abs() < 0.01);
    assert!((transform.scale.y - 3.0).abs() < 0.01);
    assert!((transform.scale.z - 4.0).abs() < 0.01);

    // Clean up
    node.queue_free();

    Ok(())
}

// Test Bevy to Godot sync
fn test_bevy_to_godot_sync(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy::plugins::core::GodotBaseCorePlugin;

    // Initialize and add plugins
    ctx.initialize_godot_bevy_resources();
    ctx.app.add_plugins(GodotBaseCorePlugin);
    ctx.app.add_plugins(GodotTransformSyncPlugin::default());

    // Create a Godot node
    let mut node = Node3D::new_alloc();
    node.set_name("SyncNode");
    node.set_position(Vector3::new(0.0, 0.0, 0.0));

    let instance_id = node.instance_id();

    // Create entity
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

    // Update once to establish baseline
    ctx.app.update();

    // Modify Bevy transform
    ctx.app
        .world_mut()
        .entity_mut(entity)
        .get_mut::<Transform>()
        .unwrap()
        .translation = Vec3::new(100.0, 200.0, 300.0);

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

// Test two-way sync
fn test_two_way_sync(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy::plugins::core::GodotBaseCorePlugin;

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
    node.set_position(Vector3::new(10.0, 10.0, 10.0));

    let instance_id = node.instance_id();

    // Create entity
    let entity = ctx
        .app
        .world_mut()
        .spawn((
            GodotNodeHandle::from_instance_id(instance_id),
            Node3DMarker,
            Transform::from_xyz(10.0, 10.0, 10.0),
            TransformSyncMetadata::default(),
        ))
        .id();

    // Update once
    ctx.app.update();

    // Test 1: Modify Godot and verify Bevy updates
    node.set_position(Vector3::new(50.0, 60.0, 70.0));
    ctx.app.update();

    let world = ctx.app.world();
    let bevy_transform = world.entity(entity).get::<Transform>().unwrap();
    assert!((bevy_transform.translation.x - 50.0).abs() < 0.01);
    assert!((bevy_transform.translation.y - 60.0).abs() < 0.01);
    assert!((bevy_transform.translation.z - 70.0).abs() < 0.01);

    // Test 2: Modify Bevy and verify Godot updates
    ctx.app
        .world_mut()
        .entity_mut(entity)
        .get_mut::<Transform>()
        .unwrap()
        .translation = Vec3::new(150.0, 160.0, 170.0);

    ctx.app.update();

    let godot_position = node.get_position();
    println!(
        "After Bevy->Godot sync: Godot position = {:?}",
        godot_position
    );
    assert!(
        (godot_position.x - 150.0).abs() < 0.01,
        "X: {} vs 150.0",
        godot_position.x
    );
    assert!(
        (godot_position.y - 160.0).abs() < 0.01,
        "Y: {} vs 160.0",
        godot_position.y
    );
    assert!(
        (godot_position.z - 170.0).abs() < 0.01,
        "Z: {} vs 170.0",
        godot_position.z
    );

    // Clean up
    node.queue_free();

    Ok(())
}

// Test rotation sync
fn test_rotation_sync(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use bevy::math::Quat;
    use godot_bevy::plugins::core::GodotBaseCorePlugin;

    // Initialize and add plugins
    ctx.initialize_godot_bevy_resources();
    ctx.app.add_plugins(GodotBaseCorePlugin);
    ctx.app.add_plugins(GodotTransformSyncPlugin::default());

    // Create a Godot node with rotation
    let mut node = Node3D::new_alloc();
    node.set_name("RotationNode");
    node.set_rotation_degrees(Vector3::new(45.0, 90.0, 0.0));

    let instance_id = node.instance_id();

    // Create entity
    let entity = ctx
        .app
        .world_mut()
        .spawn((
            GodotNodeHandle::from_instance_id(instance_id),
            Node3DMarker,
            Transform::from_rotation(Quat::from_euler(
                bevy::math::EulerRot::XYZ,
                45.0_f32.to_radians(),
                90.0_f32.to_radians(),
                0.0_f32.to_radians(),
            )),
            TransformSyncMetadata::default(),
        ))
        .id();

    // Update once
    ctx.app.update();

    // Modify Bevy rotation
    ctx.app
        .world_mut()
        .entity_mut(entity)
        .get_mut::<Transform>()
        .unwrap()
        .rotation = Quat::from_euler(
        bevy::math::EulerRot::XYZ,
        30.0_f32.to_radians(),
        60.0_f32.to_radians(),
        90.0_f32.to_radians(),
    );

    // Update to trigger sync
    ctx.app.update();

    // Verify Godot node rotation was updated (approximately)
    let rotation_deg = node.get_rotation_degrees();
    // Note: Due to Euler angle conversions, we allow more tolerance
    assert!(
        (rotation_deg.x - 30.0).abs() < 1.0,
        "X rotation should be ~30.0, got {}",
        rotation_deg.x
    );
    assert!(
        (rotation_deg.y - 60.0).abs() < 1.0,
        "Y rotation should be ~60.0, got {}",
        rotation_deg.y
    );
    assert!(
        (rotation_deg.z - 90.0).abs() < 1.0,
        "Z rotation should be ~90.0, got {}",
        rotation_deg.z
    );

    // Clean up
    node.queue_free();

    Ok(())
}

// Generate the test main function
bevy_godot_test_main! {
    test_transform_initialization,
    test_bevy_to_godot_sync,
    test_two_way_sync,
    test_rotation_sync,
}
