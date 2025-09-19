//! Tests for different transform synchronization modes

use bevy::prelude::*;
use godot::prelude::*;
use godot_bevy::plugins::transforms::{GodotTransformSyncPlugin, TransformSyncMode};
use godot_bevy_testability::*;

use crate::transforms::utils::{assert_vec3_eq, find_entity_for_node};

/// Verifies that OneWay mode only syncs from Bevy to Godot, not vice versa
pub fn one_way_mode_syncs_bevy_to_godot_only(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    // Arrange
    let mut env = ctx.setup_full_integration();
    ctx.app.add_plugins(GodotTransformSyncPlugin {
        sync_mode: TransformSyncMode::OneWay,
        auto_sync: true,
    });

    let mut node = godot::classes::Node3D::new_alloc();
    node.set_position(Vector3::new(10.0, 20.0, 30.0));
    env.add_node_to_scene(node.clone());
    ctx.app.update();

    let entity = find_entity_for_node(ctx, node.instance_id()).unwrap();

    // Test 1: Bevy changes should sync to Godot
    ctx.app
        .add_systems(Update, move |mut query: Query<&mut Transform>| {
            if let Ok(mut transform) = query.get_mut(entity) {
                transform.translation = Vec3::new(100.0, 200.0, 300.0);
            }
        });
    ctx.app.update();

    let godot_pos = node.get_position();
    assert_vec3_eq(
        Vec3::new(godot_pos.x, godot_pos.y, godot_pos.z),
        Vec3::new(100.0, 200.0, 300.0),
        "Bevy changes should sync to Godot in OneWay mode",
    );

    // Test 2: Godot changes should NOT sync back to Bevy
    node.set_position(Vector3::new(500.0, 600.0, 700.0));
    ctx.app.update();

    let world = ctx.app.world_mut();
    let transform = world.entity(entity).get::<Transform>().unwrap();
    assert_vec3_eq(
        transform.translation,
        Vec3::new(100.0, 200.0, 300.0), // Should still be the Bevy value
        "Godot changes should NOT sync to Bevy in OneWay mode",
    );

    // Cleanup
    node.queue_free();
    Ok(())
}

/// Verifies that TwoWay mode syncs bidirectionally with proper conflict resolution
pub fn two_way_mode_syncs_bidirectionally(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    // Arrange
    let mut env = ctx.setup_full_integration();
    ctx.app.add_plugins(GodotTransformSyncPlugin {
        sync_mode: TransformSyncMode::TwoWay,
        auto_sync: true,
    });

    let mut node = godot::classes::Node3D::new_alloc();
    node.set_position(Vector3::new(10.0, 20.0, 30.0));
    env.add_node_to_scene(node.clone());
    ctx.app.update();

    let entity = find_entity_for_node(ctx, node.instance_id()).unwrap();

    // Test 1: Godot changes should sync to Bevy
    node.set_position(Vector3::new(50.0, 60.0, 70.0));
    ctx.app.update();

    let world = ctx.app.world_mut();
    let transform = world.entity(entity).get::<Transform>().unwrap();
    assert_vec3_eq(
        transform.translation,
        Vec3::new(50.0, 60.0, 70.0),
        "Godot changes should sync to Bevy in TwoWay mode",
    );

    // Test 2: Bevy changes should still sync to Godot
    ctx.app
        .add_systems(Update, move |mut query: Query<&mut Transform>| {
            if let Ok(mut transform) = query.get_mut(entity) {
                transform.translation = Vec3::new(150.0, 250.0, 350.0);
            }
        });
    ctx.app.update();

    let godot_pos = node.get_position();
    assert_vec3_eq(
        Vec3::new(godot_pos.x, godot_pos.y, godot_pos.z),
        Vec3::new(150.0, 250.0, 350.0),
        "Bevy changes should sync to Godot in TwoWay mode",
    );

    // Cleanup
    node.queue_free();
    Ok(())
}

/// Verifies that Disabled mode prevents all synchronization
pub fn disabled_mode_prevents_all_sync(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    // Arrange
    let mut env = ctx.setup_full_integration();
    ctx.app.add_plugins(GodotTransformSyncPlugin {
        sync_mode: TransformSyncMode::Disabled,
        auto_sync: true,
    });

    let initial_position = Vector3::new(10.0, 20.0, 30.0);
    let mut node = godot::classes::Node3D::new_alloc();
    node.set_position(initial_position);
    env.add_node_to_scene(node.clone());
    ctx.app.update();

    let entity = find_entity_for_node(ctx, node.instance_id()).unwrap();

    // Get initial Bevy transform
    let _initial_bevy_pos = {
        let world = ctx.app.world_mut();
        let transform = world.entity(entity).get::<Transform>().unwrap();
        transform.translation
    };

    // Test 1: Bevy changes should NOT sync to Godot
    ctx.app
        .add_systems(Update, move |mut query: Query<&mut Transform>| {
            if let Ok(mut transform) = query.get_mut(entity) {
                transform.translation = Vec3::new(100.0, 200.0, 300.0);
            }
        });
    ctx.app.update();

    let godot_pos = node.get_position();
    assert_vec3_eq(
        Vec3::new(godot_pos.x, godot_pos.y, godot_pos.z),
        Vec3::new(initial_position.x, initial_position.y, initial_position.z),
        "Bevy changes should NOT sync to Godot when disabled",
    );

    // Test 2: Godot changes should NOT sync to Bevy
    node.set_position(Vector3::new(500.0, 600.0, 700.0));
    ctx.app.update();

    let world = ctx.app.world_mut();
    let transform = world.entity(entity).get::<Transform>().unwrap();

    // Transform should still be what Bevy set it to (since we ran a system)
    assert_vec3_eq(
        transform.translation,
        Vec3::new(100.0, 200.0, 300.0),
        "Godot changes should NOT sync to Bevy when disabled",
    );

    // Cleanup
    node.queue_free();
    Ok(())
}

/// Verifies that sync mode can be changed at runtime
pub fn sync_mode_can_change_at_runtime(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy::plugins::transforms::GodotTransformConfig;
    use godot_bevy_testability::BevyGodotTestContextExt;

    // Arrange - Start with OneWay mode
    let mut env = ctx.setup_full_integration();
    ctx.app.add_plugins(GodotTransformSyncPlugin {
        sync_mode: TransformSyncMode::OneWay,
        auto_sync: true,
    });

    let mut node = godot::classes::Node3D::new_alloc();
    node.set_position(Vector3::new(10.0, 20.0, 30.0));
    env.add_node_to_scene(node.clone());
    ctx.app.update();

    let entity = find_entity_for_node(ctx, node.instance_id()).unwrap();

    // Verify OneWay behavior
    node.set_position(Vector3::new(50.0, 60.0, 70.0));
    ctx.app.update();

    let world = ctx.app.world_mut();
    let transform = world.entity(entity).get::<Transform>().unwrap();
    let initial_translation = transform.translation;

    // The position should NOT have changed from Godot in OneWay mode
    assert_ne!(
        initial_translation,
        Vec3::new(50.0, 60.0, 70.0),
        "OneWay mode should not sync Godot to Bevy"
    );

    // Act - Change to TwoWay mode
    ctx.app
        .world_mut()
        .resource_mut::<GodotTransformConfig>()
        .sync_mode = TransformSyncMode::TwoWay;

    // Test TwoWay behavior
    node.set_position(Vector3::new(80.0, 90.0, 100.0));
    ctx.app.update();

    let world = ctx.app.world_mut();
    let transform = world.entity(entity).get::<Transform>().unwrap();
    assert_vec3_eq(
        transform.translation,
        Vec3::new(80.0, 90.0, 100.0),
        "After switching to TwoWay, Godot changes should sync to Bevy",
    );

    // Cleanup
    node.queue_free();
    Ok(())
}
