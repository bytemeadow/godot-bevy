//! Basic transform synchronization tests

use bevy::math::Vec3;
use bevy::prelude::*;
use godot::prelude::*;
use godot_bevy::interop::node_markers::Node3DMarker;
use godot_bevy::interop::{GodotNodeHandle, NodeMarker};
use godot_bevy::plugins::transforms::GodotTransformSyncPlugin;
use godot_bevy_testability::*;

/// Test that a Godot node creates a Bevy entity with correct transform
pub fn test_godot_node_creates_entity_with_transform(
    ctx: &mut BevyGodotTestContext,
) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    // Set up full integration with scene tree watcher
    let mut env = ctx.setup_full_integration();

    // Create and add a Node3D
    let mut node = godot::classes::Node3D::new_alloc();
    node.set_name("TestNode");
    node.set_position(Vector3::new(100.0, 200.0, 300.0));

    env.add_node_to_scene(node.clone());
    ctx.app.update();

    // Verify entity was created with GodotNodeHandle
    let world = ctx.app.world_mut();
    let mut query = world.query_filtered::<(&GodotNodeHandle, &NodeMarker), With<Node3DMarker>>();

    let mut found = false;
    for (handle, _marker) in query.iter(world) {
        if handle.instance_id() == node.instance_id() {
            found = true;
            break;
        }
    }

    assert!(
        found,
        "Should create entity with GodotNodeHandle for Node3D"
    );

    // Clean up
    node.queue_free();

    Ok(())
}

/// Test Bevy to Godot transform sync
pub fn test_bevy_to_godot_sync(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    use godot_bevy_testability::BevyGodotTestContextExt;

    // Set up full integration with watchers
    let mut env = ctx.setup_full_integration();

    // Add transform sync plugin
    ctx.app.add_plugins(GodotTransformSyncPlugin::default());

    // Create and add a Node3D
    let mut node = godot::classes::Node3D::new_alloc();
    node.set_name("TestNode");
    node.set_position(Vector3::new(10.0, 20.0, 30.0));

    env.add_node_to_scene(node.clone());
    ctx.app.update();

    // Find the entity and modify its transform
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

    // Modify the Bevy transform in a system (to work with change detection)
    ctx.app
        .add_systems(Update, move |mut query: Query<&mut Transform>| {
            if let Ok(mut transform) = query.get_mut(entity_to_modify) {
                transform.translation = Vec3::new(100.0, 200.0, 300.0);
            }
        });

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
