/*
 * Collision system integration tests
 *
 * Tests the full collision pipeline through real Godot frames:
 * - CollisionWatcher receives collision events via channel
 * - Godot calls _physics_process() → PrePhysicsUpdate runs
 * - process_godot_collisions drains channel → updates CollisionState
 * - trigger_collision_observers reads CollisionState → fires observers
 * - Collisions SystemParam provides query access
 *
 * Frame strategy: Collision processing runs in PrePhysicsUpdate, which only
 * executes during Godot's _physics_process(). Since a render frame can have
 * 0 physics ticks, we use app.physics_update() which waits for the
 * physics_frame signal (guaranteeing a tick will run) then process_frame
 * (guaranteeing both _physics_process and _process have completed).
 */

use bevy::prelude::*;
use godot::prelude::*;
use godot_bevy::prelude::*;
use godot_bevy_test::prelude::*;

/// Find the CollisionWatcher node in the scene tree.
fn find_collision_watcher(
    scene_tree: &Gd<godot::classes::Node>,
) -> Option<Gd<godot::classes::Node>> {
    for child in scene_tree.get_children().iter_shared() {
        if let Some(watcher) = child.get_node_or_null("CollisionWatcher") {
            return Some(watcher);
        }
    }
    None
}

/// Send a collision event through the CollisionWatcher channel.
fn send_collision_event(
    watcher: &mut Gd<godot::classes::Node>,
    colliding_body: &Gd<godot::classes::Node>,
    origin_node: &Gd<godot::classes::Node>,
    event_type: &str,
) {
    watcher.call(
        "collision_event",
        &[
            colliding_body.to_variant(),
            origin_node.to_variant(),
            event_type.to_variant(),
        ],
    );
}

/// Test that collision events flow through the system and update CollisionState.
#[itest(async)]
fn test_collision_state_tracks_active_pairs(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotCollisionsPlugin);
        })
        .await;

        let (mut area_a, entity_a) = app.add_node::<godot::classes::Area2D>("CollisionA").await;
        let (mut area_b, entity_b) = app.add_node::<godot::classes::Area2D>("CollisionB").await;

        let mut watcher = find_collision_watcher(&ctx_clone.scene_tree)
            .expect("CollisionWatcher should exist when GodotCollisionsPlugin is added");

        // Send Started event
        send_collision_event(
            &mut watcher,
            &area_b.clone().upcast(),
            &area_a.clone().upcast(),
            "Started",
        );

        // Wait for a physics tick + render frame so PrePhysicsUpdate drains
        // the channel and updates CollisionState.
        app.physics_update().await;

        let (contains, colliding_with_a) = app.with_world_mut(|world| {
            let mut system_state: bevy::ecs::system::SystemState<Collisions> =
                bevy::ecs::system::SystemState::new(world);
            let collisions = system_state.get(world);
            let contains = collisions.contains(entity_a, entity_b);
            let colliding: Vec<Entity> = collisions.colliding_with(entity_a).to_vec();
            (contains, colliding)
        });

        assert!(
            contains,
            "Collisions should track the active pair after Started event"
        );
        assert!(
            colliding_with_a.contains(&entity_b),
            "colliding_with should return entity_b for entity_a"
        );

        // Send Ended event
        send_collision_event(
            &mut watcher,
            &area_b.clone().upcast(),
            &area_a.clone().upcast(),
            "Ended",
        );

        app.physics_update().await;

        let still_contains = app.with_world_mut(|world| {
            let mut system_state: bevy::ecs::system::SystemState<Collisions> =
                bevy::ecs::system::SystemState::new(world);
            let collisions = system_state.get(world);
            collisions.contains(entity_a, entity_b)
        });

        assert!(
            !still_contains,
            "Collision pair should be removed after Ended event"
        );

        app.cleanup().await;
        area_a.queue_free();
        area_b.queue_free();
    })
}

/// Test that CollisionStarted observers fire from the real system pipeline.
#[itest(async)]
fn test_collision_started_observer_from_system(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
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

        let (mut area_a, _entity_a) = app.add_node::<godot::classes::Area2D>("ObsStartA").await;
        let (mut area_b, _entity_b) = app.add_node::<godot::classes::Area2D>("ObsStartB").await;

        let mut watcher =
            find_collision_watcher(&ctx_clone.scene_tree).expect("CollisionWatcher should exist");

        send_collision_event(
            &mut watcher,
            &area_b.clone().upcast(),
            &area_a.clone().upcast(),
            "Started",
        );

        // physics_update() guarantees a physics tick runs, which processes
        // the collision and triggers the observer in the same PrePhysicsUpdate.
        app.physics_update().await;

        let count = app.with_world(|world| world.resource::<CollisionCount>().0);

        assert_eq!(
            count, 1,
            "CollisionStarted observer should fire once from system pipeline"
        );

        app.cleanup().await;
        area_a.queue_free();
        area_b.queue_free();
    })
}

/// Test that CollisionEnded observers fire from the real system pipeline.
#[itest(async)]
fn test_collision_ended_observer_from_system(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
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

        let (mut area_a, _entity_a) = app.add_node::<godot::classes::Area2D>("ObsEndA").await;
        let (mut area_b, _entity_b) = app.add_node::<godot::classes::Area2D>("ObsEndB").await;

        let mut watcher =
            find_collision_watcher(&ctx_clone.scene_tree).expect("CollisionWatcher should exist");

        // First: Start collision
        send_collision_event(
            &mut watcher,
            &area_b.clone().upcast(),
            &area_a.clone().upcast(),
            "Started",
        );

        app.physics_update().await;

        // Then: End collision
        send_collision_event(
            &mut watcher,
            &area_b.clone().upcast(),
            &area_a.clone().upcast(),
            "Ended",
        );

        app.physics_update().await;

        let count = app.with_world(|world| world.resource::<EndedCount>().0);

        assert_eq!(
            count, 1,
            "CollisionEnded observer should fire once from system pipeline"
        );

        app.cleanup().await;
        area_a.queue_free();
        area_b.queue_free();
    })
}
