/*
 * Event-bridge timing tests (Phase 1: dual drain).
 *
 * The shared signal channel is now drained in BOTH First (process) and
 * PrePhysicsUpdate (physics). These tests pin the precise same-frame guarantee
 * and its documented complement, using a Godot signal as the enqueue path
 * (send_event lands in a later phase). emit_signal() invokes the connected
 * Callable synchronously, so the envelope is in the channel before the next
 * physics_process.
 */

use bevy::prelude::*;
use godot_bevy::prelude::*;
use godot_bevy_test::prelude::*;

#[derive(Event, Debug, Clone)]
struct Tick;

#[derive(Resource, Default)]
struct Timing {
    fired_in_observer: bool,
    /// Set by a PhysicsUpdate reader IF the observer flag was already set when
    /// the reader ran this physics frame. Proves same-frame physics delivery.
    seen_by_physics_this_frame: bool,
    /// How many times the observer fired (no-double-fire control).
    observer_count: u32,
    /// What `fired_in_observer` was when this frame's PhysicsUpdate reader ran --
    /// lets a test tell same-frame delivery from next-frame.
    seen_this_physics_frame_only: bool,
}

/// Connect a Button's "pressed" signal so emitting it enqueues a Tick into the
/// shared channel. Returns the button so the test can emit + free it.
async fn connect_tick_button(
    app: &mut TestApp,
    name: &str,
) -> godot::obj::Gd<godot::classes::Button> {
    let (button, _entity) = app.add_node::<godot::classes::Button>(name).await;
    let button_id = button.instance_id();

    app.with_world_mut(|world| {
        let handle = world
            .get::<GodotNodeHandle>(
                world
                    .resource::<NodeEntityIndex>()
                    .get(button_id)
                    .expect("button entity should exist in index"),
            )
            .copied()
            .expect("entity should have GodotNodeHandle");

        let mut system_state: bevy::ecs::system::SystemState<GodotSignals<Tick>> =
            bevy::ecs::system::SystemState::new(world);
        let signals = system_state.get(world);
        signals.connect(handle, "pressed", None, |_args, _handle, _entity| {
            Some(Tick)
        });
        system_state.apply(world);
    });

    // One frame so the pending signal connection is actually wired.
    app.update().await;
    button
}

#[itest(async)]
fn event_bridge_timing_pre_physics_same_frame(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotSignalsPlugin::<Tick>::default());
            app.init_resource::<Timing>();
            app.add_observer(|_t: On<Tick>, mut timing: ResMut<Timing>| {
                timing.fired_in_observer = true;
                timing.observer_count += 1;
            });
            // Runs inside physics_process, after PrePhysicsUpdate's drain -- if the
            // drain fired the observer this frame, the flag is already set here.
            app.add_systems(PhysicsUpdate, |mut timing: ResMut<Timing>| {
                let already = timing.fired_in_observer;
                timing.seen_this_physics_frame_only = already;
                if already {
                    timing.seen_by_physics_this_frame = true;
                }
            });
        })
        .await;

        let mut button = connect_tick_button(&mut app, "PrePhysicsTickButton").await;

        // Enqueue before physics runs: emit_signal calls the Callable
        // synchronously, so the Tick envelope is in the channel now.
        button.emit_signal("pressed", &[]);

        // Drive one physics frame: PrePhysicsUpdate (drain) then PhysicsUpdate (reader).
        app.physics_update().await;

        let seen = app.with_world(|world| world.resource::<Timing>().seen_by_physics_this_frame);
        assert!(
            seen,
            "an item enqueued before physics_process must be drained in \
             PrePhysicsUpdate and visible to PhysicsUpdate the SAME frame"
        );

        app.cleanup().await;
        button.queue_free();
    })
}

#[itest(async)]
fn event_bridge_timing_post_physics_next_frame(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotSignalsPlugin::<Tick>::default());
            app.init_resource::<Timing>();
            app.add_observer(|_t: On<Tick>, mut timing: ResMut<Timing>| {
                timing.fired_in_observer = true;
                timing.observer_count += 1;
            });
            app.add_systems(PhysicsUpdate, |mut timing: ResMut<Timing>| {
                let already = timing.fired_in_observer;
                timing.seen_this_physics_frame_only = already;
                if already {
                    timing.seen_by_physics_this_frame = true;
                }
            });
        })
        .await;

        let mut button = connect_tick_button(&mut app, "PostPhysicsTickButton").await;

        // Advance one physics frame with nothing queued, so this frame's tick
        // has already run by the time physics_update() returns.
        app.physics_update().await;

        // Enqueue after that physics tick -- the just-elapsed tick can't have seen it.
        button.emit_signal("pressed", &[]);

        let seen_by_prior_tick =
            app.with_world(|world| world.resource::<Timing>().seen_this_physics_frame_only);
        assert!(
            !seen_by_prior_tick,
            "an item enqueued after this frame's physics tick must NOT be seen \
             by that already-elapsed tick"
        );

        // Next physics frame drains it: delivery is deferred, never dropped.
        app.physics_update().await;
        let seen_next =
            app.with_world(|world| world.resource::<Timing>().seen_by_physics_this_frame);
        assert!(
            seen_next,
            "the deferred item must be delivered on the NEXT physics frame"
        );

        app.cleanup().await;
        button.queue_free();
    })
}

#[itest(async)]
fn event_bridge_no_double_fire(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotSignalsPlugin::<Tick>::default());
            app.init_resource::<Timing>();
            app.add_observer(|_t: On<Tick>, mut timing: ResMut<Timing>| {
                timing.observer_count += 1;
            });
        })
        .await;

        let mut button = connect_tick_button(&mut app, "NoDoubleFireButton").await;

        // Enqueue one item, then run both a render frame (First drain) and a
        // physics frame (PrePhysicsUpdate drain). try_iter removes items, so
        // whichever drain runs first consumes it; the other finds it gone.
        button.emit_signal("pressed", &[]);
        app.update().await;
        app.physics_update().await;

        let count = app.with_world(|world| world.resource::<Timing>().observer_count);
        assert_eq!(
            count, 1,
            "a single enqueued item must fire its observer exactly once across \
             both drains (no double-fire)"
        );

        app.cleanup().await;
        button.queue_free();
    })
}

use godot::obj::NewAlloc;
use godot::prelude::*;
use godot_bevy::BevyApp;
use std::sync::{
    Arc,
    atomic::{AtomicI32, Ordering::SeqCst},
};

#[derive(Event, Debug, Clone)]
struct Damage {
    amount: i32,
}

/// Resolve the autoload BevyApp node the harness wraps.
fn singleton_node(ctx: &TestContext) -> godot::obj::Gd<BevyApp> {
    ctx.scene_tree
        .get_tree()
        .get_root()
        .expect("root exists")
        .try_get_node_as::<BevyApp>("BevyAppSingleton")
        .expect("BevyAppSingleton autoload should exist")
}

/// Node-scoped send_event delivers to an On<Damage> observer.
#[itest(async)]
fn test_send_event_rust_node_scoped(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct Received(i32);

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotEventBridgePlugin);
            app.init_resource::<Received>();
            app.add_observer(|t: On<Damage>, mut r: ResMut<Received>| {
                r.0 = t.event().amount;
            });
        })
        .await;

        let node = singleton_node(&ctx_clone);
        godot_bevy::send_event(&node, Damage { amount: 7 });

        // First-schedule drain triggers the observer on the next frame.
        app.update().await;

        let got = app.with_world(|w| w.resource::<Received>().0);
        assert_eq!(got, 7, "send_event should deliver to On<Damage> observer");

        app.cleanup().await;
    })
}

/// try_singleton resolves the autoload and delivers identically.
#[itest(async)]
fn test_send_event_via_try_singleton(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct Received(i32);

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotEventBridgePlugin);
            app.init_resource::<Received>();
            app.add_observer(|t: On<Damage>, mut r: ResMut<Received>| {
                r.0 = t.event().amount;
            });
        })
        .await;

        let resolved = BevyApp::try_singleton().expect("singleton should resolve in-tree");
        godot_bevy::send_event(&resolved, Damage { amount: 9 });

        app.update().await;

        let got = app.with_world(|w| w.resource::<Received>().0);
        assert_eq!(got, 9, "try_singleton-resolved send_event should deliver");

        app.cleanup().await;
    })
}

/// After teardown, send_event is a no-op (app dead, no panic).
#[itest(async)]
fn test_send_event_after_teardown_noop(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        let witness = Arc::new(AtomicI32::new(0));
        let witness_clone = Arc::clone(&witness);

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotEventBridgePlugin);
            app.add_observer(move |t: On<Damage>| {
                witness_clone.store(t.event().amount, SeqCst);
            });
        })
        .await;

        let node = singleton_node(&ctx_clone);

        // Sanity-check the observer fires before we tear down.
        godot_bevy::send_event(&node, Damage { amount: 1 });
        app.update().await;
        assert_ne!(witness.load(SeqCst), 0, "observer must fire on a live app");
        witness.store(0, SeqCst);

        node.clone().bind_mut().teardown();
        assert!(node.bind().get_app().is_none(), "teardown clears the App");

        godot_bevy::send_event(&node, Damage { amount: 42 });
        assert_eq!(
            witness.load(SeqCst),
            0,
            "no delivery after teardown — zombie sender must not fire the observer"
        );

        // cleanup() takes the harness's handle, already torn down — safe.
        app.cleanup().await;
    })
}

/// A BevyApp with no signal channel (GodotCorePlugins only) no-ops.
#[itest(async)]
fn test_send_event_no_channel_noop(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct Received(i32);

        // GodotCorePlugins is added automatically by TestApp; add nothing that
        // installs the channel -> no SignalSender resource.
        let mut app = TestApp::new(&ctx_clone, |app| {
            app.init_resource::<Received>();
            app.add_observer(|t: On<Damage>, mut r: ResMut<Received>| {
                r.0 = t.event().amount;
            });
        })
        .await;

        let node = singleton_node(&ctx_clone);
        godot_bevy::send_event(&node, Damage { amount: 5 });

        // Drive the world; the observer must not fire -- no channel, no delivery.
        app.update().await;

        let got = app.with_world(|w| w.resource::<Received>().0);
        assert_eq!(got, 0, "no delivery when there is no signal channel");

        app.cleanup().await;
    })
}

/// A never-initialized BevyApp node (get_app() == None) no-ops.
#[itest(async)]
fn test_send_event_no_live_app_noop(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        // Never initialized, so the app is None -- no world, so nothing to witness
        // delivery. All this test can prove is that send_event survives (no FFI panic).
        let mut node = BevyApp::new_alloc();
        node.set_name("BevyAppUninitialized");
        let mut scene_tree = ctx_clone.scene_tree.clone();
        scene_tree.add_child(&node.clone().upcast::<godot::classes::Node>());

        assert!(node.bind().get_app().is_none(), "uninitialized app is None");

        godot_bevy::send_event(&node, Damage { amount: 3 });

        // Synchronous cleanup: remove from tree then free (deferred queue_free()
        // never runs during the synchronous itest suite and would leak the node).
        scene_tree.remove_child(&node.clone().upcast::<godot::classes::Node>());
        node.upcast::<godot::classes::Node>().free();
    })
}

/// Two live BevyApp instances; each event lands in its own app. Guards
/// node-scoping -- a single global sender would route both to whichever app
/// initialized last.
#[itest(async)]
fn test_send_event_multi_live_app(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct Received(i32);

        // App A: the autoload, via the harness.
        let mut app_a = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotEventBridgePlugin);
            app.init_resource::<Received>();
            app.add_observer(|t: On<Damage>, mut r: ResMut<Received>| {
                r.0 = t.event().amount;
            });
        })
        .await;
        let node_a = singleton_node(&ctx_clone);

        // App B: a second live BevyApp built by hand (mirrors TestApp::new).
        let mut node_b = BevyApp::new_alloc();
        node_b.set_name("BevyAppSecondInstance");
        node_b
            .bind_mut()
            .set_instance_init_func(Box::new(|app: &mut App| {
                app.add_plugins(GodotEventBridgePlugin);
                app.init_resource::<Received>();
                app.add_observer(|t: On<Damage>, mut r: ResMut<Received>| {
                    r.0 = t.event().amount;
                });
            }));
        ctx_clone
            .scene_tree
            .clone()
            .add_child(&node_b.clone().upcast::<godot::classes::Node>());
        node_b.bind_mut().initialize();
        // Let B settle (process() runs its First drain just like A).
        app_a.update().await;

        godot_bevy::send_event(&node_a, Damage { amount: 1 });
        godot_bevy::send_event(&node_b, Damage { amount: 2 });

        // Two updates so both apps' First drains have run.
        app_a.update().await;
        app_a.update().await;

        let got_a = app_a.with_world(|w| w.resource::<Received>().0);
        let got_b = node_b
            .bind()
            .get_app()
            .expect("B still live")
            .world()
            .resource::<Received>()
            .0;

        assert_eq!(got_a, 1, "App A must receive only its own event (amount 1)");
        assert_eq!(got_b, 2, "App B must receive only its own event (amount 2)");

        app_a.cleanup().await;
        node_b.bind_mut().teardown();
        node_b.upcast::<godot::classes::Node>().queue_free();
    })
}

/// Resolve the test's BevyApp autoload node as a generic `Gd<Node>` for `.call`.
fn bridge_node(scene_tree: &Gd<godot::classes::Node>) -> Gd<godot::classes::Node> {
    let tree = scene_tree.get_tree();
    let root = tree.get_root().expect("root should exist");
    root.try_get_node_as::<godot::classes::Node>("BevyAppSingleton")
        .expect("BevyAppSingleton should exist")
}

#[derive(Event, Debug, Clone)]
struct DamageI64 {
    amount: i64,
}

/// GDScript `send_event` with a dict payload delivers to an On<DamageI64> observer.
#[itest(async)]
fn test_send_event_dict_payload(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct Received(i64);

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotEventBridgePlugin);
            app.init_resource::<Received>();
            app.add_godot_event::<DamageI64>("damage", |payload| {
                let dict = payload.try_to::<godot::builtin::VarDictionary>().ok()?;
                Some(DamageI64 {
                    amount: dict.get("amount")?.try_to::<i64>().ok()?,
                })
            });
            app.add_observer(|t: On<DamageI64>, mut r: ResMut<Received>| {
                r.0 = t.event().amount;
            });
        })
        .await;

        let mut bridge = bridge_node(&ctx_clone.scene_tree);
        bridge.call(
            "send_event",
            &[
                "damage".to_variant(),
                vdict! { "amount" => 7i64 }.to_variant(),
            ],
        );

        app.update().await;

        let got = app.with_world(|w| w.resource::<Received>().0);
        assert_eq!(got, 7, "dict-payload event should deliver amount=7");

        app.cleanup().await;
    })
}

#[derive(Event, Debug, Clone, GodotConvert)]
#[godot(transparent)]
struct Volume(f64);

/// GDScript `send_event` with a newtype payload via `add_godot_event_from`.
#[itest(async)]
fn test_send_event_from_newtype(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct GotVolume(f64);

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotEventBridgePlugin);
            app.init_resource::<GotVolume>();
            app.add_godot_event_from::<Volume>("volume");
            app.add_observer(|t: On<Volume>, mut r: ResMut<GotVolume>| {
                r.0 = t.event().0;
            });
        })
        .await;

        let mut bridge = bridge_node(&ctx_clone.scene_tree);
        bridge.call("send_event", &["volume".to_variant(), 0.5f64.to_variant()]);

        app.update().await;

        let got = app.with_world(|w| w.resource::<GotVolume>().0);
        assert_eq!(got, 0.5, "newtype FromGodot event should deliver 0.5");

        app.cleanup().await;
    })
}

#[derive(Event, Debug, Clone)]
struct GameOver;

/// GDScript `send_event` with null payload fires a unit event.
#[itest(async)]
fn test_send_event_unit_via_null(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct GameOverSeen(bool);

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotEventBridgePlugin);
            app.init_resource::<GameOverSeen>();
            app.add_godot_event::<GameOver>("game_over", |_payload| Some(GameOver));
            app.add_observer(|_t: On<GameOver>, mut r: ResMut<GameOverSeen>| {
                r.0 = true;
            });
        })
        .await;

        let mut bridge = bridge_node(&ctx_clone.scene_tree);
        bridge.call("send_event", &["game_over".to_variant(), Variant::nil()]);

        app.update().await;

        let seen = app.with_world(|w| w.resource::<GameOverSeen>().0);
        assert!(seen, "unit event fired with null payload should deliver");

        app.cleanup().await;
    })
}

#[itest(async)]
fn test_send_event_unknown_and_bad_payload_drop(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct Received(i64);

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotEventBridgePlugin);
            app.init_resource::<Received>();
            app.add_godot_event::<DamageI64>("damage", |payload| {
                let dict = payload.try_to::<godot::builtin::VarDictionary>().ok()?;
                Some(DamageI64 {
                    amount: dict.get("amount")?.try_to::<i64>().ok()?,
                })
            });
            app.add_observer(|t: On<DamageI64>, mut r: ResMut<Received>| {
                r.0 = t.event().amount;
            });
        })
        .await;

        let mut bridge = bridge_node(&ctx_clone.scene_tree);
        // Unknown name -> warn (lists registered names) + no-op, no panic across FFI.
        bridge.call("send_event", &["nope".to_variant(), Variant::nil()]);
        // Bad payload (string, not a dict) -> mapper returns None -> dropped, no panic.
        bridge.call(
            "send_event",
            &["damage".to_variant(), "not a dict".to_variant()],
        );

        app.update().await;

        let got = app.with_world(|w| w.resource::<Received>().0);
        assert_eq!(got, 0, "unknown name and bad payload must not deliver");

        app.cleanup().await;
    })
}

#[itest(async)]
fn test_send_event_strict_int_drop_then_success(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct Received(i64);

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotEventBridgePlugin);
            app.init_resource::<Received>();
            app.add_godot_event::<DamageI64>("damage", |payload| {
                let dict = payload.try_to::<godot::builtin::VarDictionary>().ok()?;
                Some(DamageI64 {
                    amount: dict.get("amount")?.try_to::<i64>().ok()?,
                })
            });
            app.add_observer(|t: On<DamageI64>, mut r: ResMut<Received>| {
                r.0 = t.event().amount;
            });
        })
        .await;

        let mut bridge = bridge_node(&ctx_clone.scene_tree);

        // Float payload: strict try_to::<i64>() fails -> mapper None -> dropped.
        bridge.call(
            "send_event",
            &[
                "damage".to_variant(),
                vdict! { "amount" => 10.0f64 }.to_variant(),
            ],
        );
        app.update().await;
        assert_eq!(
            app.with_world(|w| w.resource::<Received>().0),
            0,
            "float payload must be dropped under strict i64 conversion"
        );

        // Int payload: succeeds.
        bridge.call(
            "send_event",
            &[
                "damage".to_variant(),
                vdict! { "amount" => 10i64 }.to_variant(),
            ],
        );
        app.update().await;
        assert_eq!(
            app.with_world(|w| w.resource::<Received>().0),
            10,
            "int payload must deliver amount=10"
        );

        app.cleanup().await;
    })
}

#[itest(async)]
fn test_send_event_unknown_name_spam_is_rate_limited(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct Received(i64);

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotEventBridgePlugin);
            app.init_resource::<Received>();
            app.add_godot_event::<DamageI64>("damage", |payload| {
                let dict = payload.try_to::<godot::builtin::VarDictionary>().ok()?;
                Some(DamageI64 {
                    amount: dict.get("amount")?.try_to::<i64>().ok()?,
                })
            });
            app.add_observer(|t: On<DamageI64>, mut r: ResMut<Received>| {
                r.0 = t.event().amount;
            });
        })
        .await;

        let mut bridge = bridge_node(&ctx_clone.scene_tree);
        // 200 unknown-name calls -- assert no panic and no delivery. The warner's
        // power-of-two decay is covered by the RateLimitedWarner unit tests.
        for _ in 0..200 {
            bridge.call("send_event", &["nope".to_variant(), Variant::nil()]);
        }

        app.update().await;

        let got = app.with_world(|w| w.resource::<Received>().0);
        assert_eq!(
            got, 0,
            "200 unknown-name calls must deliver nothing and not panic"
        );

        app.cleanup().await;
    })
}

#[derive(Event, Debug, Clone)]
struct Heal {
    amount: i64,
}

/// Two registered names produce two independent event types; both observers fire.
#[itest(async)]
fn test_send_event_multiple_names(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct Tally {
            damage: i64,
            heal: i64,
        }

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotEventBridgePlugin);
            app.init_resource::<Tally>();
            app.add_godot_event::<DamageI64>("damage", |p| {
                let d = p.try_to::<godot::builtin::VarDictionary>().ok()?;
                Some(DamageI64 {
                    amount: d.get("amount")?.try_to::<i64>().ok()?,
                })
            });
            app.add_godot_event::<Heal>("heal", |p| {
                let d = p.try_to::<godot::builtin::VarDictionary>().ok()?;
                Some(Heal {
                    amount: d.get("amount")?.try_to::<i64>().ok()?,
                })
            });
            app.add_observer(|t: On<DamageI64>, mut r: ResMut<Tally>| r.damage = t.event().amount);
            app.add_observer(|t: On<Heal>, mut r: ResMut<Tally>| r.heal = t.event().amount);
        })
        .await;

        let mut bridge = bridge_node(&ctx_clone.scene_tree);
        bridge.call(
            "send_event",
            &[
                "damage".to_variant(),
                vdict! { "amount" => 3i64 }.to_variant(),
            ],
        );
        bridge.call(
            "send_event",
            &[
                "heal".to_variant(),
                vdict! { "amount" => 5i64 }.to_variant(),
            ],
        );

        app.update().await;

        let (d, h) = app.with_world(|w| {
            let t = w.resource::<Tally>();
            (t.damage, t.heal)
        });
        assert_eq!((d, h), (3, 5), "both registered events must fire");

        app.cleanup().await;
    })
}

#[derive(Event, Debug, Clone)]
struct EvA;

#[derive(Event, Debug, Clone)]
struct EvB;

/// Registering the same name twice: last registration wins (EvA overwritten by EvB).
#[itest(async)]
fn test_send_event_same_name_last_wins(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct Seen {
            a: bool,
            b: bool,
        }

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotEventBridgePlugin);
            app.init_resource::<Seen>();
            app.add_godot_event::<EvA>("x", |_p| Some(EvA));
            app.add_godot_event::<EvB>("x", |_p| Some(EvB));
            app.add_observer(|_t: On<EvA>, mut s: ResMut<Seen>| s.a = true);
            app.add_observer(|_t: On<EvB>, mut s: ResMut<Seen>| s.b = true);
        })
        .await;

        let mut bridge = bridge_node(&ctx_clone.scene_tree);
        bridge.call("send_event", &["x".to_variant(), Variant::nil()]);

        app.update().await;

        let (a, b) = app.with_world(|w| {
            let s = w.resource::<Seen>();
            (s.a, s.b)
        });
        assert!(!a, "first-registered EvA must NOT fire (overwritten)");
        assert!(b, "last-registered EvB must fire (last-wins)");

        app.cleanup().await;
    })
}

/// `BevyApp::send_event` (method form) delivers to an `On<Damage>` observer.
/// This is also the guard for the name collision: if a `#[func] send_event`
/// ever shadows this method, the call below resolves to the private
/// `GString`/`Variant` func and the test stops compiling.
#[itest(async)]
fn test_send_event_method_form_delivers(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct Received(i32);

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.add_plugins(GodotEventBridgePlugin);
            app.init_resource::<Received>();
            app.add_observer(|t: On<Damage>, mut r: ResMut<Received>| {
                r.0 = t.event().amount;
            });
        })
        .await;

        // Natural method form — must resolve to the typed Rust method, not the GDScript #[func].
        let singleton = singleton_node(&ctx_clone);
        singleton.bind().send_event(Damage { amount: 13 });

        app.update().await;

        let got = app.with_world(|w| w.resource::<Received>().0);
        assert_eq!(
            got, 13,
            "app.bind().send_event(ev) should deliver to On<Damage> observer"
        );

        app.cleanup().await;
    })
}

/// A mapper re-enters `send_event` on the same BevyApp node during decode.
///
/// `send_event`'s `#[func]` takes `&self`, so this synchronous re-entry can't
/// double-mutably-borrow the node, and so doesn't panic. The mapper captures the
/// node's `InstanceId` (not the `Gd`, which isn't `Send + Sync`) and recovers it
/// to make the nested call.
#[itest(async)]
fn test_send_event_reentrant_mapper(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    let scene_tree = ctx_clone.scene_tree.clone();

    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct Seen {
            outer: bool,
            inner: bool,
        }

        // Obtain the BevyApp node's InstanceId before registering mappers so we
        // can capture it (Send + Sync + Copy) without capturing the Gd<Node>.
        let bevy_app_iid = scene_tree
            .get_tree()
            .get_root()
            .expect("root exists")
            .try_get_node_as::<godot::classes::Node>("BevyAppSingleton")
            .expect("BevyAppSingleton exists")
            .instance_id();

        let mut app = TestApp::new(&ctx_clone, move |app| {
            app.add_plugins(GodotEventBridgePlugin);
            app.init_resource::<Seen>();
            app.add_godot_event::<EvB>("inner", |_p| Some(EvB));
            // The "outer" mapper calls `send_event` on the same BevyApp node via its
            // InstanceId — a genuine synchronous re-entry into `#[func] send_event`.
            // With `&self`, this cannot double-mutably-borrow; no panic.
            app.add_godot_event::<EvA>("outer", move |_p| {
                if let Ok(mut node) =
                    godot::obj::Gd::<godot::classes::Node>::try_from_instance_id(bevy_app_iid)
                {
                    node.call("send_event", &["inner".to_variant(), Variant::nil()]);
                }
                Some(EvA)
            });
            app.add_observer(|_t: On<EvA>, mut s: ResMut<Seen>| s.outer = true);
            app.add_observer(|_t: On<EvB>, mut s: ResMut<Seen>| s.inner = true);
        })
        .await;

        let mut bridge = bridge_node(&scene_tree);
        bridge.call("send_event", &["outer".to_variant(), Variant::nil()]);

        // Two drains: the re-entrant inner enqueue may be delivered on the next
        // drain rather than within the current #[func] call.
        app.update().await;
        app.update().await;

        let (outer, inner) = app.with_world(|w| {
            let s = w.resource::<Seen>();
            (s.outer, s.inner)
        });
        assert!(outer, "outer event must fire (no re-entrancy panic)");
        assert!(
            inner,
            "re-entrant inner event must also be enqueued and fire"
        );

        app.cleanup().await;
    })
}
