use bevy::prelude::{Component, Entity, Name};
use godot::obj::{InstanceId, NewAlloc};
use godot::prelude::*;
use godot_bevy::plugins::scene_tree::ProtectedNodeEntity;
use godot_bevy::prelude::*;
use godot_bevy_test::prelude::*;

#[derive(Component, Clone, Copy, Debug, PartialEq)]
struct SceneTreePayload(i32);

async fn settle_scene_tree(app: &TestApp) {
    for _ in 0..4 {
        app.physics_update().await;
    }
}

fn assert_node_alive(id: InstanceId) {
    let node = Gd::<Node>::try_from_instance_id(id).expect("node must remain valid");
    assert!(
        !node.is_queued_for_deletion(),
        "node must not be queued for deletion"
    );
}

fn assert_unmirrored(app: &TestApp, id: InstanceId, entity: Entity, protected: bool) {
    assert_eq!(
        app.entity_for_node(id),
        None,
        "departed node must leave the index"
    );
    app.with_world(|world| {
        if protected {
            assert_eq!(
                world.get::<SceneTreePayload>(entity),
                Some(&SceneTreePayload(42))
            );
            assert!(world.get::<ProtectedNodeEntity>(entity).is_some());
            assert!(world.get::<GodotNodeHandle>(entity).is_none());
            assert!(world.get::<GodotScene>(entity).is_none());
            assert!(world.get::<Name>(entity).is_none());
            assert!(world.get::<Groups>(entity).is_none());
            assert!(world.get::<GodotChildOf>(entity).is_none());
            assert!(world.get::<GodotChildren>(entity).is_none());
        } else {
            assert!(
                world.get_entity(entity).is_err(),
                "ordinary mirror entity must despawn"
            );
        }
    });
}

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

#[itest(async)]
fn test_reparent_into_excluded_tears_down_entity(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;
        let (mut node, entity) = app.add_node::<Node>("Reparented").await;
        let node_id = node.instance_id();
        let child = Node::new_alloc();
        let child_id = child.instance_id();
        node.add_child(&child);
        let mut excluded_parent = Node::new_alloc();
        excluded_parent.set_meta("_bevy_exclude", &true.to_variant());
        ctx_clone.scene_tree.clone().add_child(&excluded_parent);
        settle_scene_tree(&app).await;
        let child_entity = app.entity_for_node(child_id).expect("child entity");

        node.reparent(&excluded_parent);
        settle_scene_tree(&app).await;

        for (id, entity) in [(node_id, entity), (child_id, child_entity)] {
            assert_node_alive(id);
            assert_unmirrored(&app, id, entity, false);
        }
        assert_eq!(node.get_parent(), Some(excluded_parent.clone()));
        assert_eq!(child.get_parent(), Some(node));

        app.cleanup().await;
        excluded_parent.free();
    })
}

/// Reparenting a mirrored node out to the scene root drops its `GodotChildOf`, so the
/// ECS hierarchy stops reflecting the old parent instead of stranding a stale edge.
#[itest(async)]
fn test_reparent_to_root_clears_godot_child_of(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        let (parent, _pe) = app.add_node::<godot::classes::Node>("RtrParent").await;
        let child = Node::new_alloc();
        let child_id = child.instance_id();
        parent.clone().add_child(&child);

        let mut entity = None;
        for _ in 0..4 {
            app.update().await;
            if let Some(e) = app.entity_for_node(child_id) {
                entity = Some(e);
                break;
            }
        }
        let entity = entity.expect("entity for child");
        assert!(
            app.with_world(|w| w.get::<GodotChildOf>(entity).is_some()),
            "child under a mirrored parent should have GodotChildOf"
        );

        let root = ctx_clone.scene_tree.get_tree().get_root().unwrap();
        child
            .clone()
            .reparent(&root.upcast::<godot::classes::Node>());
        app.updates(3).await;

        assert!(
            app.with_world(|w| w.get::<GodotChildOf>(entity).is_none()),
            "GodotChildOf must be cleared after reparenting to the scene root"
        );

        app.cleanup().await;
        child.free();
        parent.free();
    })
}

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

#[itest(async)]
fn test_node_renamed_event(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        let (mut node, _entity) = app.add_node::<godot::classes::Node2D>("OriginalName").await;

        let node_id = node.instance_id();

        node.set_name("RenamedNode");
        app.updates(2).await;

        assert!(
            app.has_entity_for_node(node_id),
            "Entity should still exist after rename"
        );

        app.cleanup().await;
        node.free();
    })
}

#[itest(async)]
fn test_protected_node_entity(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        for queued in [false, true] {
            let mut app = TestApp::new(&ctx_clone, |_app| {}).await;
            let (mut node, entity) = app.add_node::<Node>("ProtectedNode").await;
            let node_id = node.instance_id();
            app.with_world_mut(|world| {
                world
                    .entity_mut(entity)
                    .insert((ProtectedNodeEntity, SceneTreePayload(42)));
            });

            if queued {
                node.queue_free();
                assert!(node.is_queued_for_deletion());
            } else {
                node.free();
                assert!(!node_id.lookup_validity());
            }
            settle_scene_tree(&app).await;

            assert!(!node_id.lookup_validity());
            assert_eq!(app.entity_for_node(node_id), None);
            app.with_world(|world| {
                assert_eq!(
                    world.get::<SceneTreePayload>(entity),
                    Some(&SceneTreePayload(42))
                );
                assert!(world.get::<ProtectedNodeEntity>(entity).is_some());
                assert!(world.get::<GodotNodeHandle>(entity).is_none());
            });
            app.cleanup().await;
        }
    })
}

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

#[itest(async)]
fn test_node_reparenting_preserves_entity(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        for protected in [false, true] {
            for remove_add in [false, true] {
                let mut app = TestApp::new(&ctx_clone, |_app| {}).await;
                let (mut parent1, _) = app.add_node::<Node>("Parent1").await;
                let (mut parent2, parent2_entity) = app.add_node::<Node>("Parent2").await;
                let mut child = Node::new_alloc();
                let child_id = child.instance_id();
                parent1.add_child(&child);
                settle_scene_tree(&app).await;
                let entity = app.entity_for_node(child_id).expect("child entity");
                app.with_world_mut(|world| {
                    world.entity_mut(entity).insert(SceneTreePayload(42));
                    if protected {
                        world.entity_mut(entity).insert(ProtectedNodeEntity);
                    }
                });

                if remove_add {
                    parent1.remove_child(&child);
                    parent2.add_child(&child);
                } else {
                    child.reparent(&parent2);
                }
                settle_scene_tree(&app).await;

                assert_node_alive(child_id);
                assert_eq!(
                    app.entity_for_node(child_id),
                    Some(entity),
                    "Entity should still exist after reparenting"
                );
                app.with_world(|world| {
                    assert_eq!(
                        world.get::<SceneTreePayload>(entity),
                        Some(&SceneTreePayload(42))
                    );
                    assert_eq!(
                        world.get::<GodotChildOf>(entity).map(GodotChildOf::get),
                        Some(parent2_entity)
                    );
                    assert_eq!(
                        world.get::<GodotNodeHandle>(entity).unwrap().instance_id(),
                        child_id
                    );
                });

                app.cleanup().await;
                parent1.free();
                parent2.free();
            }
        }
    })
}

/// A reparent must not re-seed the registry-initialized Transform from the node,
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

        app.updates(2).await;

        let entity = app
            .entity_for_node(child.instance_id())
            .expect("Child entity should exist");

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

#[itest(async)]
fn test_remove_child_despawns_entity(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;
        let (mut parent, _) = app.add_node::<Node>("RemoveChildParent").await;
        let child = Node::new_alloc();
        let child_id = child.instance_id();
        parent.add_child(&child);
        settle_scene_tree(&app).await;
        let entity = app.entity_for_node(child_id).expect("child entity");

        parent.remove_child(&child);
        settle_scene_tree(&app).await;

        assert_node_alive(child_id);
        assert!(
            app.with_world(|world| world.get_entity(entity).is_err()),
            "Entity should be despawned after remove_child()"
        );
        assert_unmirrored(&app, child_id, entity, false);
        assert!(!child.is_inside_tree());

        app.cleanup().await;
        child.free();
        parent.free();
    })
}

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

#[itest(async)]
fn test_node_entity_index_updated_on_remove(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;
        let (mut node, entity) = app.add_node::<Node>("IndexRemovalTestNode").await;
        let node_id = node.instance_id();
        assert_eq!(app.entity_for_node(node_id), Some(entity));

        node.queue_free();
        assert!(node.is_queued_for_deletion());
        settle_scene_tree(&app).await;

        assert!(!node_id.lookup_validity());
        assert_unmirrored(&app, node_id, entity, false);
        app.cleanup().await;
    })
}

/// A packed-scene spawn (handle attached outside the scene-tree plugin) must
/// reconcile to its existing entity on NodeAdded rather than spawning a duplicate.
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

#[itest(async)]
fn test_protected_detach_preserves_node(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;
        let (mut parent, parent_entity) = app.add_node::<Node>("ProtectedDetachParent").await;
        let child = Node::new_alloc();
        let child_id = child.instance_id();
        parent.add_child(&child);
        settle_scene_tree(&app).await;
        let entity = app.entity_for_node(child_id).expect("child entity");
        app.with_world_mut(|world| {
            assert_eq!(
                world.get::<GodotChildOf>(entity).map(GodotChildOf::get),
                Some(parent_entity)
            );
            world
                .entity_mut(entity)
                .insert((ProtectedNodeEntity, SceneTreePayload(42)));
        });

        parent.remove_child(&child);
        settle_scene_tree(&app).await;

        assert_node_alive(child_id);
        assert_unmirrored(&app, child_id, entity, true);
        app.with_world(|world| {
            assert!(
                !world
                    .get::<GodotChildren>(parent_entity)
                    .is_some_and(|children| children.contains(entity))
            );
        });
        app.cleanup().await;
        child.free();
        parent.free();
    })
}

#[itest(async)]
fn test_detached_subtree_cleanup(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        for auto_despawn_children in [true, false] {
            for protected_root in [false, true] {
                for protected_child in [false, true] {
                    let mut app = TestApp::new(&ctx_clone, move |app| {
                        app.world_mut()
                            .resource_mut::<SceneTreeConfig>()
                            .auto_despawn_children = auto_despawn_children;
                    })
                    .await;
                    let (mut parent, _) = app.add_node::<Node>("SubtreeParent").await;
                    let mut node = Node::new_alloc();
                    let mut child = Node::new_alloc();
                    let grandchild = Node::new_alloc();
                    child.add_child(&grandchild);
                    node.add_child(&child);
                    parent.add_child(&node);
                    settle_scene_tree(&app).await;
                    let members = [
                        (&node, protected_root),
                        (&child, protected_child),
                        (&grandchild, !protected_child),
                    ]
                    .map(|(node, protected)| {
                        let id = node.instance_id();
                        let entity = app.entity_for_node(id).expect("subtree entity");
                        app.with_world_mut(|world| {
                            world.entity_mut(entity).insert(SceneTreePayload(42));
                            if protected {
                                world.entity_mut(entity).insert(ProtectedNodeEntity);
                            }
                        });
                        (id, entity, protected)
                    });

                    parent.remove_child(&node);
                    settle_scene_tree(&app).await;

                    for (id, entity, protected) in members {
                        assert_node_alive(id);
                        assert_unmirrored(&app, id, entity, protected);
                    }
                    assert!(!node.is_inside_tree());
                    assert_eq!(child.get_parent(), Some(node.clone()));
                    assert_eq!(grandchild.get_parent(), Some(child));

                    app.cleanup().await;
                    node.free();
                    parent.free();
                }
            }
        }
    })
}

#[itest(async)]
fn test_reparent_to_detached_parent(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;
        let (mut node, entity) = app.add_node::<Node>("OfflineReparent").await;
        let node_id = node.instance_id();
        let offline_parent = Node::new_alloc();

        node.reparent(&offline_parent);
        settle_scene_tree(&app).await;

        assert_node_alive(node_id);
        assert_unmirrored(&app, node_id, entity, false);
        assert!(!node.is_inside_tree());
        assert_eq!(node.get_parent(), Some(offline_parent.clone()));

        app.cleanup().await;
        offline_parent.free();
    })
}

#[itest(async)]
fn test_protected_reparent_into_excluded(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;
        let (mut node, entity) = app.add_node::<Node>("ProtectedExcluded").await;
        let node_id = node.instance_id();
        let child = Node::new_alloc();
        let child_id = child.instance_id();
        node.add_child(&child);
        let mut excluded_parent = Node::new_alloc();
        excluded_parent.set_meta("_bevy_exclude", &true.to_variant());
        ctx_clone.scene_tree.clone().add_child(&excluded_parent);
        settle_scene_tree(&app).await;
        let child_entity = app.entity_for_node(child_id).expect("child entity");
        app.with_world_mut(|world| {
            for entity in [entity, child_entity] {
                world
                    .entity_mut(entity)
                    .insert((ProtectedNodeEntity, SceneTreePayload(42)));
            }
        });

        node.reparent(&excluded_parent);
        settle_scene_tree(&app).await;

        for (id, entity) in [(node_id, entity), (child_id, child_entity)] {
            assert_node_alive(id);
            assert_unmirrored(&app, id, entity, true);
        }
        assert_eq!(node.get_parent(), Some(excluded_parent.clone()));
        assert_eq!(child.get_parent(), Some(node));

        app.cleanup().await;
        excluded_parent.free();
    })
}

#[itest(async)]
fn test_reparent_out_of_excluded_creates_entities(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;
        let mut excluded_parent = Node::new_alloc();
        excluded_parent.set_meta("_bevy_exclude", &true.to_variant());
        let mut node = Node::new_alloc();
        let child = Node::new_alloc();
        let node_id = node.instance_id();
        let child_id = child.instance_id();
        node.add_child(&child);
        excluded_parent.add_child(&node);
        ctx_clone.scene_tree.clone().add_child(&excluded_parent);
        let (destination, destination_entity) = app.add_node::<Node>("IncludedParent").await;
        settle_scene_tree(&app).await;
        assert_eq!(app.entity_for_node(node_id), None);
        assert_eq!(app.entity_for_node(child_id), None);

        node.reparent(&destination);
        settle_scene_tree(&app).await;

        assert_node_alive(node_id);
        assert_node_alive(child_id);
        let entity = app
            .entity_for_node(node_id)
            .expect("node must enter the mirror");
        let child_entity = app
            .entity_for_node(child_id)
            .expect("child must enter the mirror");
        app.with_world(|world| {
            assert_eq!(
                world.get::<GodotNodeHandle>(entity).unwrap().instance_id(),
                node_id
            );
            assert_eq!(
                world
                    .get::<GodotNodeHandle>(child_entity)
                    .unwrap()
                    .instance_id(),
                child_id
            );
            assert_eq!(
                world.get::<GodotChildOf>(entity).map(GodotChildOf::get),
                Some(destination_entity)
            );
            assert_eq!(
                world
                    .get::<GodotChildOf>(child_entity)
                    .map(GodotChildOf::get),
                Some(entity)
            );
        });

        app.cleanup().await;
        excluded_parent.free();
        destination.free();
    })
}

#[itest(async)]
fn test_excluded_detach_preserves_node(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;
        let mut excluded_parent = Node::new_alloc();
        excluded_parent.set_meta("_bevy_exclude", &true.to_variant());
        let mut node = Node::new_alloc();
        let child = Node::new_alloc();
        let ids = [node.instance_id(), child.instance_id()];
        node.add_child(&child);
        excluded_parent.add_child(&node);
        ctx_clone.scene_tree.clone().add_child(&excluded_parent);
        settle_scene_tree(&app).await;
        for id in ids {
            assert_eq!(app.entity_for_node(id), None);
        }

        excluded_parent.remove_child(&node);
        settle_scene_tree(&app).await;

        for id in ids {
            assert_node_alive(id);
            assert_eq!(app.entity_for_node(id), None);
        }
        app.cleanup().await;
        node.free();
        excluded_parent.free();
    })
}

#[itest(async)]
fn test_detached_node_reenters_tree(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        for protected in [false, true] {
            let mut app = TestApp::new(&ctx_clone, |_app| {}).await;
            let (mut parent, _) = app.add_node::<Node>("ReentryParent").await;
            let node = Node::new_alloc();
            let node_id = node.instance_id();
            parent.add_child(&node);
            settle_scene_tree(&app).await;
            let old_entity = app.entity_for_node(node_id).expect("initial entity");
            app.with_world_mut(|world| {
                world.entity_mut(old_entity).insert(SceneTreePayload(42));
                if protected {
                    world.entity_mut(old_entity).insert(ProtectedNodeEntity);
                }
            });

            parent.remove_child(&node);
            settle_scene_tree(&app).await;
            assert_node_alive(node_id);
            assert_unmirrored(&app, node_id, old_entity, protected);

            parent.add_child(&node);
            settle_scene_tree(&app).await;

            assert_node_alive(node_id);
            assert_eq!(node.instance_id(), node_id);
            let new_entity = app.entity_for_node(node_id).expect("reentry entity");
            assert_ne!(new_entity, old_entity);
            app.with_world(|world| {
                assert!(world.get::<SceneTreePayload>(new_entity).is_none());
                assert_eq!(
                    world
                        .get::<GodotNodeHandle>(new_entity)
                        .unwrap()
                        .instance_id(),
                    node_id
                );
                if protected {
                    assert_eq!(
                        world.get::<SceneTreePayload>(old_entity),
                        Some(&SceneTreePayload(42))
                    );
                    assert!(world.get::<GodotNodeHandle>(old_entity).is_none());
                } else {
                    assert!(world.get_entity(old_entity).is_err());
                }
            });
            app.cleanup().await;
            parent.free();
        }
    })
}

#[itest(async)]
fn test_protected_rebind_then_despawn(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        for reattach in [false, true] {
            let mut app = TestApp::new(&ctx_clone, |_app| {}).await;
            let (mut parent, _) = app.add_node::<Node>("RebindParent").await;
            let node = Node::new_alloc();
            let node_id = node.instance_id();
            parent.add_child(&node);
            settle_scene_tree(&app).await;
            let entity = app.entity_for_node(node_id).expect("initial entity");
            app.with_world_mut(|world| {
                world
                    .entity_mut(entity)
                    .insert((ProtectedNodeEntity, SceneTreePayload(42)));
            });

            parent.remove_child(&node);
            settle_scene_tree(&app).await;
            assert_node_alive(node_id);
            assert_unmirrored(&app, node_id, entity, true);
            app.with_world_mut(|world| {
                world
                    .entity_mut(entity)
                    .insert(GodotNodeHandle::from(node_id));
            });
            if reattach {
                parent.add_child(&node);
                settle_scene_tree(&app).await;
            }
            assert_eq!(app.entity_for_node(node_id), Some(entity));
            app.with_world_mut(|world| {
                assert_eq!(
                    world.get::<SceneTreePayload>(entity),
                    Some(&SceneTreePayload(42))
                );
                world.entity_mut(entity).despawn();
            });
            assert!(node.is_queued_for_deletion());
            settle_scene_tree(&app).await;

            assert!(!node_id.lookup_validity());
            assert_unmirrored(&app, node_id, entity, false);
            app.cleanup().await;
            parent.free();
        }
    })
}

#[itest(async)]
fn test_ecs_despawn_frees_node(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        for protected in [false, true] {
            for detach in [false, true] {
                let mut app = TestApp::new(&ctx_clone, |_app| {}).await;
                let (mut parent, _) = app.add_node::<Node>("DespawnParent").await;
                let node = Node::new_alloc();
                let node_id = node.instance_id();
                parent.add_child(&node);
                settle_scene_tree(&app).await;
                let entity = app.entity_for_node(node_id).expect("node entity");
                app.with_world_mut(|world| {
                    if protected {
                        world.entity_mut(entity).insert(ProtectedNodeEntity);
                    }
                });

                if detach {
                    parent.remove_child(&node);
                }
                app.with_world_mut(|world| {
                    world.entity_mut(entity).despawn();
                });
                assert!(node.is_queued_for_deletion());
                settle_scene_tree(&app).await;

                assert!(!node_id.lookup_validity());
                assert_unmirrored(&app, node_id, entity, false);
                app.cleanup().await;
                parent.free();
            }
        }
    })
}

#[itest(async)]
fn test_handle_removal_frees_node(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        for protected in [false, true] {
            for detach in [false, true] {
                let mut app = TestApp::new(&ctx_clone, |_app| {}).await;
                let (mut parent, _) = app.add_node::<Node>("HandleRemovalParent").await;
                let node = Node::new_alloc();
                let node_id = node.instance_id();
                parent.add_child(&node);
                settle_scene_tree(&app).await;
                let entity = app.entity_for_node(node_id).expect("node entity");
                app.with_world_mut(|world| {
                    world.entity_mut(entity).insert(SceneTreePayload(42));
                    if protected {
                        world.entity_mut(entity).insert(ProtectedNodeEntity);
                    }
                });

                if detach {
                    parent.remove_child(&node);
                }
                app.with_world_mut(|world| {
                    world.entity_mut(entity).remove::<GodotNodeHandle>();
                });
                assert!(node.is_queued_for_deletion());
                settle_scene_tree(&app).await;

                assert!(!node_id.lookup_validity());
                assert_eq!(app.entity_for_node(node_id), None);
                app.with_world(|world| {
                    assert_eq!(
                        world.get::<SceneTreePayload>(entity),
                        Some(&SceneTreePayload(42))
                    );
                    assert!(world.get::<GodotNodeHandle>(entity).is_none());
                });
                app.cleanup().await;
                parent.free();
            }
        }
    })
}

#[itest(async)]
fn test_free_cleans_up_entity(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;
        let (node, entity) = app.add_node::<Node>("ImmediateFree").await;
        let node_id = node.instance_id();

        node.free();
        assert!(!node_id.lookup_validity());
        settle_scene_tree(&app).await;

        assert_unmirrored(&app, node_id, entity, false);
        app.cleanup().await;
    })
}

#[itest(async)]
fn test_ecs_parent_despawn_with_auto_children_disabled(
    ctx: &TestContext,
) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        for protected_child in [false, true] {
            let mut app = TestApp::new(&ctx_clone, |app| {
                app.world_mut()
                    .resource_mut::<SceneTreeConfig>()
                    .auto_despawn_children = false;
            })
            .await;
            let (mut parent, parent_entity) = app.add_node::<Node>("NoCascadeParent").await;
            let parent_id = parent.instance_id();
            let child = Node::new_alloc();
            let child_id = child.instance_id();
            parent.add_child(&child);
            settle_scene_tree(&app).await;
            let child_entity = app.entity_for_node(child_id).expect("child entity");
            app.with_world_mut(|world| {
                world.entity_mut(child_entity).insert(SceneTreePayload(42));
                if protected_child {
                    world.entity_mut(child_entity).insert(ProtectedNodeEntity);
                }
                world.entity_mut(parent_entity).despawn();
                assert!(
                    world.get_entity(child_entity).is_ok(),
                    "ECS cascade must be disabled"
                );
            });
            assert!(parent.is_queued_for_deletion());
            settle_scene_tree(&app).await;

            assert!(!parent_id.lookup_validity());
            assert!(!child_id.lookup_validity());
            assert_unmirrored(&app, parent_id, parent_entity, false);
            assert_unmirrored(&app, child_id, child_entity, protected_child);
            app.cleanup().await;
        }
    })
}
