//! Hierarchy transform synchronization tests

use bevy::math::Vec3;
use bevy::prelude::*;
use godot::prelude::*;
use godot_bevy::interop::GodotNodeHandle;
use godot_bevy::interop::node_markers::Node3DMarker;
use godot_bevy::plugins::transforms::GodotTransformSyncPlugin;
use godot_bevy_testability::BevyGodotTestContextExt;
use godot_bevy_testability::*;

/// Test parent-child hierarchy with scene tree
pub fn test_hierarchy_transform_sync(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    // Set up full integration
    let mut env = ctx.setup_full_integration();

    // Add transform plugin
    ctx.app.add_plugins(GodotTransformSyncPlugin::default());

    // Create parent and child nodes
    let mut parent = godot::classes::Node3D::new_alloc();
    parent.set_name("ParentNode");
    parent.set_position(Vector3::new(100.0, 0.0, 0.0));

    let mut child = godot::classes::Node3D::new_alloc();
    child.set_name("ChildNode");
    child.set_position(Vector3::new(50.0, 0.0, 0.0)); // Local to parent

    // Set up hierarchy
    parent.add_child(&child.clone().upcast::<godot::classes::Node>());

    // Add to scene tree
    env.add_node_to_scene(parent.clone());

    // Update to process scene tree events
    ctx.app.update();

    // Both entities should exist
    let world = ctx.app.world_mut();
    let mut query = world.query::<&GodotNodeHandle>();
    let count = query.iter(world).count();
    assert!(count >= 2, "Should have at least parent and child entities");

    // Find parent entity and move it
    let parent_entity = {
        let world = ctx.app.world_mut();
        let mut query = world.query_filtered::<(Entity, &GodotNodeHandle), With<Node3DMarker>>();
        let mut found_entity = None;
        for (entity, handle) in query.iter(world) {
            if handle.instance_id() == parent.instance_id() {
                found_entity = Some(entity);
                break;
            }
        }
        found_entity.expect("Should find parent entity")
    };

    ctx.app
        .add_systems(Update, move |mut query: Query<&mut Transform>| {
            if let Ok(mut transform) = query.get_mut(parent_entity) {
                transform.translation = Vec3::new(200.0, 100.0, 0.0);
            }
        });

    ctx.app.update();

    // Verify parent moved
    let parent_pos = parent.get_position();
    assert!(
        (parent_pos.x - 200.0).abs() < 0.01,
        "Parent X should be 200.0, got {}",
        parent_pos.x
    );

    // Clean up
    parent.queue_free();

    Ok(())
}
