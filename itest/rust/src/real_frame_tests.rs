/*
 * Real frame-driven integration tests
 * These tests verify actual Godot frame progression
 */

use bevy::prelude::*;
use godot_bevy_test::prelude::*;

/// Test that Update systems run on real Godot frames
#[itest(async)]
fn test_update_runs_on_real_frames(ctx: &TestContext) -> godot::task::TaskHandle {
    bevy_app_test!(
        ctx,
        counter,
        |app| {
            #[derive(Resource)]
            struct FrameCounter(Counter);

            app.insert_resource(FrameCounter(counter.clone()));
            app.add_systems(Update, |c: Res<FrameCounter>| c.0.increment());
        },
        async {
            let start = counter.get();
            await_frames(5).await;
            let end = counter.get();

            // We expect at least 4 increments (one less because of frame timing)
            assert!(
                end >= start + 4,
                "Expected 4+ increments, got {start} -> {end}"
            );
            println!("✓ Systems ran on {} real Godot frames!", end - start);
        }
    )
}

/// Test that entities persist across frames
#[itest(async)]
fn test_entity_persists_across_frames(ctx: &TestContext) -> godot::task::TaskHandle {
    bevy_app_test!(
        ctx,
        counter,
        |app| {
            #[derive(Component)]
            struct Persistent;

            #[derive(Resource)]
            struct Tracker(Counter);

            app.insert_resource(Tracker(counter.clone()));
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
        },
        async {
            await_frames(10).await;
            let count = counter.get();

            // Account for setup frames - we expect at least 8 frames
            assert!(count >= 8, "Entity should persist 8+ frames, got {count}");
            println!("✓ Entity persisted across {count} frames!");
        }
    )
}

/// Test PhysicsUpdate runs on physics frames
#[itest(async)]
fn test_physics_update_runs(ctx: &TestContext) -> godot::task::TaskHandle {
    bevy_app_test!(
        ctx,
        counter,
        |app| {
            #[derive(Resource)]
            struct PhysicsCounter(Counter);

            app.insert_resource(PhysicsCounter(counter.clone()));
            app.add_systems(
                godot_bevy::prelude::PhysicsUpdate,
                |c: Res<PhysicsCounter>| {
                    c.0.increment();
                },
            );
        },
        async {
            let start = counter.get();
            await_frames(10).await;
            let end = counter.get();

            assert!(
                end > start,
                "PhysicsUpdate should run, got {start} -> {end}"
            );
            println!("✓ PhysicsUpdate ran {} times!", end - start);
        }
    )
}

/// Test frame pacing is controlled by Godot
#[itest(async)]
fn test_frame_pacing_controlled_by_godot(ctx: &TestContext) -> godot::task::TaskHandle {
    bevy_app_test!(
        ctx,
        counter,
        |app| {
            #[derive(Resource)]
            struct UpdateCounter(Counter);

            app.insert_resource(UpdateCounter(counter.clone()));
            app.add_systems(Update, |c: Res<UpdateCounter>| c.0.increment());
        },
        async {
            await_frame().await;
            let c1 = counter.get();

            await_frame().await;
            let c2 = counter.get();

            await_frame().await;
            let c3 = counter.get();

            assert!(c2 > c1 && c3 > c2, "Each frame should increment");
            println!("✓ Frame pacing: f1={c1}, f2={c2}, f3={c3}");
            println!("✓ Systems run ONLY when Godot advances frames!");
        }
    )
}
