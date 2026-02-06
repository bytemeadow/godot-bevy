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
        await_frames(1).await;

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
        await_frames(5).await;
        let end = counter.get();

        assert!(
            end >= start + 4,
            "Expected 4+ increments, got {start} -> {end}"
        );

        app.cleanup();
        await_frames(1).await;
    })
}

/// Test that entities persist across frames
#[itest(async)]
fn test_entity_persists_across_frames(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        await_frames(1).await;

        let counter = Counter::new();
        let c = counter.clone();

        let mut app = TestApp::new(&ctx_clone, move |app| {
            #[derive(Component)]
            struct Persistent;

            #[derive(Resource)]
            struct Tracker(Counter);

            app.insert_resource(Tracker(c.clone()));
            app.add_systems(
                Update,
                (
                    |mut cmd: Commands, q: Query<(), With<Persistent>>| {
                        if q.is_empty() {
                            cmd.spawn(Persistent);
                        }
                    },
                    |q: Query<(), With<Persistent>>, t: Res<Tracker>| {
                        if !q.is_empty() {
                            t.0.increment();
                        }
                    },
                )
                    .chain(),
            );
        })
        .await;

        await_frames(10).await;
        let count = counter.get();

        assert!(count >= 8, "Entity should persist 8+ frames, got {count}");

        app.cleanup();
        await_frames(1).await;
    })
}

/// Test PhysicsUpdate runs on physics frames
#[itest(async)]
fn test_physics_update_runs(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        await_frames(1).await;

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
        await_frames(10).await;
        let end = counter.get();

        assert!(
            end > start,
            "PhysicsUpdate should run, got {start} -> {end}"
        );

        app.cleanup();
        await_frames(1).await;
    })
}

/// Test frame pacing is controlled by Godot
#[itest(async)]
fn test_frame_pacing_controlled_by_godot(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        await_frames(1).await;

        let counter = Counter::new();
        let c = counter.clone();

        let mut app = TestApp::new(&ctx_clone, move |app| {
            #[derive(Resource)]
            struct UpdateCounter(Counter);

            app.insert_resource(UpdateCounter(c.clone()));
            app.add_systems(Update, |c: Res<UpdateCounter>| c.0.increment());
        })
        .await;

        await_frame().await;
        let c1 = counter.get();

        await_frame().await;
        let c2 = counter.get();

        await_frame().await;
        let c3 = counter.get();

        assert!(c2 > c1 && c3 > c2, "Each frame should increment");

        app.cleanup();
        await_frames(1).await;
    })
}
