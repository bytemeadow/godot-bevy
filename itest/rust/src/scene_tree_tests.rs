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

        let (node, _entity) = app.add_node::<godot::classes::Node2D>("TestNode").await;

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
        node.free();
    })
}

/// `_bevy_exclude` is subtree-wide: neither the marked node nor its descendants are
/// mirrored, while unmarked siblings still are. Exercises the runtime NodeAdded path's
/// ancestor walk (the child's parent carries the meta, not the child itself).
#[itest(async)]
fn test_bevy_exclude_skips_subtree(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        let mut excluded = Node::new_alloc();
        excluded.set_meta("_bevy_exclude", &true.to_variant());
        let excluded_child = Node::new_alloc();
        let excluded_child_id = excluded_child.instance_id();
        excluded.clone().add_child(&excluded_child);

        let sibling = Node::new_alloc();
        let sibling_id = sibling.instance_id();

        ctx_clone.scene_tree.clone().add_child(&excluded);
        ctx_clone.scene_tree.clone().add_child(&sibling);

        app.updates(3).await;

        assert!(
            !app.has_entity_for_node(excluded.instance_id()),
            "node carrying _bevy_exclude must not be mirrored"
        );
        assert!(
            !app.has_entity_for_node(excluded_child_id),
            "descendant of an excluded node must not be mirrored (subtree-wide)"
        );
        assert!(
            app.has_entity_for_node(sibling_id),
            "an unmarked sibling must still be mirrored"
        );

        app.cleanup().await;
        excluded.free();
        sibling.free();
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
        node.free();
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
            let mut godot = system_state
                .get_mut(world)
                .expect("system params should be valid in test");

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
        node.free();
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
        parent1.free();
        parent2.free();
    })
}

/// Test that a reparent does not re-seed the registry-initialized Transform from the node,
/// clobbering a value a system authored. Uses `auto_sync: false` so the ECS value never
/// propagates to the node and stays observably distinct.
#[itest(async)]
fn test_reparent_preserves_registry_transform(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        use godot_bevy::bevy_math::Vec3;
        use godot_bevy::bevy_transform::components::Transform;

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotTransformSyncPlugin {
                auto_sync: false,
                ..Default::default()
            });
        })
        .await;

        let mut parent1 = Node::new_alloc();
        parent1.set_name("TransformParent1");
        let mut parent2 = Node::new_alloc();
        parent2.set_name("TransformParent2");
        ctx_clone.scene_tree.clone().add_child(&parent1);
        ctx_clone.scene_tree.clone().add_child(&parent2);

        let mut child = godot::classes::Node2D::new_alloc();
        child.set_name("TransformChild");
        parent1
            .clone()
            .add_child(&child.clone().upcast::<godot::classes::Node>());

        // Wait for the entity to be created and its Transform seeded from the node.
        app.updates(2).await;

        let entity = app
            .entity_for_node(child.instance_id())
            .expect("Child entity should exist");

        // A system authors a sentinel translation the node does not carry.
        app.with_world_mut(|world| {
            let mut transform = world
                .get_mut::<Transform>(entity)
                .expect("registry should seed a Transform for the Node2D");
            transform.translation = Vec3::new(999.0, 999.0, 0.0);
        });

        child
            .clone()
            .upcast::<godot::classes::Node>()
            .reparent(&parent2);
        app.updates(2).await;

        let translation =
            app.with_world(|world| world.get::<Transform>(entity).map(|t| t.translation));
        assert_eq!(
            translation,
            Some(Vec3::new(999.0, 999.0, 0.0)),
            "reparent must not re-seed Transform from the node (would reset to origin)"
        );

        app.cleanup().await;
        parent1.free();
        parent2.free();
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
        parent.free();
    })
}

/// Test that NodeEntityIndex is populated when nodes are added
#[itest(async)]
fn test_node_entity_index_populated_on_add(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        let (node, entity) = app
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
        node.free();
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

/// A packed-scene spawn (handle attached outside the scene-tree plugin) must
/// reconcile to its existing entity on NodeAdded, never spawn a duplicate — the
/// invariant the naive "route lookups through NodeEntityIndex" change broke.
#[itest(async)]
fn test_packed_scene_spawn_reconciles_to_single_entity(
    ctx: &TestContext,
) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        // GodotCorePlugins (what the test app uses) does not include assets, so
        // add the packed scene plugin and its asset dependency explicitly.
        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotAssetsPlugin);
            app.add_plugins(GodotPackedScenePlugin);
        })
        .await;

        // spawn_scene (PostUpdate) instantiates the node, add_child's it, and
        // attaches a GodotNodeHandle to THIS entity; the resulting node_added
        // signal yields a NodeAdded message processed next First.
        let scene_entity = app.with_world_mut(|world| {
            world
                .spawn(GodotScene::from_path("res://test_spawn_scene.tscn"))
                .id()
        });

        // Enough frames for spawn -> add_child -> node_added -> reconciliation.
        app.updates(5).await;

        let handle = app
            .with_world_mut(|world| world.get::<GodotNodeHandle>(scene_entity).copied())
            .expect("spawned scene entity should have a GodotNodeHandle");

        let count = app.with_world_mut(|world| {
            let mut q = world.query::<&GodotNodeHandle>();
            q.iter(world)
                .filter(|h| h.instance_id() == handle.instance_id())
                .count()
        });

        assert_eq!(
            count, 1,
            "expected exactly one entity for the spawned node, found {count} (duplicate = reconciliation broke)"
        );

        println!("✓ packed-scene spawn reconciles to a single entity");

        // Free the spawned node so it does not leak into later tests.
        if let Ok(mut node) = Gd::<godot::classes::Node>::try_from_instance_id(handle.instance_id())
        {
            node.queue_free();
        }
        app.updates(2).await;
        app.cleanup().await;
    })
}
