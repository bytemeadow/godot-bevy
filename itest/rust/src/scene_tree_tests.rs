/*
 * Scene tree integration tests
 *
 * Tests automatic entity creation, removal, renaming, reparenting,
 * ProtectedNodeEntity, GodotNodeHandle validity, and NodeEntityIndex.
 */

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
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        let initial_count =
            app.with_world_mut(|world| world.query::<&GodotNodeHandle>().iter(world).count());

        let (mut node, _entity) = app.add_node::<godot::classes::Node2D>("TestNode").await;

        let final_count =
            app.with_world_mut(|world| world.query::<&GodotNodeHandle>().iter(world).count());

        assert!(
            final_count > initial_count,
            "Entity should be created for new node, initial={initial_count}, final={final_count}"
        );

        assert!(
            app.has_entity_for_node(node.instance_id()),
            "Entity should have correct GodotNodeHandle"
        );

        app.cleanup().await;
        node.queue_free();
    })
}

/// Test that removing a node generates appropriate events/cleanup
#[itest(async)]
fn test_node_removed_cleanup(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        let (mut node, _entity) = app
            .add_node::<godot::classes::Node2D>("RemovalTestNode")
            .await;

        assert!(
            app.has_entity_for_node(node.instance_id()),
            "Entity should exist before removal"
        );

        node.queue_free();
        // Wait for removal to process (crash-freedom check only;
        // test_node_entity_index_updated_on_remove verifies actual removal).
        app.update().await;

        app.cleanup().await;
    })
}

/// Test that renaming a node is handled correctly
#[itest(async)]
fn test_node_renamed_event(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        let (mut node, _entity) = app.add_node::<godot::classes::Node2D>("OriginalName").await;

        let node_id = node.instance_id();

        node.set_name("RenamedNode");
        // Wait for rename to propagate to ECS
        app.updates(2).await;

        assert!(
            app.has_entity_for_node(node_id),
            "Entity should still exist after rename"
        );

        app.cleanup().await;
        node.queue_free();
    })
}

/// Test that ProtectedNodeEntity prevents despawn when node is freed
#[itest(async)]
fn test_protected_node_entity(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        let (mut node, entity) = app
            .add_node::<godot::classes::Node2D>("ProtectedNode")
            .await;

        let node_id = node.instance_id();

        app.with_world_mut(|world| {
            world.entity_mut(entity).insert(ProtectedNodeEntity);
        });

        node.queue_free();
        // Wait for removal to propagate to ECS
        app.updates(2).await;

        let entity_still_exists = app.with_world(|world| world.get_entity(entity).is_ok());

        assert!(
            entity_still_exists,
            "Protected entity should not be despawned when node is freed"
        );

        let handle_removed = app.with_world(|world| world.get::<GodotNodeHandle>(entity).is_none());

        assert!(
            handle_removed,
            "GodotNodeHandle should be removed from protected entity"
        );

        let index_cleared =
            app.with_world(|world| !world.resource::<NodeEntityIndex>().contains(node_id));

        assert!(
            index_cleared,
            "NodeEntityIndex should remove entry for protected entity when node is freed"
        );

        app.cleanup().await;
    })
}

/// Test that GodotNodeHandle points to correct node
#[itest(async)]
fn test_node_handle_validity(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        let (mut node, entity) = app
            .add_node::<godot::classes::Node2D>("UniqueNodeName")
            .await;

        node.set_position(Vector2::new(42.0, 84.0));

        let position_match = app.with_world_mut(|world| {
            let handle = world
                .get::<GodotNodeHandle>(entity)
                .copied()
                .expect("Entity should have GodotNodeHandle");

            let mut system_state: bevy::ecs::system::SystemState<GodotAccess> =
                bevy::ecs::system::SystemState::new(world);
            let mut godot = system_state.get_mut(world);

            let matched = if let Some(gd_node) = godot.try_get::<godot::classes::Node2D>(handle) {
                let pos = gd_node.get_position();
                (pos.x - 42.0).abs() < 0.1 && (pos.y - 84.0).abs() < 0.1
            } else {
                false
            };

            system_state.apply(world);
            matched
        });

        assert!(
            position_match,
            "GodotNodeHandle should reference correct node"
        );

        app.cleanup().await;
        node.queue_free();
    })
}

/// Test that entity data survives node reparenting
#[itest(async)]
fn test_node_reparenting_preserves_entity(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        let mut parent1 = Node::new_alloc();
        parent1.set_name("Parent1");
        let mut parent2 = Node::new_alloc();
        parent2.set_name("Parent2");

        ctx_clone.scene_tree.clone().add_child(&parent1);
        ctx_clone.scene_tree.clone().add_child(&parent2);

        let mut child = Node::new_alloc();
        child.set_name("Child");
        parent1.clone().add_child(&child);

        // Wait for entities to be created
        app.updates(2).await;

        let entity = app
            .entity_for_node(child.instance_id())
            .expect("Child entity should exist");

        #[derive(bevy::prelude::Component, Clone, Copy, Debug, PartialEq)]
        struct CustomData(i32);

        app.with_world_mut(|world| {
            world.entity_mut(entity).insert(CustomData(42));
        });

        child.reparent(&parent2);
        // Wait for reparent to propagate to ECS
        app.updates(2).await;

        let entity_exists = app.with_world(|world| world.get_entity(entity).is_ok());

        assert!(
            entity_exists,
            "Entity should still exist after reparenting (BUG: entity gets despawned)"
        );

        if entity_exists {
            let data = app.with_world(|world| world.get::<CustomData>(entity).copied());
            assert_eq!(
                data,
                Some(CustomData(42)),
                "Component data should be preserved"
            );

            let child_id = child.instance_id();
            let index_entity =
                app.with_world(|world| world.resource::<NodeEntityIndex>().get(child_id));
            assert_eq!(
                index_entity,
                Some(entity),
                "NodeEntityIndex should still map to same entity after reparenting"
            );
        }

        app.cleanup().await;
        parent1.queue_free();
        parent2.queue_free();
    })
}

/// Test that remove_child() despawns the entity (unlike reparent which preserves it)
#[itest(async)]
fn test_remove_child_despawns_entity(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        let mut parent = Node::new_alloc();
        parent.set_name("RemoveChildParent");
        ctx_clone.scene_tree.clone().add_child(&parent);

        let mut child = Node::new_alloc();
        child.set_name("RemoveChildTest");
        parent.clone().add_child(&child);

        // Wait for entities to be created
        app.updates(2).await;

        let entity = app
            .entity_for_node(child.instance_id())
            .expect("Child entity should exist");

        parent.remove_child(&child);
        // Wait for removal to propagate to ECS
        app.updates(2).await;

        let entity_exists = app.with_world(|world| world.get_entity(entity).is_ok());

        assert!(
            !entity_exists,
            "Entity should be despawned after remove_child()"
        );

        app.cleanup().await;
        parent.queue_free();
    })
}

/// Test that NodeEntityIndex is populated when nodes are added
#[itest(async)]
fn test_node_entity_index_populated_on_add(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        let (mut node, entity) = app
            .add_node::<godot::classes::Node2D>("IndexTestNode")
            .await;

        let node_id = node.instance_id();

        assert!(
            app.has_entity_for_node(node_id),
            "NodeEntityIndex should contain entry for added node"
        );

        let entity_from_index = app.entity_for_node(node_id);

        assert_eq!(
            entity_from_index,
            Some(entity),
            "NodeEntityIndex should map to correct entity"
        );

        app.cleanup().await;
        node.queue_free();
    })
}

/// Test that NodeEntityIndex is updated when nodes are removed
#[itest(async)]
fn test_node_entity_index_updated_on_remove(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        let (mut node, _entity) = app
            .add_node::<godot::classes::Node2D>("IndexRemovalTestNode")
            .await;

        let node_id = node.instance_id();

        assert!(
            app.has_entity_for_node(node_id),
            "Node should be in index after add"
        );

        node.queue_free();
        // Wait for removal to propagate to ECS
        app.updates(2).await;

        assert!(
            !app.has_entity_for_node(node_id),
            "NodeEntityIndex should remove entry when node is freed"
        );

        app.cleanup().await;
    })
}
