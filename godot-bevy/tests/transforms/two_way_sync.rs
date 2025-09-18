//! Two-way transform synchronization tests

use bevy::math::Vec3;
use bevy::prelude::*;
use godot::prelude::*;
use godot_bevy::interop::GodotNodeHandle;
use godot_bevy::interop::node_markers::Node3DMarker;
use godot_bevy::plugins::transforms::{GodotTransformSyncPlugin, TransformSyncMode};
use godot_bevy_testability::BevyGodotTestContextExt;
use godot_bevy_testability::*;

/// Test two-way sync with full scene tree integration
pub fn test_two_way_sync_with_scene_tree(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    // Set up full integration
    let mut env = ctx.setup_full_integration();

    // Add transform plugin with two-way sync
    ctx.app.add_plugins(GodotTransformSyncPlugin {
        sync_mode: TransformSyncMode::TwoWay,
        auto_sync: true,
    });

    // Create a node
    let mut node = godot::classes::Node3D::new_alloc();
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
    for (handle, transform) in query.iter(world) {
        if handle.instance_id() == node.instance_id() {
            assert!((transform.translation.x - 50.0).abs() < 0.01);
            assert!((transform.translation.y - 60.0).abs() < 0.01);
            assert!((transform.translation.z - 70.0).abs() < 0.01);
            verified_bevy_update = true;
        }
    }
    assert!(verified_bevy_update, "Bevy transform should have updated");

    // Test 2: Modify Bevy and verify Godot updates
    // In two-way sync, we need to modify Bevy transforms in a system that runs during Update
    // to avoid being overwritten by pre_update_godot_transforms

    // Find the entity
    let entity_to_modify = {
        let world = ctx.app.world_mut();
        let mut query = world.query_filtered::<(Entity, &GodotNodeHandle), With<Node3DMarker>>();
        let mut found_entity = None;
        for (entity, handle) in query.iter(world) {
            if handle.instance_id() == node.instance_id() {
                found_entity = Some(entity);
                break;
            }
        }
        found_entity.expect("Should find entity")
    };

    // Create a system that will modify the transform during Update schedule
    ctx.app
        .add_systems(Update, move |mut query: Query<&mut Transform>| {
            if let Ok(mut transform) = query.get_mut(entity_to_modify) {
                transform.translation = Vec3::new(500.0, 600.0, 700.0);
            }
        });

    // Run update which will:
    // 1. Run PreUpdate (pre_update_godot_transforms syncs Godot->Bevy)
    // 2. Run Update (our system modifies Bevy transform)
    // 3. Run Last (post_update_godot_transforms syncs Bevy->Godot)
    ctx.app.update();

    let pos = node.get_position();
    assert!(
        (pos.x - 500.0).abs() < 0.01,
        "X should be 500.0, got {}",
        pos.x
    );
    assert!(
        (pos.y - 600.0).abs() < 0.01,
        "Y should be 600.0, got {}",
        pos.y
    );
    assert!(
        (pos.z - 700.0).abs() < 0.01,
        "Z should be 700.0, got {}",
        pos.z
    );

    // Clean up
    node.queue_free();

    Ok(())
}
