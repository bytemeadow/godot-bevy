use bevy::prelude::*;
use godot::prelude::*;
use godot_bevy::interop::{CanvasItemMarker, Node2DMarker, NodeMarker};
use godot_bevy::prelude::*;
use godot_bevy_test::prelude::*;

#[derive(Component, PartialEq, Debug, Clone)]
struct TestSpeed(f32);
impl Default for TestSpeed {
    fn default() -> Self {
        TestSpeed(1.0)
    }
}

#[derive(Component, Default)]
struct TestGrounded;

#[derive(Component, GodotNode, Default)]
#[gdbevy(base = Node2D, class_name = AutoSyncPlayerNode)]
#[gdbevy(require(TestGrounded), require(speed: TestSpeed, as = f32, default = 250.0))]
struct AutoSyncPlayer;

#[itest(async)]
fn scene_spawn_carries_export_value(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx, |_app| {}).await;
        let mut node = AutoSyncPlayerNode::new_alloc();
        node.set("speed", &99.0f32.to_variant()); // set BEFORE tree-add
        let as_node = node.clone().upcast::<godot::classes::Node>();
        app.ctx().scene_tree.clone().add_child(&as_node);
        let mut entity = None;
        for _ in 0..3 {
            app.update().await;
            if let Some(e) = app.entity_for_node(node.instance_id()) {
                entity = Some(e);
                break;
            }
        }
        let entity = entity.expect("entity for AutoSyncPlayerNode");
        assert_eq!(
            app.with_world(|w| w.get::<TestSpeed>(entity).cloned()),
            Some(TestSpeed(99.0))
        );
        assert!(app.with_world(|w| w.get::<TestGrounded>(entity).is_some()));
        app.cleanup().await;
        node.free();
    })
}

/// A Godot reparent must not re-run the autosync bundle creator: a value a system
/// authored after spawn survives the move instead of being reset to the node's `#[export]`.
#[itest(async)]
fn reparent_preserves_autosync_component(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx, |_app| {}).await;

        let mut parent1 = Node::new_alloc();
        parent1.set_name("AutoSyncParent1");
        let mut parent2 = Node::new_alloc();
        parent2.set_name("AutoSyncParent2");
        app.ctx().scene_tree.clone().add_child(&parent1);
        app.ctx().scene_tree.clone().add_child(&parent2);

        let mut node = AutoSyncPlayerNode::new_alloc();
        node.set("speed", &99.0f32.to_variant()); // authored export value
        parent1
            .clone()
            .add_child(&node.clone().upcast::<godot::classes::Node>());

        let mut entity = None;
        for _ in 0..3 {
            app.update().await;
            if let Some(e) = app.entity_for_node(node.instance_id()) {
                entity = Some(e);
                break;
            }
        }
        let entity = entity.expect("entity for AutoSyncPlayerNode");
        assert_eq!(
            app.with_world(|w| w.get::<TestSpeed>(entity).cloned()),
            Some(TestSpeed(99.0)),
            "spawn should seed TestSpeed from the exported value"
        );

        app.with_world_mut(|w| {
            w.entity_mut(entity).insert(TestSpeed(7.0));
        });

        node.clone()
            .upcast::<godot::classes::Node>()
            .reparent(&parent2);
        app.updates(2).await;

        assert_eq!(
            app.with_world(|w| w.get::<TestSpeed>(entity).cloned()),
            Some(TestSpeed(7.0)),
            "reparent must not re-read the export (would clobber back to 99.0)"
        );

        app.cleanup().await;
        parent1.free();
        parent2.free();
    })
}

/// Node-type markers must cover a node's whole native ancestry in one shot. A plain
/// Node2D takes the native-leaf path; a GDExtension node (leaf class "AutoSyncPlayerNode",
/// not a Godot class) must still get the Node2D ancestor markers -- the decoration skips
/// the unknown leaf and matches its first native ancestor.
#[itest(async)]
fn node_type_markers_cover_native_ancestors(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx, |_app| {}).await;

        let custom = AutoSyncPlayerNode::new_alloc();
        app.ctx()
            .scene_tree
            .clone()
            .add_child(&custom.clone().upcast::<godot::classes::Node>());

        let plain = godot::classes::Node2D::new_alloc();
        app.ctx()
            .scene_tree
            .clone()
            .add_child(&plain.clone().upcast::<godot::classes::Node>());

        let mut custom_entity = None;
        let mut plain_entity = None;
        for _ in 0..4 {
            app.update().await;
            custom_entity = custom_entity.or_else(|| app.entity_for_node(custom.instance_id()));
            plain_entity = plain_entity.or_else(|| app.entity_for_node(plain.instance_id()));
            if custom_entity.is_some() && plain_entity.is_some() {
                break;
            }
        }
        let custom_entity = custom_entity.expect("entity for AutoSyncPlayerNode");
        let plain_entity = plain_entity.expect("entity for Node2D");

        for (label, entity) in [
            ("AutoSyncPlayerNode", custom_entity),
            ("Node2D", plain_entity),
        ] {
            assert!(
                app.with_world(|w| w.get::<Node2DMarker>(entity).is_some()),
                "{label} missing Node2DMarker"
            );
            assert!(
                app.with_world(|w| w.get::<CanvasItemMarker>(entity).is_some()),
                "{label} missing CanvasItemMarker"
            );
            assert!(
                app.with_world(|w| w.get::<NodeMarker>(entity).is_some()),
                "{label} missing NodeMarker"
            );
        }

        app.cleanup().await;
        custom.free();
        plain.free();
    })
}

#[itest(async)]
fn bevy_spawn_gets_declared_default(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx, |_app| {}).await;
        let e = app.with_world_mut(|w| w.spawn(AutoSyncPlayer).id());
        assert_eq!(
            app.with_world(|w| w.get::<TestSpeed>(e).cloned()),
            Some(TestSpeed(250.0))
        );
        assert!(app.with_world(|w| w.get::<TestGrounded>(e).is_some()));
        app.cleanup().await;
    })
}
