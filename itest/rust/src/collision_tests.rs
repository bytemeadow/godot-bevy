/*
 * Collision system integration tests
 *
 * Tests the full collision pipeline through real Godot frames:
 * - CollisionWatcher receives collision events via channel
 * - Godot calls _physics_process() → FixedFirst runs
 * - process_godot_collisions drains channel → updates CollisionState
 * - trigger_collision_observers reads CollisionState → fires observers
 * - Collisions SystemParam provides query access
 *
 * Frame strategy: Collision processing runs in FixedFirst, which only
 * executes during Godot's _physics_process(). Since a render frame can have
 * 0 physics ticks, we use app.physics_update() which waits for the
 * physics_frame signal (guaranteeing a tick will run) then process_frame
 * (guaranteeing both _physics_process and _process have completed).
 */

use bevy::prelude::*;
use godot::classes::{
    Area2D, CircleShape2D, CollisionShape2D, RectangleShape2D, RigidBody2D, StaticBody2D,
};
use godot::prelude::*;
use godot_bevy::prelude::*;
use godot_bevy_test::prelude::*;

/// Read `Collisions::contains` through a `SystemState`, like the pure-state tests.
fn collisions_contains(app: &mut TestApp, a: Entity, b: Entity) -> bool {
    app.with_world_mut(|world| {
        let mut system_state: bevy::ecs::system::SystemState<Collisions> =
            bevy::ecs::system::SystemState::new(world);
        let collisions = system_state
            .get(world)
            .expect("system params should be valid in test");
        collisions.contains(a, b)
    })
}

/// Attach a `CollisionShape2D` with a `CircleShape2D` child to a physics node.
fn add_circle_collision<T>(parent: &Gd<T>, radius: f32)
where
    T: godot::obj::Inherits<Node>,
{
    let mut circle = CircleShape2D::new_gd();
    circle.set_radius(radius);
    let mut shape = CollisionShape2D::new_alloc();
    shape.set_shape(&circle);
    parent.clone().upcast::<Node>().add_child(&shape);
}

/// Attach a `CollisionShape2D` with a `RectangleShape2D` child to a physics node.
fn add_rect_collision<T>(parent: &Gd<T>, size: Vector2)
where
    T: godot::obj::Inherits<Node>,
{
    let mut rect = RectangleShape2D::new_gd();
    rect.set_size(size);
    let mut shape = CollisionShape2D::new_alloc();
    shape.set_shape(&rect);
    parent.clone().upcast::<Node>().add_child(&shape);
}

/// Find the CollisionWatcher node in the scene tree.
fn find_collision_watcher(
    scene_tree: &Gd<godot::classes::Node>,
) -> Option<Gd<godot::classes::Node>> {
    let tree = scene_tree.get_tree();
    let root = tree.get_root()?;
    root.try_get_node_as::<godot::classes::Node>("BevyAppSingleton/CollisionWatcher")
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

        let (area_a, entity_a) = app.add_node::<godot::classes::Area2D>("CollisionA").await;
        let (area_b, entity_b) = app.add_node::<godot::classes::Area2D>("CollisionB").await;

        let mut watcher = find_collision_watcher(&ctx_clone.scene_tree)
            .expect("CollisionWatcher should exist when GodotCollisionsPlugin is added");

        // Send Started event
        send_collision_event(
            &mut watcher,
            &area_b.clone().upcast(),
            &area_a.clone().upcast(),
            "Started",
        );

        // Wait for a physics tick + render frame so FixedFirst drains
        // the channel and updates CollisionState.
        app.physics_update().await;

        let (contains, colliding_with_a) = app.with_world_mut(|world| {
            let mut system_state: bevy::ecs::system::SystemState<Collisions> =
                bevy::ecs::system::SystemState::new(world);
            let collisions = system_state
                .get(world)
                .expect("system params should be valid in test");
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
            let collisions = system_state
                .get(world)
                .expect("system params should be valid in test");
            collisions.contains(entity_a, entity_b)
        });

        assert!(
            !still_contains,
            "Collision pair should be removed after Ended event"
        );

        app.cleanup().await;
        area_a.free();
        area_b.free();
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

        let (area_a, _entity_a) = app.add_node::<godot::classes::Area2D>("ObsStartA").await;
        let (area_b, _entity_b) = app.add_node::<godot::classes::Area2D>("ObsStartB").await;

        let mut watcher =
            find_collision_watcher(&ctx_clone.scene_tree).expect("CollisionWatcher should exist");

        send_collision_event(
            &mut watcher,
            &area_b.clone().upcast(),
            &area_a.clone().upcast(),
            "Started",
        );

        // physics_update() guarantees a physics tick runs, which processes
        // the collision and triggers the observer in the same FixedFirst.
        app.physics_update().await;

        let count = app.with_world(|world| world.resource::<CollisionCount>().0);

        assert_eq!(
            count, 1,
            "CollisionStarted observer should fire once from system pipeline"
        );

        app.cleanup().await;
        area_a.free();
        area_b.free();
    })
}

/// Reparenting a collision body must not drop its signal connection. The skip branch
/// leaves the existing connect intact (Godot preserves connections across a reparent),
/// so the `area_entered` connection count stays exactly one. Reads the connection state
/// directly rather than injecting synthetic events, which bypass the connection.
#[itest(async)]
fn test_reparent_keeps_collision_connection(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotCollisionsPlugin);
        })
        .await;

        let mut parent2 = Node::new_alloc();
        parent2.set_name("CollisionReparentParent");
        ctx_clone.scene_tree.clone().add_child(&parent2);

        let (area, _entity) = app.add_node::<godot::classes::Area2D>("ReparentArea").await;

        let baseline = area.get_signal_connection_list("area_entered").len();
        assert_eq!(
            baseline, 1,
            "decoration should connect area_entered exactly once"
        );

        area.clone()
            .upcast::<godot::classes::Node>()
            .reparent(&parent2);
        app.updates(2).await;

        let after = area.get_signal_connection_list("area_entered").len();
        assert_eq!(
            after, 1,
            "reparent must preserve the single area_entered connection"
        );

        app.cleanup().await;
        parent2.free();
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

        let (area_a, _entity_a) = app.add_node::<godot::classes::Area2D>("ObsEndA").await;
        let (area_b, _entity_b) = app.add_node::<godot::classes::Area2D>("ObsEndB").await;

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
        area_a.free();
        area_b.free();
    })
}

/// Freeing a node while it overlaps another must purge the pair and fire
/// `CollisionEnded` -- the "bullet dies on hit" idiom, which the channel path drops
/// because the freed node's index entry is gone before the exit resolves. Real Area2D
/// physics, so the overlap and the purge are polled as eventual transitions.
#[itest(async)]
fn test_free_while_overlapping(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct EndedCount(u32);

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotCollisionsPlugin);
            app.init_resource::<EndedCount>();
            app.add_observer(|_: On<CollisionEnded>, mut c: ResMut<EndedCount>| c.0 += 1);
        })
        .await;

        let (area_a, entity_a) = app.add_node::<Area2D>("OverlapA").await;
        add_circle_collision(&area_a, 32.0);
        let (area_b, entity_b) = app.add_node::<Area2D>("OverlapB").await;
        add_circle_collision(&area_b, 32.0);

        // Wait for the real overlap to form (polled, not assumed on a fixed frame).
        let mut overlapping = false;
        for _ in 0..30 {
            app.physics_update().await;
            if collisions_contains(&mut app, entity_a, entity_b) {
                overlapping = true;
                break;
            }
        }
        assert!(overlapping, "areas at the same position should overlap");

        // Free one while overlapping -- deferred, real frames run it.
        area_b.clone().upcast::<Node>().queue_free();

        let mut purged = false;
        for _ in 0..30 {
            app.physics_update().await;
            if !collisions_contains(&mut app, entity_a, entity_b) {
                purged = true;
                break;
            }
        }
        assert!(purged, "freeing an overlapping node must purge its pair");

        let ended = app.with_world(|w| w.resource::<EndedCount>().0);
        assert!(ended >= 1, "CollisionEnded should fire for the freed pair");

        app.cleanup().await;
        area_a.free();
    })
}

/// Spawning a monitoring Area directly into an existing overlap. The peer is a
/// StaticBody2D, which emits no enter signal for the Area, so the seed at connect
/// (`get_overlapping_bodies`) is the ONLY path that can capture the pair -- the test
/// genuinely fails if the seed is a no-op. (A monitoring-Area peer would capture the
/// pair via its own live `area_entered`, hiding a broken seed.)
#[itest(async)]
fn test_spawn_into_overlap(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct StartedCount(u32);

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotCollisionsPlugin);
            app.init_resource::<StartedCount>();
            app.add_observer(|_: On<CollisionStarted>, mut c: ResMut<StartedCount>| c.0 += 1);
        })
        .await;

        let (wall, wall_entity) = app.add_node::<StaticBody2D>("Wall").await;
        add_rect_collision(&wall, Vector2::new(64.0, 64.0));
        // Let physics register the static body's shape before the Area spawns.
        app.physics_update().await;
        app.physics_update().await;

        // Build the Area fully -- shape child attached -- BEFORE it enters the tree, so
        // the overlap exists at the first flush and the seed reads a populated body_map.
        let seed = Area2D::new_alloc();
        add_circle_collision(&seed, 16.0);
        let (seed, seed_entity) = app.add_prebuilt_node(seed, "SeedArea").await;

        // body_entered(Wall) fired to zero connections before connect.
        let mut captured = false;
        for _ in 0..30 {
            app.physics_update().await;
            if collisions_contains(&mut app, seed_entity, wall_entity) {
                captured = true;
                break;
            }
        }
        assert!(
            captured,
            "the seed must capture the spawn-into-overlap pair"
        );

        let started = app.with_world(|w| w.resource::<StartedCount>().0);
        assert!(
            started >= 1,
            "CollisionStarted should fire for the seeded overlap"
        );

        app.cleanup().await;
        seed.free();
        wall.free();
    })
}

/// The contact-monitor warn predicate. A default RigidBody2D has `contact_monitor` disabled, so its
/// `body_entered/exited` never fire and godot-bevy warns at connect. This mirrors the
/// private `is_rigid_body_without_contact_monitor` condition on a real node and drives
/// the connect+warn path (must not panic). The warn line itself is a manual check (log
/// capture); this asserts the boolean condition it keys on.
#[itest(async)]
fn test_rigid_body_without_contact_monitor(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotCollisionsPlugin);
        })
        .await;

        let (rigid, _entity) = app.add_node::<RigidBody2D>("NoContactMonitor").await;
        assert!(
            !rigid.is_contact_monitor_enabled(),
            "a default RigidBody2D has contact_monitor disabled -- the warn condition"
        );

        // Enabling it flips the predicate false (no warn).
        let mut enabled = RigidBody2D::new_alloc();
        enabled.set_contact_monitor(true);
        assert!(enabled.is_contact_monitor_enabled());
        enabled.free();

        // The connect+warn path ran for `rigid` without panicking.
        app.physics_update().await;

        app.cleanup().await;
        rigid.free();
    })
}
