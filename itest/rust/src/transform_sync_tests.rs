/*
 * Transform synchronization tests
 *
 * Tests all transform sync modes using Bevy-style TestApp API:
 * - OneWay (Bevy → Godot only)
 * - TwoWay (bidirectional)
 * - Disabled (no sync)
 *
 * Uses explicit frame-by-frame control with app.update().await
 */

use bevy::prelude::*;
use godot::prelude::*;
use godot::obj::NewAlloc;
use godot_bevy_itest_macros::itest;

use crate::framework::{TestContext, await_frames, TestApp};

/// Test that transforms sync from Bevy to Godot (OneWay mode - default)
#[itest(async)]
fn test_bevy_to_godot_transform_sync(ctx: &TestContext) -> godot::task::TaskHandle {
    use godot_bevy::prelude::*;

    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        await_frames(1).await;

        // Create a Godot Node2D
        let mut node = godot::classes::Node2D::new_alloc();
        node.set_name("BevyMoverNode");
        node.set_position(Vector2::new(0.0, 0.0));
        ctx_clone.scene_tree.clone().add_child(&node);

        let node_id = node.instance_id();

        // Create test app with transform sync (OneWay mode)
        let mut app = TestApp::new(&ctx_clone, move |app| {
            app.add_plugins(GodotTransformSyncPlugin::default());
            app.insert_resource(GodotTransformConfig::one_way());

            // Spawn entity with Godot node handle
            app.add_systems(Startup, move |mut commands: Commands| {
                commands.spawn((
                    GodotNodeHandle::from_instance_id(node_id),
                    Transform::default(),
                    TransformSyncMetadata::default(),
                    Node2DMarker,
                ));
            });

            // System to move entity each frame
            app.add_systems(Update, |mut q: Query<&mut Transform>| {
                for mut transform in q.iter_mut() {
                    transform.translation.x += 1.0;
                }
            });
        }).await;

        // Frame 1: Initial setup
        app.update().await;

        let start_pos = node.get_position().x;

        // Frame 2-5: Move in Bevy, should sync to Godot
        for _ in 0..4 {
            app.update().await;
        }

        let end_pos = node.get_position().x;

        assert!(end_pos > start_pos,
            "Godot node should move (Bevy→Godot sync), start={:.1}, end={:.1}",
            start_pos, end_pos);

        println!("✓ Bevy→Godot transform sync: moved from {:.1} to {:.1}", start_pos, end_pos);

        // Cleanup: free BevyApp BEFORE freeing node
        app.cleanup();
        node.queue_free();
        await_frames(1).await;
    })
}

/// Test that transforms sync from Godot to Bevy (TwoWay mode)
#[itest(async)]
fn test_godot_to_bevy_transform_sync(ctx: &TestContext) -> godot::task::TaskHandle {
    use godot_bevy::prelude::*;

    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        await_frames(1).await;

        // Create a Godot Node2D
        let mut node = godot::classes::Node2D::new_alloc();
        node.set_name("GodotMoverNode");
        node.set_position(Vector2::new(0.0, 0.0));
        ctx_clone.scene_tree.clone().add_child(&node);

        let node_id = node.instance_id();

        // Create test app with TwoWay transform sync
        let mut app = TestApp::new(&ctx_clone, move |app| {
            app.add_plugins(GodotTransformSyncPlugin::default());
            app.insert_resource(GodotTransformConfig::two_way());

            // Spawn entity with Godot node handle
            app.add_systems(Startup, move |mut commands: Commands| {
                commands.spawn((
                    GodotNodeHandle::from_instance_id(node_id),
                    Transform::default(),
                    TransformSyncMetadata::default(),
                    Node2DMarker,
                ));
            });
        }).await;

        // Frame 1: Initial setup
        app.update().await;

        let entity = app.single_entity_with::<Transform>();
        let initial_x = app.with_world(|world| {
            world.get::<Transform>(entity).unwrap().translation.x
        });

        // Move the Godot node (should sync to Bevy in TwoWay mode)
        node.set_position(Vector2::new(10.0, 0.0));

        // Frame 2-3: Wait for sync
        app.update().await;
        app.update().await;

        let synced_x = app.with_world(|world| {
            world.get::<Transform>(entity).unwrap().translation.x
        });

        assert!((synced_x - 10.0).abs() < 0.1,
            "Bevy should detect Godot transform changes, expected ~10.0, got {:.1}",
            synced_x);

        println!("✓ Godot→Bevy transform sync: {:.1} → {:.1}", initial_x, synced_x);

        // Cleanup: free BevyApp BEFORE freeing node
        app.cleanup();
        node.queue_free();
        await_frames(1).await;
    })
}

/// Test bidirectional transform sync (TwoWay mode)
#[itest(async)]
fn test_bidirectional_transform_sync(ctx: &TestContext) -> godot::task::TaskHandle {
    use godot_bevy::prelude::*;

    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        await_frames(1).await;

        // Create two Godot Node2Ds
        let mut bevy_node = godot::classes::Node2D::new_alloc();
        bevy_node.set_name("BevyControlled");
        bevy_node.set_position(Vector2::new(0.0, 0.0));
        ctx_clone.scene_tree.clone().add_child(&bevy_node);

        let mut godot_node = godot::classes::Node2D::new_alloc();
        godot_node.set_name("GodotControlled");
        godot_node.set_position(Vector2::new(0.0, 0.0));
        ctx_clone.scene_tree.clone().add_child(&godot_node);

        let bevy_id = bevy_node.instance_id();
        let godot_id = godot_node.instance_id();

        // Create test app with bidirectional sync
        let mut app = TestApp::new(&ctx_clone, move |app| {
            app.add_plugins(GodotTransformSyncPlugin::default());
            app.insert_resource(GodotTransformConfig::two_way());

            // Spawn entities for both nodes
            app.add_systems(Startup, move |mut commands: Commands| {
                commands.spawn((
                    GodotNodeHandle::from_instance_id(bevy_id),
                    Transform::default(),
                    TransformSyncMetadata::default(),
                    Node2DMarker,
                ));
                commands.spawn((
                    GodotNodeHandle::from_instance_id(godot_id),
                    Transform::default(),
                    TransformSyncMetadata::default(),
                    Node2DMarker,
                ));
            });

            // System to move the first entity (Bevy→Godot test)
            app.add_systems(Update, move |mut q: Query<(&GodotNodeHandle, &mut Transform)>| {
                for (handle, mut transform) in q.iter_mut() {
                    if handle.instance_id() == bevy_id {
                        transform.translation.x += 1.0;
                    }
                }
            });
        }).await;

        // Frame 1: Initial setup
        app.update().await;

        let bevy_start = bevy_node.get_position().x;

        // Move Godot node (tests Godot→Bevy sync)
        godot_node.set_position(Vector2::new(20.0, 0.0));

        // Frame 2-5: Run updates, checking both directions
        for _ in 0..4 {
            app.update().await;
        }

        let bevy_end = bevy_node.get_position().x;

        // Check Bevy→Godot sync
        assert!(bevy_end > bevy_start,
            "Bevy-controlled node should move (Bevy→Godot), start={:.1}, end={:.1}",
            bevy_start, bevy_end);

        // Check Godot→Bevy sync
        let godot_entity_x = app.with_world_mut(|world| {
            let mut query = world.query::<(&GodotNodeHandle, &Transform)>();
            for (handle, transform) in query.iter(world) {
                if handle.instance_id() == godot_id {
                    return transform.translation.x;
                }
            }
            0.0
        });

        assert!((godot_entity_x - 20.0).abs() < 0.1,
            "Godot-controlled entity should sync to Bevy (Godot→Bevy), expected ~20.0, got {:.1}",
            godot_entity_x);

        println!("✓ Bidirectional sync: Bevy {:.1}→{:.1}, Godot→Bevy {:.1}",
                 bevy_start, bevy_end, godot_entity_x);

        // Cleanup: free BevyApp BEFORE freeing nodes
        app.cleanup();
        bevy_node.queue_free();
        godot_node.queue_free();
        await_frames(1).await;
    })
}

/// Test that sync can be disabled
#[itest(async)]
fn test_transform_sync_disabled(ctx: &TestContext) -> godot::task::TaskHandle {
    use godot_bevy::prelude::*;

    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        await_frames(1).await;

        // Create a Godot Node2D
        let mut node = godot::classes::Node2D::new_alloc();
        node.set_name("NoSyncNode");
        node.set_position(Vector2::new(0.0, 0.0));
        ctx_clone.scene_tree.clone().add_child(&node);

        let node_id = node.instance_id();

        // Create test app with sync DISABLED
        let mut app = TestApp::new(&ctx_clone, move |app| {
            app.add_plugins(GodotTransformSyncPlugin::default());
            app.insert_resource(GodotTransformConfig::disabled());

            // Spawn entity with Godot node handle
            app.add_systems(Startup, move |mut commands: Commands| {
                commands.spawn((
                    GodotNodeHandle::from_instance_id(node_id),
                    Transform::default(),
                    TransformSyncMetadata::default(),
                    Node2DMarker,
                ));
            });

            // System to move entity in Bevy (should NOT sync to Godot)
            app.add_systems(Update, |mut q: Query<&mut Transform>| {
                for mut transform in q.iter_mut() {
                    transform.translation.x += 10.0;
                }
            });
        }).await;

        // Frame 1: Initial setup
        app.update().await;

        let start_pos = node.get_position().x;

        // Frame 2-5: Modify in Bevy (should NOT sync)
        for _ in 0..4 {
            app.update().await;
        }

        let end_pos = node.get_position().x;

        // Verify Bevy entity moved internally
        let entity = app.single_entity_with::<Transform>();
        let bevy_x = app.with_world(|world| {
            world.get::<Transform>(entity).unwrap().translation.x
        });

        assert!(bevy_x > 0.0, "Bevy entity should move internally, got {:.1}", bevy_x);
        assert_eq!(end_pos, start_pos,
            "Godot node should NOT move when sync disabled, start={:.1}, end={:.1}",
            start_pos, end_pos);

        println!("✓ Transform sync disabled: Godot at {:.1}, Bevy at {:.1} (no sync)", start_pos, bevy_x);

        // Cleanup: free BevyApp BEFORE freeing node
        app.cleanup();
        node.queue_free();
        await_frames(1).await;
    })
}
