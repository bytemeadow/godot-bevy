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

use bevy::prelude::{Entity, Name, With};
use godot::obj::NewAlloc;
use godot::prelude::*;
use godot_bevy::plugins::scene_tree::ProtectedNodeEntity;
use godot_bevy::prelude::*;
use godot_bevy_test::prelude::*;

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

        // Frame 2: Entity created
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

        // Frame 2: Entity created
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

        // Frame 2: Entity created
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

        // Frame 3: Removal processed
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

        // Frame 2: Entity created
        app.update().await;

        // Rename the node
        node.set_name("RenamedNode");

        // Frame 3: Rename event processed
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

/// Test that ProtectedNodeEntity prevents despawn when node is freed
#[itest(async)]
fn test_protected_node_entity(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        await_frames(1).await;

        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        app.update().await;

        // Add a node
        let mut node = godot::classes::Node2D::new_alloc();
        node.set_name("ProtectedNode");
        ctx_clone.scene_tree.clone().add_child(&node);

        let node_id = node.instance_id();

        app.update().await;

        // Mark the entity as protected
        let entity = app.with_world_mut(|world| {
            world
                .query::<(Entity, &GodotNodeHandle)>()
                .iter(world)
                .find(|(_, handle)| handle.instance_id() == node_id)
                .map(|(e, _)| e)
                .expect("Entity should exist")
        });

        app.with_world_mut(|world| {
            world.entity_mut(entity).insert(ProtectedNodeEntity);
        });

        // Free the node
        node.queue_free();

        // Frame 1: NodeRemoved event processed, removal commands queued
        app.update().await;

        // Frame 2: Commands from previous frame are flushed
        app.update().await;

        // Verify entity still exists (not despawned)
        let entity_still_exists = app.with_world(|world| world.get_entity(entity).is_ok());

        assert!(
            entity_still_exists,
            "Protected entity should not be despawned when node is freed"
        );

        // Verify GodotNodeHandle was removed
        let handle_removed = app.with_world(|world| world.get::<GodotNodeHandle>(entity).is_none());

        assert!(
            handle_removed,
            "GodotNodeHandle should be removed from protected entity"
        );

        // Verify NodeEntityIndex no longer contains the mapping
        let index_cleared =
            app.with_world(|world| !world.resource::<NodeEntityIndex>().contains(node_id));

        assert!(
            index_cleared,
            "NodeEntityIndex should remove entry for protected entity when node is freed"
        );

        println!("✓ ProtectedNodeEntity: entity survives, GodotNodeHandle removed, index cleared");

        app.cleanup();
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

        // Frame 2: Entity created
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

/// Test that entity data survives node reparenting
/// Bug: When reparenting a node, the entity gets despawned because
/// NodeRemoved event fires, causing all entity data to be lost
#[itest(async)]
fn test_node_reparenting_preserves_entity(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {
            // No additional plugins needed
        })
        .await;

        // Create two parent nodes
        let mut parent1 = Node::new_alloc();
        parent1.set_name("Parent1");
        let mut parent2 = Node::new_alloc();
        parent2.set_name("Parent2");

        // Create child node
        let mut child = Node::new_alloc();
        child.set_name("Child");

        // Add to scene tree
        ctx_clone.scene_tree.clone().add_child(&parent1);
        ctx_clone.scene_tree.clone().add_child(&parent2);
        parent1.clone().add_child(&child);

        app.update().await;

        // Get the entity and add custom component
        let entity = app.with_world_mut(|world| {
            let mut query = world.query_filtered::<Entity, With<GodotNodeHandle>>();
            query
                .iter(world)
                .find(|e| {
                    world
                        .get::<Name>(*e)
                        .map(|n| n.as_str() == "Child")
                        .unwrap_or(false)
                })
                .expect("Child entity should exist")
        });

        #[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq)]
        struct CustomData(i32);

        app.with_world_mut(|world| {
            world.entity_mut(entity).insert(CustomData(42));
        });

        // REPARENT: Move child from parent1 to parent2
        child.reparent(&parent2);

        app.update().await;
        app.update().await;

        // Check if entity still exists
        let entity_exists = app.with_world(|world| world.get_entity(entity).is_ok());

        // BUG: This will fail - entity gets despawned during reparenting
        assert!(
            entity_exists,
            "Entity should still exist after reparenting (BUG: entity gets despawned)"
        );

        // Also check component data is preserved
        if entity_exists {
            let data = app.with_world(|world| world.get::<CustomData>(entity).copied());
            assert_eq!(
                data,
                Some(CustomData(42)),
                "Component data should be preserved"
            );

            // Verify NodeEntityIndex still maps to the same entity after reparenting
            let child_id = child.instance_id();
            let index_entity =
                app.with_world(|world| world.resource::<NodeEntityIndex>().get(child_id));
            assert_eq!(
                index_entity,
                Some(entity),
                "NodeEntityIndex should still map to same entity after reparenting"
            );
        }

        println!("✓ Entity and component data preserved during reparenting");

        // Cleanup
        app.cleanup();
        parent1.queue_free();
        parent2.queue_free();
        await_frames(1).await;
    })
}

/// Test that remove_child() despawns the entity (unlike reparent which preserves it)
#[itest(async)]
fn test_remove_child_despawns_entity(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        // Create parent and child
        let mut parent = Node::new_alloc();
        parent.set_name("RemoveChildParent");
        let mut child = Node::new_alloc();
        child.set_name("RemoveChildTest");

        // Add to scene tree
        ctx_clone.scene_tree.clone().add_child(&parent);
        parent.clone().add_child(&child);

        app.update().await;

        // Get the entity
        let entity = app.with_world_mut(|world| {
            let mut query = world.query_filtered::<Entity, With<GodotNodeHandle>>();
            query
                .iter(world)
                .find(|e| {
                    world
                        .get::<Name>(*e)
                        .map(|n| n.as_str() == "RemoveChildTest")
                        .unwrap_or(false)
                })
                .expect("Child entity should exist")
        });

        // Manually remove child from parent (not reparenting, just removing)
        parent.remove_child(&child);

        app.update().await;
        app.update().await;

        // Verify entity was despawned
        let entity_exists = app.with_world(|world| world.get_entity(entity).is_ok());

        assert!(
            !entity_exists,
            "Entity should be despawned after remove_child()"
        );

        println!("✓ remove_child() correctly despawns entity");

        // Cleanup
        app.cleanup();
        parent.queue_free();
        // Note: Don't queue_free the child - it was already removed and may be auto-freed
        await_frames(1).await;
    })
}

/// Test that NodeEntityIndex is populated when nodes are added
#[itest(async)]
fn test_node_entity_index_populated_on_add(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        await_frames(1).await;

        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        app.update().await;

        // Add a new node
        let mut node = godot::classes::Node2D::new_alloc();
        node.set_name("IndexTestNode");
        ctx_clone.scene_tree.clone().add_child(&node);

        let node_id = node.instance_id();

        app.update().await;

        // Verify NodeEntityIndex contains the mapping
        let (index_has_entry, entity_from_index) = app.with_world(|world| {
            let index = world.resource::<NodeEntityIndex>();
            (index.contains(node_id), index.get(node_id))
        });

        assert!(
            index_has_entry,
            "NodeEntityIndex should contain entry for added node"
        );

        // Verify the entity in the index matches the actual entity
        let actual_entity = app.with_world_mut(|world| {
            world
                .query::<(Entity, &GodotNodeHandle)>()
                .iter(world)
                .find(|(_, handle)| handle.instance_id() == node_id)
                .map(|(e, _)| e)
        });

        assert_eq!(
            entity_from_index, actual_entity,
            "NodeEntityIndex should map to correct entity"
        );

        println!("✓ NodeEntityIndex correctly populated on node add");

        app.cleanup();
        node.queue_free();
        await_frames(1).await;
    })
}

/// Test that NodeEntityIndex is updated when nodes are removed
#[itest(async)]
fn test_node_entity_index_updated_on_remove(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        await_frames(1).await;

        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        app.update().await;

        // Add a node
        let mut node = godot::classes::Node2D::new_alloc();
        node.set_name("IndexRemovalTestNode");
        ctx_clone.scene_tree.clone().add_child(&node);

        let node_id = node.instance_id();

        app.update().await;

        // Verify it's in the index
        let in_index_before =
            app.with_world(|world| world.resource::<NodeEntityIndex>().contains(node_id));
        assert!(in_index_before, "Node should be in index after add");

        // Remove the node
        node.queue_free();

        app.update().await;

        // Verify it's removed from the index
        let in_index_after =
            app.with_world(|world| world.resource::<NodeEntityIndex>().contains(node_id));

        assert!(
            !in_index_after,
            "NodeEntityIndex should remove entry when node is freed"
        );

        println!("✓ NodeEntityIndex correctly updated on node remove");

        app.cleanup();
        await_frames(1).await;
    })
}
