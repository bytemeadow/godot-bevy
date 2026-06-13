/*
 * Autosync class-keyed lookup correctness.
 *
 * Reuses the registered fixtures from the benchmarks module so no additional
 * autosync types are registered (which would skew the benchmark comparison).
 */

use crate::benchmarks::{BenchAutosyncNode0, BenchMarker0};
use godot_bevy_test::prelude::*;

/// A scene-spawned node of a registered type gets its bundle component.
#[itest(async)]
fn test_autosync_matches_registered_type(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        let (mut node, entity) = app.add_node::<BenchAutosyncNode0>("MatchNode").await;

        app.with_world(|world| {
            assert!(
                world.get::<BenchMarker0>(entity).is_some(),
                "registered type should receive its bundle component"
            );
        });

        app.cleanup().await;
        node.queue_free();
    })
}

/// A scene-spawned node of an unregistered type gets no autosync component.
#[itest(async)]
fn test_autosync_skips_unregistered_type(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |_app| {}).await;

        let (mut node, entity) = app.add_node::<godot::classes::Node2D>("MissNode").await;

        app.with_world(|world| {
            assert!(
                world.get::<BenchMarker0>(entity).is_none(),
                "unregistered type must not receive autosync components"
            );
        });

        app.cleanup().await;
        node.queue_free();
    })
}
