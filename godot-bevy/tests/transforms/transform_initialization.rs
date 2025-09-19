//! Tests that verify Godot transforms are correctly initialized in Bevy entities

use bevy::prelude::*;
use godot::prelude::*;
use godot_bevy::plugins::transforms::GodotTransformSyncPlugin;
use godot_bevy_testability::*;

use crate::transforms::utils::{assert_vec3_eq_with_tolerance, find_entity_for_node};

/// Verifies that a Godot node's transform is correctly copied to the Bevy entity on creation
pub fn godot_transform_initializes_bevy_entity(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    // Arrange
    let mut env = ctx.setup_full_integration();
    ctx.app.add_plugins(GodotTransformSyncPlugin::default());

    // Create a node with a specific transform (including rotation)
    let expected_position = Vector3::new(123.45, -67.89, 42.0);
    let expected_rotation = Vector3::new(0.5, 1.0, 1.5); // Euler angles in radians
    let expected_scale = Vector3::new(2.0, 0.5, 3.0);

    let mut node = godot::classes::Node3D::new_alloc();
    node.set_position(expected_position);
    node.set_rotation(expected_rotation);
    node.set_scale(expected_scale);

    // Store the transform before adding to scene (Godot may modify it when added to tree)
    let initial_transform = node.get_transform();

    // Act
    env.add_node_to_scene(node.clone());
    ctx.app.update();

    // Assert - Find the entity and verify its transform matches
    let entity =
        find_entity_for_node(ctx, node.instance_id()).expect("Entity should be created for Node3D");

    let world = ctx.app.world_mut();
    let transform = world
        .entity(entity)
        .get::<Transform>()
        .expect("Entity should have Transform component");

    // Verify position
    assert_vec3_eq_with_tolerance(
        transform.translation,
        Vec3::new(
            expected_position.x,
            expected_position.y,
            expected_position.z,
        ),
        0.001,
        "Position should be initialized from Godot node",
    );

    // Verify scale
    assert_vec3_eq_with_tolerance(
        transform.scale,
        Vec3::new(expected_scale.x, expected_scale.y, expected_scale.z),
        0.001,
        "Scale should be initialized from Godot node",
    );

    // Verify rotation by checking if both transforms produce the same result
    // (Euler angles can have multiple representations for the same rotation)
    // We use the transform BEFORE adding to scene, as Godot may modify it when added
    let test_vectors = vec![Vec3::X, Vec3::Y, Vec3::Z];

    // Use the initial transform from BEFORE adding to scene, as that's what gets synced to Bevy
    let godot_basis = initial_transform.basis;
    // Get the scale to normalize the basis vectors
    let godot_scale = godot_basis.get_scale();

    for test_vec in test_vectors {
        // For rotation comparison, use the basis columns which represent
        // the transformed unit vectors, but normalize them to remove scale
        let godot_rotated = match test_vec {
            v if v == Vec3::X => {
                let col = godot_basis.col_a();
                Vec3::new(
                    col.x / godot_scale.x,
                    col.y / godot_scale.x,
                    col.z / godot_scale.x,
                )
            }
            v if v == Vec3::Y => {
                let col = godot_basis.col_b();
                Vec3::new(
                    col.x / godot_scale.y,
                    col.y / godot_scale.y,
                    col.z / godot_scale.y,
                )
            }
            _ => {
                // Vec3::Z
                let col = godot_basis.col_c();
                Vec3::new(
                    col.x / godot_scale.z,
                    col.y / godot_scale.z,
                    col.z / godot_scale.z,
                )
            }
        };

        // Transform using Bevy's quaternion
        let bevy_rotated = transform.rotation * test_vec;

        // They should produce the same result
        assert_vec3_eq_with_tolerance(
            bevy_rotated,
            godot_rotated,
            0.001,
            &format!("Rotation should transform {:?} identically", test_vec),
        );
    }

    // Cleanup
    node.queue_free();
    Ok(())
}

/// Verifies that nodes created at origin have identity transforms in Bevy
pub fn node_at_origin_has_identity_transform(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    // Arrange
    let mut env = ctx.setup_full_integration();
    ctx.app.add_plugins(GodotTransformSyncPlugin::default());

    let mut node = godot::classes::Node3D::new_alloc();
    // Node3D defaults to origin with no rotation and unit scale

    // Act
    env.add_node_to_scene(node.clone());
    ctx.app.update();

    // Assert
    let entity = find_entity_for_node(ctx, node.instance_id()).expect("Entity should be created");

    let world = ctx.app.world_mut();
    let transform = world.entity(entity).get::<Transform>().unwrap();

    assert_eq!(transform.translation, Vec3::ZERO, "Position should be zero");
    assert_eq!(
        transform.rotation,
        Quat::IDENTITY,
        "Rotation should be identity"
    );
    assert_eq!(transform.scale, Vec3::ONE, "Scale should be one");

    // Cleanup
    node.queue_free();
    Ok(())
}

/// Verifies that Y-axis rotation is correctly synced
pub fn y_axis_rotation_syncs_correctly(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    // Arrange
    let mut env = ctx.setup_full_integration();
    ctx.app.add_plugins(GodotTransformSyncPlugin::default());

    // 90° rotation around Y axis
    let mut node = godot::classes::Node3D::new_alloc();
    node.set_rotation(Vector3::new(0.0, std::f32::consts::FRAC_PI_2, 0.0));

    // Act
    env.add_node_to_scene(node.clone());
    ctx.app.update();

    // Assert
    let entity = find_entity_for_node(ctx, node.instance_id())
        .expect("Entity should be created for Y-axis rotation");

    let world = ctx.app.world_mut();
    let transform = world.entity(entity).get::<Transform>().unwrap();

    // After 90° Y rotation, X axis should point toward -Z
    let rotated = transform.rotation * Vec3::X;
    assert_vec3_eq_with_tolerance(
        rotated,
        Vec3::new(0.0, 0.0, -1.0),
        0.01,
        "90° Y rotation should rotate X axis to -Z axis",
    );

    // Cleanup
    node.queue_free();
    Ok(())
}

/// Verifies that compound rotations (multiple axes) are correctly synced
pub fn compound_rotations_sync_correctly(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    // Arrange
    let mut env = ctx.setup_full_integration();
    ctx.app.add_plugins(GodotTransformSyncPlugin::default());

    // Test a compound rotation
    let rotation = Vector3::new(0.5, 1.0, 1.5);
    let mut node = godot::classes::Node3D::new_alloc();
    node.set_rotation(rotation);

    let initial_transform = node.get_transform();

    // Act
    env.add_node_to_scene(node.clone());
    ctx.app.update();

    // Assert
    let entity = find_entity_for_node(ctx, node.instance_id()).expect("Entity should be created");

    let world = ctx.app.world_mut();
    let transform = world.entity(entity).get::<Transform>().unwrap();

    // Verify by comparing transformation of multiple test vectors
    let test_vectors = [Vec3::X, Vec3::Y, Vec3::Z];
    let basis = initial_transform.basis;

    for (i, test_vec) in test_vectors.iter().enumerate() {
        let expected = match i {
            0 => basis.col_a(),
            1 => basis.col_b(),
            _ => basis.col_c(),
        };

        let rotated = transform.rotation * *test_vec;
        assert_vec3_eq_with_tolerance(
            rotated,
            Vec3::new(expected.x, expected.y, expected.z),
            0.001,
            &format!(
                "Compound rotation should correctly transform {:?}",
                test_vec
            ),
        );
    }

    // Cleanup
    node.queue_free();
    Ok(())
}

/// Verifies that extreme position values are handled correctly
pub fn extreme_position_values_handled_correctly(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    // Arrange
    let mut env = ctx.setup_full_integration();
    ctx.app.add_plugins(GodotTransformSyncPlugin::default());

    let mut node = godot::classes::Node3D::new_alloc();
    let extreme_pos = Vector3::new(1e6, -1e6, 1e-6);
    node.set_position(extreme_pos);

    // Act
    env.add_node_to_scene(node.clone());
    ctx.app.update();

    // Assert
    let entity = find_entity_for_node(ctx, node.instance_id())
        .expect("Entity should be created for extreme positions");

    let world = ctx.app.world_mut();
    let transform = world.entity(entity).get::<Transform>().unwrap();

    assert_vec3_eq_with_tolerance(
        transform.translation,
        Vec3::new(extreme_pos.x, extreme_pos.y, extreme_pos.z),
        0.01,
        "Should handle extreme positions",
    );

    // Cleanup
    node.queue_free();
    Ok(())
}

/// Verifies that extreme scale values are handled correctly
pub fn extreme_scale_values_handled_correctly(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    // Arrange
    let mut env = ctx.setup_full_integration();
    ctx.app.add_plugins(GodotTransformSyncPlugin::default());

    let mut node = godot::classes::Node3D::new_alloc();
    let extreme_scale = Vector3::new(1000.0, 1000.0, 0.001);
    node.set_scale(extreme_scale);

    // Act
    env.add_node_to_scene(node.clone());
    ctx.app.update();

    // Assert
    let entity = find_entity_for_node(ctx, node.instance_id())
        .expect("Entity should be created for extreme scales");

    let world = ctx.app.world_mut();
    let transform = world.entity(entity).get::<Transform>().unwrap();

    assert_vec3_eq_with_tolerance(
        transform.scale,
        Vec3::new(extreme_scale.x, extreme_scale.y, extreme_scale.z),
        0.01,
        "Should handle extreme scales",
    );

    // Cleanup
    node.queue_free();
    Ok(())
}
