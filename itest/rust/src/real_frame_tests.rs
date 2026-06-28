/*
 * Real frame-driven integration tests
 * These tests verify actual Godot frame progression
 */

use bevy::prelude::*;
use godot_bevy_test::prelude::*;

/// Exactly-one-clear: RemovedComponents retains for the whole render frame and is
/// gone the next (the clear invariant). Also verifies Changed<T> set in PreUpdate
/// reaches both FixedUpdate and Update in the same render frame (the prefix→fixed→
/// suffix ordering invariant -- a separate property; clear_trackers does not reset
/// component change ticks, so the Changed checks do not depend on the clear).
///
/// What a failure means:
/// - removed_activation != 1 → clear_trackers ran mid-frame (e.g. after prefix),
///   robbing the suffix of the removal.
/// - removed_next != 0 → clear_trackers didn't run at end of frame (leak).
/// - fixed_saw_changed == 0 after 10 frames → Changed<T> never reached FixedUpdate;
///   means PreUpdate did not run before FixedMain (broken prefix→fixed ordering).
/// - update_saw_changed == 0 → Changed<T> never reached Update (always broken).
#[itest(async)]
fn test_exactly_one_clear(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        #[derive(Component)]
        struct Marker;

        #[derive(Component)]
        struct Tracked(i32);

        #[derive(Resource, Default)]
        struct RemoveProbe {
            // 0 = idle; 1 = armed (PreUpdate will remove + advance to 2);
            // 2 = removal done, Update records activation count + advances to 3;
            // 3 = next frame, Update records the follow-up count + advances to 4.
            phase: u8,
            removed_activation: usize,
            removed_next: usize,
        }

        let fixed_saw_changed = Counter::new();
        let update_saw_changed = Counter::new();
        let fsc = fixed_saw_changed.clone();
        let usc = update_saw_changed.clone();

        let mut app = TestApp::new(&ctx_clone, move |app| {
            let fsc_sys = fsc.clone();
            let usc_sys = usc.clone();

            app.init_resource::<RemoveProbe>();
            app.add_systems(Startup, |mut commands: Commands| {
                commands.spawn((Marker, Tracked(0)));
            });
            // When armed (phase 1), remove the marker and bump to phase 2 in the
            // same PreUpdate run -- prefix always runs before Update within a frame,
            // so the phase==2 Update is guaranteed to follow the removal regardless
            // of which frame the arming landed in. Also mutates Tracked every frame
            // so FixedUpdate/Update can observe Changed<Tracked>.
            app.add_systems(
                PreUpdate,
                |mut probe: ResMut<RemoveProbe>,
                 mut commands: Commands,
                 markers: Query<Entity, With<Marker>>,
                 mut tracked: Query<&mut Tracked>| {
                    if probe.phase == 1 {
                        let mut removed_any = false;
                        for e in markers.iter() {
                            commands.entity(e).remove::<Marker>();
                            removed_any = true;
                        }
                        if removed_any {
                            probe.phase = 2;
                        }
                    }
                    for mut t in tracked.iter_mut() {
                        t.0 += 1;
                    }
                },
            );
            app.add_systems(
                FixedUpdate,
                move |changed: Query<Entity, Changed<Tracked>>| {
                    if !changed.is_empty() {
                        fsc_sys.increment();
                    }
                },
            );
            app.add_systems(
                Update,
                (
                    move |changed: Query<Entity, Changed<Tracked>>| {
                        if !changed.is_empty() {
                            usc_sys.increment();
                        }
                    },
                    |mut probe: ResMut<RemoveProbe>, mut removed: RemovedComponents<Marker>| {
                        let count = removed.read().count();
                        if probe.phase == 2 {
                            probe.removed_activation = count;
                            probe.phase = 3;
                        } else if probe.phase == 3 {
                            probe.removed_next = count;
                            probe.phase = 4;
                        }
                    },
                ),
            );
        })
        .await;

        // Changed<T>: over 10 frames both FixedUpdate and Update must observe
        // Changed<Tracked> (PreUpdate mutates it every frame).
        app.updates(10).await;

        assert!(
            update_saw_changed.get() > 0,
            "Update must see Changed<Tracked>: PreUpdate mutates it every frame"
        );
        assert!(
            fixed_saw_changed.get() > 0,
            "FixedUpdate must see Changed<Tracked> at least once in 10 frames -- \
             if zero the split driver clears trackers between prefix and FixedMain"
        );

        // RemovedComponents: a few frames are enough -- the phase machine is
        // robust to which frame the arming lands in.
        app.with_world_mut(|world| {
            world.resource_mut::<RemoveProbe>().phase = 1;
        });
        app.updates(5).await;

        let (removed_act, removed_next) = app.with_world(|world| {
            let p = world.resource::<RemoveProbe>();
            (p.removed_activation, p.removed_next)
        });
        assert_eq!(
            removed_act, 1,
            "exactly one removal on the activation frame"
        );
        assert_eq!(
            removed_next, 0,
            "zero removals on the next frame -- clear_trackers ran at end of render frame"
        );

        app.cleanup().await;
    })
}

/// Native ordering end-to-end: First runs before FixedUpdate, FixedUpdate before
/// Update, all within one render frame driven by real Godot clocks.
///
/// Uses per-frame boolean tracking instead of a cumulative log, so partial
/// frames accumulated across multiple calls don't corrupt the ordering check.
/// Over 10 frames at least one must have all three schedules in order.
#[itest(async)]
fn test_native_ordering_end_to_end(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct OrderProbe {
            // reset each frame by First
            first_ran: bool,
            fixed_ran_after_first: bool,
            // accumulated across frames
            first_before_fixed_ok: bool,
            fixed_before_update_ok: bool,
            frame_with_all_three: bool,
        }

        let mut app = TestApp::new(&ctx_clone, |app| {
            app.init_resource::<OrderProbe>();
            app.add_systems(First, |mut p: ResMut<OrderProbe>| {
                p.first_ran = true;
                p.fixed_ran_after_first = false;
            });
            app.add_systems(FixedUpdate, |mut p: ResMut<OrderProbe>| {
                if p.first_ran {
                    p.fixed_ran_after_first = true;
                    p.first_before_fixed_ok = true;
                }
            });
            app.add_systems(Update, |mut p: ResMut<OrderProbe>| {
                if p.first_ran && p.fixed_ran_after_first {
                    p.fixed_before_update_ok = true;
                    p.frame_with_all_three = true;
                }
                p.first_ran = false;
                p.fixed_ran_after_first = false;
            });
        })
        .await;

        app.updates(10).await;

        let (first_before_fixed, fixed_before_update, all_three) = app.with_world(|world| {
            let p = world.resource::<OrderProbe>();
            (
                p.first_before_fixed_ok,
                p.fixed_before_update_ok,
                p.frame_with_all_three,
            )
        });

        assert!(
            all_three,
            "at least one render frame must have First → FixedUpdate → Update in order"
        );
        assert!(first_before_fixed, "First must run before FixedUpdate");
        assert!(fixed_before_update, "FixedUpdate must run before Update");

        app.cleanup().await;
    })
}

/// First (prefix) and Update (suffix) each run exactly once per render frame, so
/// their run counts must advance in lockstep. Comparing the two counters (sampled
/// at Last, a fixed in-frame point) sidesteps the async-boundary slop an absolute
/// count check suffers.
#[itest(async)]
fn test_prefix_runs_every_render_frame(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        let first = Counter::new();
        let update = Counter::new();
        let mismatch = Counter::new();
        let fc = first.clone();
        let uc = update.clone();
        let mc = mismatch.clone();

        let mut app = TestApp::new(&ctx_clone, move |app| {
            #[derive(Resource)]
            struct Counters {
                first: Counter,
                update: Counter,
                mismatch: Counter,
            }

            app.insert_resource(Counters {
                first: fc.clone(),
                update: uc.clone(),
                mismatch: mc.clone(),
            });
            app.add_systems(First, |c: Res<Counters>| c.first.increment());
            app.add_systems(Update, |c: Res<Counters>| c.update.increment());
            app.add_systems(Last, |c: Res<Counters>| {
                if c.first.get() != c.update.get() {
                    c.mismatch.increment();
                }
            });
        })
        .await;

        let update_start = update.get();
        app.updates(5).await;
        let update_delta = update.get() - update_start;

        assert_eq!(
            mismatch.get(),
            0,
            "First (prefix) must run once per render frame, as often as Update (suffix): \
             prefix_done must reset each frame -- saw {} frames where the counts diverged",
            mismatch.get()
        );
        assert!(
            update_delta >= 4,
            "render frames must actually run (suffix advanced {update_delta} over 5 frames)"
        );

        app.cleanup().await;
    })
}

/// After tearing down one TestApp and initializing another, the new app's Startup
/// systems must run -- pinning that do_initialize() properly resets internal state.
#[itest(async)]
fn test_reinit_runs_startup(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        let mut app1 = TestApp::new(&ctx_clone, |_| {}).await;
        app1.cleanup().await;

        let counter = Counter::new();
        let c = counter.clone();

        let mut app2 = TestApp::new(&ctx_clone, move |app| {
            #[derive(Resource)]
            struct StartupRan(Counter);

            app.insert_resource(StartupRan(c.clone()));
            app.add_systems(Startup, |s: Res<StartupRan>| s.0.increment());
        })
        .await;

        let val = counter.get();
        assert!(
            val > 0,
            "Startup must run on the re-initialized app; counter = {val}"
        );

        app2.cleanup().await;
    })
}

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

/// BeforeFixedMainLoop anchor must fire under real Godot physics driving -- confirms
/// that hosting RunFixedMainLoop makes the standard anchor sets live (needed for
/// leafwing-class plugins).
#[itest(async)]
fn test_fixed_main_loop_anchor_runs_on_physics_frames(
    ctx: &TestContext,
) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();

    godot::task::spawn(async move {
        let counter = Counter::new();
        let c = counter.clone();

        let mut app = TestApp::new(&ctx_clone, move |app| {
            #[derive(Resource)]
            struct AnchorCounter(Counter);

            app.insert_resource(AnchorCounter(c.clone()));
            app.add_systems(
                bevy::app::RunFixedMainLoop,
                (|c: Res<AnchorCounter>| c.0.increment())
                    .in_set(bevy::app::RunFixedMainLoopSystems::BeforeFixedMainLoop),
            );
        })
        .await;

        let start = counter.get();
        app.updates(10).await;
        let end = counter.get();

        assert!(
            end > start,
            "BeforeFixedMainLoop anchor should run under physics driving, got {start} -> {end}"
        );

        app.cleanup().await;
    })
}

/// Time<Virtual> must still advance in Update even though the fixed loop is driven
/// from Godot's physics step (guards against the host starving time_system).
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

/// A value written in Update is visible after a SINGLE update().await. Fails if
/// update() resolves before the suffix has run (e.g. if it waits on process_frame
/// instead of the bevy_frame_complete signal).
#[cfg(feature = "test-frame-signal")]
#[itest(async)]
fn test_update_returns_after_full_frame(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx_clone = ctx.clone();
    godot::task::spawn(async move {
        #[derive(Resource, Default)]
        struct Seen(u32);
        let mut app = TestApp::new(&ctx_clone, |app| {
            app.init_resource::<Seen>();
            app.add_systems(Update, |mut s: ResMut<Seen>| s.0 += 1);
        })
        .await;
        let before = app.with_world(|w| w.resource::<Seen>().0);
        app.update().await;
        let after = app.with_world(|w| w.resource::<Seen>().0);
        assert_eq!(
            after,
            before + 1,
            "exactly one Update ran and is visible after one update()"
        );
        app.cleanup().await;
    })
}

// Panic-safety note: process() emits bevy_frame_complete BEFORE resume_unwind and
// even when app == None, so a panicking frame resumes its awaiter (fast clean
// failure) instead of hanging the suite to --quit-after. We can't ship a
// deliberate-panic itest to guard this -- the harness flags any system panic via
// has_godot_task_panicked, so such a test can never be green. The contract is
// structural (see the emit in app.rs) and the app==None branch is exercised by
// every test's cleanup(), which awaits the signal after teardown.
