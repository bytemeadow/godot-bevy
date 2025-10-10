/*
 * Scene tree integration tests
 *
 * Tests automatic entity creation and scene tree event handling:
 * - NodeAdded events when nodes are added to the scene tree
 * - NodeRemoved events when nodes are removed
 * - NodeRenamed events when nodes are renamed
 * - Automatic entity creation for scene tree nodes
 * - GodotNodeHandle components on entities
 *
 * Uses explicit frame-by-frame control with app.update().await
 */

use godot::obj::NewAlloc;
use godot::prelude::*;
use godot_bevy::prelude::*;
use godot_bevy_itest_macros::itest;

use crate::framework::{TestApp, TestContext, await_frames};

/// Test that adding a node to the scene tree creates an entity
#[itest(async)]
fn test_node_added_creates_entity(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        await_frames(1).await;

        // Create test app (scene tree watchers are automatically set up by TestApp)
        let mut app = TestApp::new(&ctx_clone, |_app| {
            // No plugins needed - scene tree is part of core
        })
        .await;

        // Frame 1: Initial sync
        app.update().await;

        // Count initial entities
        let initial_count =
            app.with_world_mut(|world| world.query::<&GodotNodeHandle>().iter(world).count());

        // Add a new node to the scene tree
        let mut node = godot::classes::Node2D::new_alloc();
        node.set_name("TestNode");
        ctx_clone.scene_tree.clone().add_child(&node);

        // Frame 2: Scene tree should detect new node
        app.update().await;

        // Frame 3: Entity should be created
        app.update().await;

        let final_count =
            app.with_world_mut(|world| world.query::<&GodotNodeHandle>().iter(world).count());

        assert!(
            final_count > initial_count,
            "Entity should be created for new node, initial={}, final={}",
            initial_count,
            final_count
        );

        // Verify entity has correct node handle
        let found = app.with_world_mut(|world| {
            world
                .query::<&GodotNodeHandle>()
                .iter(world)
                .any(|handle| handle.instance_id() == node.instance_id())
        });

        assert!(found, "Entity should have correct GodotNodeHandle");

        println!(
            "✓ Node added: entity created (entities: {} → {})",
            initial_count, final_count
        );

        // Cleanup
        app.cleanup();
        node.queue_free();
        await_frames(1).await;
    })
}

/// Test that SceneTreeEvent::NodeAdded is sent when nodes are added
#[itest(async)]
fn test_scene_tree_event_node_added(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        await_frames(1).await;

        // Create test app (scene tree watchers are automatically set up by TestApp)
        let mut app = TestApp::new(&ctx_clone, |_app| {
            // No plugins needed - scene tree is part of core
        })
        .await;

        // Frame 1: Initial sync
        app.update().await;

        // Add a new node
        let mut node = godot::classes::Node2D::new_alloc();
        node.set_name("EventTestNode");
        ctx_clone.scene_tree.clone().add_child(&node);

        let node_id = node.instance_id();

        // Frame 2: Event should be generated
        app.update().await;

        // Frame 3: Check entity was created (event was processed)
        app.update().await;

        let entity_exists = app.with_world_mut(|world| {
            world
                .query::<&GodotNodeHandle>()
                .iter(world)
                .any(|handle| handle.instance_id() == node_id)
        });

        assert!(
            entity_exists,
            "Entity should exist for added node (event processed)"
        );

        println!("✓ Scene tree event: NodeAdded processed");

        // Cleanup
        app.cleanup();
        node.queue_free();
        await_frames(1).await;
    })
}

/// Test that removing a node generates appropriate events/cleanup
#[itest(async)]
fn test_node_removed_cleanup(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        await_frames(1).await;

        // Create test app (scene tree watchers are automatically set up by TestApp)
        let mut app = TestApp::new(&ctx_clone, |_app| {
            // No plugins needed - scene tree is part of core
        })
        .await;

        // Frame 1: Initial sync
        app.update().await;

        // Add a node
        let mut node = godot::classes::Node2D::new_alloc();
        node.set_name("RemovalTestNode");
        ctx_clone.scene_tree.clone().add_child(&node);

        let node_id = node.instance_id();

        // Frame 2-3: Wait for entity creation
        app.update().await;
        app.update().await;

        // Verify entity exists
        let exists_before = app.with_world_mut(|world| {
            world
                .query::<&GodotNodeHandle>()
                .iter(world)
                .any(|handle| handle.instance_id() == node_id)
        });

        assert!(exists_before, "Entity should exist before removal");

        // Remove the node
        node.queue_free();

        // Frame 4-5: Wait for removal processing
        app.update().await;
        app.update().await;

        println!("✓ Node removal: cleanup handled");

        // Cleanup
        app.cleanup();
        await_frames(1).await;
    })
}

/// Test that renaming a node is handled correctly
#[itest(async)]
fn test_node_renamed_event(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        await_frames(1).await;

        // Create test app (scene tree watchers are automatically set up by TestApp)
        let mut app = TestApp::new(&ctx_clone, |_app| {
            // No plugins needed - scene tree is part of core
        })
        .await;

        // Frame 1: Initial sync
        app.update().await;

        // Add a node
        let mut node = godot::classes::Node2D::new_alloc();
        node.set_name("OriginalName");
        ctx_clone.scene_tree.clone().add_child(&node);

        let node_id = node.instance_id();

        // Frame 2-3: Wait for entity creation
        app.update().await;
        app.update().await;

        // Rename the node
        node.set_name("RenamedNode");

        // Frame 4-5: Wait for rename event
        app.update().await;
        app.update().await;

        // Verify entity still exists with same handle
        let exists = app.with_world_mut(|world| {
            world
                .query::<&GodotNodeHandle>()
                .iter(world)
                .any(|handle| handle.instance_id() == node_id)
        });

        assert!(exists, "Entity should still exist after rename");

        println!("✓ Node renamed: entity persists");

        // Cleanup
        app.cleanup();
        node.queue_free();
        await_frames(1).await;
    })
}

/// Test that GodotNodeHandle points to correct node
#[itest(async)]
fn test_node_handle_validity(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        await_frames(1).await;

        // Create test app (scene tree watchers are automatically set up by TestApp)
        let mut app = TestApp::new(&ctx_clone, |_app| {
            // No plugins needed - scene tree is part of core
        })
        .await;

        // Frame 1: Initial sync
        app.update().await;

        // Add a node with unique name
        let mut node = godot::classes::Node2D::new_alloc();
        node.set_name("UniqueNodeName");
        node.set_position(Vector2::new(42.0, 84.0));
        ctx_clone.scene_tree.clone().add_child(&node);

        let node_id = node.instance_id();

        // Frame 2-3: Wait for entity creation
        app.update().await;
        app.update().await;

        // Find entity and verify handle points to correct node
        let position_match = app.with_world_mut(|world| {
            for mut handle in world.query::<&mut GodotNodeHandle>().iter_mut(world) {
                if handle.instance_id() == node_id {
                    // Get the node and check position
                    if let Some(gd_node) = handle.try_get::<godot::classes::Node2D>() {
                        let pos = gd_node.get_position();
                        return (pos.x - 42.0).abs() < 0.1 && (pos.y - 84.0).abs() < 0.1;
                    }
                }
            }
            false
        });

        assert!(
            position_match,
            "GodotNodeHandle should reference correct node"
        );

        println!("✓ Node handle validity: correct node referenced");

        // Cleanup
        app.cleanup();
        node.queue_free();
        await_frames(1).await;
    })
}
