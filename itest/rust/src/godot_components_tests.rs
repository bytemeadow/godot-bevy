/*
 * Integration tests for the `#[godot_components]` feature: scene-spawned nodes
 * populate companion components from exports, and Bevy-side spawns get the
 * declared required-component defaults.
 *
 * The BevyBundle autosync match/miss behavior is covered by autosync_match_tests.
 */

use bevy::prelude::*;
use godot::obj::NewAlloc;
use godot::prelude::*;
use godot_bevy::prelude::GodotNode;
use godot_bevy_test::prelude::*;

#[derive(Component, Debug, PartialEq, Clone)]
pub struct TestSpeed(f32);

impl Default for TestSpeed {
    // Distinct from the exported default below on purpose: lets the Bevy-side
    // spawn test prove it uses the declared default(250.0), not this impl.
    fn default() -> Self {
        Self(1.0)
    }
}

#[derive(Component, Debug, Default, PartialEq, Clone)]
pub struct TestGrounded;

#[derive(Component, GodotNode, Debug, Default)]
#[godot_node(base(Node2D), class_name(AutoSyncPlayerNode))]
#[godot_components(
    (TestGrounded),
    speed(TestSpeed, export_type(f32), default(250.0)),
)]
pub struct AutoSyncPlayer;

/// A scene-spawned node should populate the primary component plus its
/// companions, with the newtype companion carrying the editor-set export value.
#[itest(async)]
fn test_godot_components_from_scene(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        let mut node = AutoSyncPlayerNode::new_alloc();
        node.set_name("AutoSyncPlayer");
        // Editor-set value via the exported property, before entering the tree.
        // "speed" must match the prop name declared in #[godot_components(speed(..))].
        node.set("speed", &99.0f32.to_variant());
        ctx_clone
            .scene_tree
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
        let entity = entity.expect("Entity should exist for AutoSyncPlayerNode");

        app.with_world(|world| {
            assert!(
                world.get::<AutoSyncPlayer>(entity).is_some(),
                "Primary component inserted"
            );
            assert!(
                world.get::<TestGrounded>(entity).is_some(),
                "Marker companion inserted"
            );
            assert_eq!(
                world.get::<TestSpeed>(entity),
                Some(&TestSpeed(99.0)),
                "Companion carries exported value"
            );
        });

        app.cleanup().await;
        node.queue_free();
    })
}

/// A pure Bevy `world.spawn(Primary)` should pull in the companions via the
/// required-components path, using the declared default(..) values rather than
/// the companion's own Default impl.
#[itest(async)]
fn test_godot_components_bevy_spawn_defaults(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        // No update() needed: required components are applied synchronously by
        // Bevy's ECS at spawn time, not across a Godot frame.
        let entity = app.with_world_mut(|world| world.spawn(AutoSyncPlayer).id());

        app.with_world(|world| {
            assert!(
                world.get::<TestGrounded>(entity).is_some(),
                "Marker companion required"
            );
            assert_eq!(
                world.get::<TestSpeed>(entity),
                Some(&TestSpeed(250.0)),
                "Declared default, not TestSpeed::default()"
            );
        });

        app.cleanup().await;
    })
}
