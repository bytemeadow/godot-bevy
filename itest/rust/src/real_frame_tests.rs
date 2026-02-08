/*
 * Real frame-driven integration tests
 * These tests verify actual Godot frame progression
 */

use bevy::prelude::*;
use godot_bevy_test::prelude::*;

/// Test that Update systems run on real Godot frames
#[itest(async)]
fn test_update_runs_on_real_frames(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let counter = Counter::new();
        let c = counter.clone();

        let mut app = TestApp::new(&ctx_clone, move |app| {
            #[derive(Resource)]
            struct FrameCounter(Counter);

            app.insert_resource(FrameCounter(c.clone()));
            app.add_systems(Update, |c: Res<FrameCounter>| c.0.increment());
        })
        .await;

        let start = counter.get();
        app.updates(5).await;
        let end = counter.get();

        assert!(
            end >= start + 4,
            "Expected 4+ increments, got {start} -> {end}"
        );

        app.cleanup().await;
    })
}

/// Test PhysicsUpdate runs on physics frames
#[itest(async)]
fn test_physics_update_runs(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let counter = Counter::new();
        let c = counter.clone();

        let mut app = TestApp::new(&ctx_clone, move |app| {
            #[derive(Resource)]
            struct PhysicsCounter(Counter);

            app.insert_resource(PhysicsCounter(c.clone()));
            app.add_systems(
                godot_bevy::prelude::PhysicsUpdate,
                |c: Res<PhysicsCounter>| {
                    c.0.increment();
                },
            );
        })
        .await;

        let start = counter.get();
        app.updates(10).await;
        let end = counter.get();

        assert!(
            end > start,
            "PhysicsUpdate should run, got {start} -> {end}"
        );

        app.cleanup().await;
    })
}

/// Test frame pacing is controlled by Godot
#[itest(async)]
fn test_frame_pacing_controlled_by_godot(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let counter = Counter::new();
        let c = counter.clone();

        let mut app = TestApp::new(&ctx_clone, move |app| {
            #[derive(Resource)]
            struct UpdateCounter(Counter);

            app.insert_resource(UpdateCounter(c.clone()));
            app.add_systems(Update, |c: Res<UpdateCounter>| c.0.increment());
        })
        .await;

        app.update().await;
        let c1 = counter.get();

        app.update().await;
        let c2 = counter.get();

        app.update().await;
        let c3 = counter.get();

        assert!(c2 > c1 && c3 > c2, "Each frame should increment");

        app.cleanup().await;
    })
}
