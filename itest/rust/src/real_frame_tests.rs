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

/// Test FixedUpdate runs on physics frames
#[itest(async)]
fn test_fixed_update_runs_each_frame(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let counter = Counter::new();
        let c = counter.clone();

        let mut app = TestApp::new(&ctx_clone, move |app| {
            #[derive(Resource)]
            struct PhysicsCounter(Counter);

            app.insert_resource(PhysicsCounter(c.clone()));
            app.add_systems(FixedUpdate, |c: Res<PhysicsCounter>| {
                c.0.increment();
            });
        })
        .await;

        let start = counter.get();
        app.updates(10).await;
        let end = counter.get();

        assert!(end > start, "FixedUpdate should run, got {start} -> {end}");

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

/// FixedUpdate now ticks on Godot's physics clock: each physics tick runs
/// FixedMain exactly once, and Res<Time> inside it is positive.
///
/// IMPORTANT: never `assert!` inside a Bevy system — that panics into Godot's
/// `_physics_process` callback, where `BevyApp::physics_process`'s catch_unwind
/// swallows it and the async runner (which only checks `has_godot_task_panicked`)
/// never sees the failure. Record observations into a resource and assert in the
/// task body after `.await`, like every other itest.
#[itest(async)]
fn test_fixed_update_runs_on_physics_tick(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        #[derive(Resource)]
        struct FixedProbe {
            counter: Counter,
            last_delta: f32,
        }

        let counter = Counter::new();
        let c = counter.clone();

        let mut app = TestApp::new(&ctx_clone, move |app| {
            app.insert_resource(FixedProbe {
                counter: c.clone(),
                last_delta: 0.0,
            });
            app.add_systems(
                FixedUpdate,
                |time: Res<Time>, mut probe: ResMut<FixedProbe>| {
                    probe.counter.increment();
                    probe.last_delta = time.delta_secs();
                },
            );
        })
        .await;

        let start = counter.get();
        app.physics_update().await;
        let end = counter.get();
        assert!(
            end > start,
            "FixedUpdate should run on a physics tick, got {start} -> {end}"
        );

        let delta = app.with_world(|w| w.resource::<FixedProbe>().last_delta);
        assert!(
            delta > 0.0,
            "Res<Time> in FixedUpdate must be Godot's physics delta, got {delta}"
        );

        app.cleanup().await;
    })
}

/// Time<Virtual> must still advance in Update after neutralizing the in-process
/// fixed loop (guards against over-aggressive neutralization starving time_system).
#[itest(async)]
fn test_virtual_time_advances_in_update(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct MaxDelta(f32);

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.init_resource::<MaxDelta>();
            app.add_systems(Update, |time: Res<Time>, mut m: ResMut<MaxDelta>| {
                m.0 = m.0.max(time.delta_secs());
            });
        })
        .await;

        app.updates(5).await;
        let max = app.with_world(|w| w.resource::<MaxDelta>().0);
        assert!(
            max > 0.0,
            "Time<Virtual> delta should be positive in Update, got {max}"
        );

        app.cleanup().await;
    })
}
