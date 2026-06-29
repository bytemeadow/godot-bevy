use bevy::prelude::*;
use godot::prelude::*;
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
#[bevy(base = Node2D, class_name = AutoSyncPlayerNode)]
#[bevy(require(TestGrounded), require(speed: TestSpeed, as = f32, default = 250.0))]
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
