/*
 * Collision system integration tests
 *
 * Tests that GodotCollisionsPlugin initializes correctly and that
 * CollisionStarted/CollisionEnded observers fire when triggered.
 */

use bevy::prelude::*;
use godot_bevy::prelude::*;
use godot_bevy_test::prelude::*;

/// Test that GodotCollisionsPlugin initializes and that Collisions is accessible.
#[itest(async)]
fn test_collision_plugin_initializes(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        await_frames(1).await;

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotCollisionsPlugin);
        })
        .await;

        app.update().await;

        let (is_empty, len) = app.with_world_mut(|world| {
            let mut system_state: bevy::ecs::system::SystemState<Collisions> =
                bevy::ecs::system::SystemState::new(world);
            let collisions = system_state.get(world);
            (collisions.is_empty(), collisions.len())
        });

        assert!(is_empty, "Collisions should start empty");
        assert_eq!(len, 0, "Collisions should have 0 pairs initially");

        println!("✓ GodotCollisionsPlugin initializes correctly");

        app.cleanup();
        await_frames(1).await;
    })
}

/// Test that CollisionStarted observers fire when the event is triggered.
#[itest(async)]
fn test_collision_started_observer(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        await_frames(1).await;

        #[derive(Resource, Default)]
        struct CollisionCount(u32);

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotCollisionsPlugin);
            app.init_resource::<CollisionCount>();
            app.add_observer(
                |_trigger: On<CollisionStarted>, mut count: ResMut<CollisionCount>| {
                    count.0 += 1;
                },
            );
        })
        .await;

        app.update().await;

        app.with_world_mut(|world| {
            let e1 = world.spawn_empty().id();
            let e2 = world.spawn_empty().id();
            world.trigger(CollisionStarted {
                entity1: e1,
                entity2: e2,
            });
        });

        app.update().await;

        let count = app.with_world(|world| world.resource::<CollisionCount>().0);

        assert_eq!(count, 1, "CollisionStarted observer should fire once");

        println!("✓ CollisionStarted observer fires correctly");

        app.cleanup();
        await_frames(1).await;
    })
}

/// Test that CollisionEnded observers fire when the event is triggered.
#[itest(async)]
fn test_collision_ended_observer(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        await_frames(1).await;

        #[derive(Resource, Default)]
        struct EndedCount(u32);

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotCollisionsPlugin);
            app.init_resource::<EndedCount>();
            app.add_observer(
                |_trigger: On<CollisionEnded>, mut count: ResMut<EndedCount>| {
                    count.0 += 1;
                },
            );
        })
        .await;

        app.update().await;

        app.with_world_mut(|world| {
            let e1 = world.spawn_empty().id();
            let e2 = world.spawn_empty().id();
            world.trigger(CollisionEnded {
                entity1: e1,
                entity2: e2,
            });
        });

        app.update().await;

        let count = app.with_world(|world| world.resource::<EndedCount>().0);

        assert_eq!(count, 1, "CollisionEnded observer should fire once");

        println!("✓ CollisionEnded observer fires correctly");

        app.cleanup();
        await_frames(1).await;
    })
}
