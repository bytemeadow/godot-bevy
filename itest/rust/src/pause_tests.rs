//! Pause coherence (Leg B). Two pause surfaces must agree:
//!
//! - `SceneTree.paused` is mirrored onto `Time<Virtual>` (edge-triggered, one-way), and
//! - the Godot-driven fixed schedule is frozen whenever `Time<Virtual>` is paused.
//!
//! Because `BevyApp` is now `process_mode = ALWAYS`, both callbacks keep firing under a
//! tree-pause, so the frame signal still resolves and these awaits never deadlock. A
//! regression to PAUSABLE would instead hang the await forever: the itest runner is
//! sequential and the drop guard only runs on unwind, so the hang stalls the whole suite
//! until `--quit-after` fires -- it surfaces as a global timeout, not a single red test.
//! `SceneTree` and `Time<Virtual>` are process-global, so every test resets pause through a
//! drop guard -- a failed assert can't leave the tree paused for the next test.
//!
//! Counting rule: physics_process still fires under pause, so the frame signal's step
//! count is NOT a measure of FixedUpdate runs. We count FixedUpdate runs with our own
//! counter resource instead.

use bevy::prelude::*;
use godot::obj::NewAlloc;
use godot::prelude::*;
use godot_bevy::prelude::*;
use godot_bevy_test::prelude::*;

/// Restores `SceneTree.paused` to `false` on drop, even if an assertion unwinds.
struct ResetPause(Gd<godot::classes::SceneTree>);

impl Drop for ResetPause {
    fn drop(&mut self) {
        self.0.set_pause(false);
    }
}

fn tree(ctx: &TestContext) -> Gd<godot::classes::SceneTree> {
    ctx.scene_tree.get_tree()
}

#[derive(Resource)]
struct FixedTicks(Counter);

#[derive(Resource)]
struct UpdateTicks(Counter);

/// Build an app with a FixedUpdate counter and a (non-delta-scaled) Update counter, and
/// return handles to read them from the test.
async fn app_with_counters(ctx: &TestContext) -> (TestApp, Counter, Counter) {
    let fixed = Counter::new();
    let update = Counter::new();
    let (f, u) = (fixed.clone(), update.clone());

    let app = TestApp::new(ctx, move |app| {
        app.insert_resource(FixedTicks(f));
        app.insert_resource(UpdateTicks(u));
        app.add_systems(FixedUpdate, |t: Res<FixedTicks>| t.0.increment());
        app.add_systems(Update, |t: Res<UpdateTicks>| t.0.increment());
    })
    .await;

    (app, fixed, update)
}

/// (a)+(b) A tree-pause freezes FixedUpdate wholesale while a non-delta-scaled Update
/// system keeps running -- the breaking change from `process_mode = ALWAYS`.
#[itest(async)]
fn test_tree_pause_freezes_fixed_runs_update(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, fixed, update) = app_with_counters(&ctx).await;
        let _reset = ResetPause(tree(&ctx));

        // Baseline: both schedules advance while unpaused.
        app.updates(5).await;
        assert!(fixed.get() > 0, "FixedUpdate should advance before pause");
        assert!(update.get() > 0, "Update should advance before pause");

        tree(&ctx).set_pause(true);
        app.updates(3).await; // let the rising edge propagate and any in-flight step drain

        let fixed_at_pause = fixed.get();
        let update_at_pause = update.get();
        app.updates(10).await;

        assert_eq!(
            fixed.get(),
            fixed_at_pause,
            "FixedUpdate must be frozen under SceneTree.paused"
        );
        assert!(
            update.get() > update_at_pause,
            "Update must keep running under SceneTree.paused (process_mode = ALWAYS)"
        );

        app.cleanup().await;
    })
}

/// (c) Bevy-only pause: `Time<Virtual>::pause()` with the tree UNPAUSED freezes FixedUpdate
/// (the driver keys on `Time<Virtual>`) while Update runs, and the edge mirror is one-way,
/// so `SceneTree.paused` stays false.
#[itest(async)]
fn test_bevy_only_pause_freezes_fixed_tree_untouched(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, fixed, update) = app_with_counters(&ctx).await;
        // Guard is a safety net: this test never pauses the tree, but a bug could.
        let _reset = ResetPause(tree(&ctx));

        app.updates(5).await;

        app.with_world_mut(|w| w.resource_mut::<Time<Virtual>>().pause());
        app.updates(3).await; // settle

        let fixed_at_pause = fixed.get();
        let update_at_pause = update.get();
        app.updates(10).await;

        assert_eq!(
            fixed.get(),
            fixed_at_pause,
            "FixedUpdate must be frozen by a Bevy-only Time<Virtual>::pause()"
        );
        assert!(
            update.get() > update_at_pause,
            "Update must keep running under a Bevy-only pause"
        );
        assert!(
            !tree(&ctx).is_paused(),
            "the mirror is one-way (tree -> virtual); a Bevy pause must not touch the tree"
        );

        app.cleanup().await;
    })
}

/// (d) Edge no-clobber: a user's `Time<Virtual>::pause()` is not cleared while the tree
/// stays unpaused across many frames. A blind per-frame `set_paused(tree.is_paused())`
/// would unpause virtual every frame; the edge trigger must leave it alone.
#[itest(async)]
fn test_edge_mirror_does_not_clobber_user_pause(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, fixed, _update) = app_with_counters(&ctx).await;
        let _reset = ResetPause(tree(&ctx));

        app.updates(3).await;
        app.with_world_mut(|w| w.resource_mut::<Time<Virtual>>().pause());
        app.updates(3).await; // settle

        let fixed_at_pause = fixed.get();
        app.updates(20).await; // long window: an every-frame overwrite would leak here

        assert!(
            app.with_world(|w| w.resource::<Time<Virtual>>().is_paused()),
            "user Time<Virtual>::pause() must survive across frames (edge, not per-frame overwrite)"
        );
        assert_eq!(
            fixed.get(),
            fixed_at_pause,
            "FixedUpdate must stay frozen the whole window (pause never clobbered)"
        );
        assert!(
            !tree(&ctx).is_paused(),
            "tree must stay unpaused throughout"
        );

        app.cleanup().await;
    })
}

/// (d2) Provenance: a user's `Time<Virtual>::pause()` survives a full tree pause/unpause
/// cycle. The mirror resumes only the pause it applied itself, so opening then closing a
/// SceneTree pause menu over an existing Bevy pause (e.g. a cutscene) must not clear it.
#[itest(async)]
fn test_user_pause_survives_tree_pause_cycle(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, fixed, _update) = app_with_counters(&ctx).await;
        let _reset = ResetPause(tree(&ctx));

        // User pauses virtual time; the tree keeps running.
        app.updates(3).await;
        app.with_world_mut(|w| w.resource_mut::<Time<Virtual>>().pause());
        app.updates(3).await; // settle
        let frozen = fixed.get();

        // A tree pause/unpause cycle rides over the user's pause without claiming it.
        tree(&ctx).set_pause(true);
        app.updates(3).await;
        tree(&ctx).set_pause(false);
        app.updates(5).await;

        assert!(
            app.with_world(|w| w.resource::<Time<Virtual>>().is_paused()),
            "user pause must survive a tree pause/unpause cycle (mirror resumes only its own pause)"
        );
        assert_eq!(
            fixed.get(),
            frozen,
            "FixedUpdate must stay frozen -- the tree cycle must not resume the user's pause"
        );

        app.cleanup().await;
    })
}

/// (e) Unpausing the tree resumes FixedUpdate (falling edge -> Time<Virtual>::unpause()).
#[itest(async)]
fn test_unpause_resumes_fixedupdate(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let (mut app, fixed, _update) = app_with_counters(&ctx).await;
        let _reset = ResetPause(tree(&ctx));

        tree(&ctx).set_pause(true);
        app.updates(3).await; // settle paused
        let frozen = fixed.get();
        app.updates(5).await;
        assert_eq!(fixed.get(), frozen, "FixedUpdate frozen while paused");

        tree(&ctx).set_pause(false);
        app.updates(3).await; // settle unpaused
        let resumed_base = fixed.get();
        app.updates(10).await;
        assert!(
            fixed.get() > resumed_base,
            "FixedUpdate must resume after the tree is unpaused ({resumed_base} -> {})",
            fixed.get()
        );

        app.cleanup().await;
    })
}

/// (f) Transform sync (TwoWay) is frozen under pause: a Godot-side node move is NOT read
/// into Bevy while paused (the FixedFirst read rides inside the frozen fixed driver), then
/// is picked up once the tree is unpaused. Exercises decision #2's driver gate; the
/// PreUpdate 0-tick fallback gate (decision #4) is not reachable here because the harness
/// runs `--fixed-fps 60`, so every render frame runs exactly one physics step.
#[itest(async)]
fn test_transform_read_frozen_under_pause(ctx: &TestContext) -> godot::task::TaskHandle {
    let ctx = ctx.clone();
    godot::task::spawn(async move {
        let mut node = godot::classes::Node2D::new_alloc();
        node.set_name("PausedTransformNode");
        node.set_position(Vector2::new(0.0, 0.0));
        ctx.scene_tree.clone().add_child(&node);
        let node_id = node.instance_id();

        let mut app = TestApp::new(&ctx, |app| {
            app.add_plugins(GodotTransformSyncPlugin::default());
            app.insert_resource(GodotTransformConfig::two_way());
        })
        .await;
        let _reset = ResetPause(tree(&ctx));

        let entity = app.entity_for_node(node_id).expect("entity for node");

        // Settle at the origin, then record the baseline Bevy x.
        app.physics_update().await;
        let x_before = app.with_world(|w| w.get::<Transform>(entity).unwrap().translation.x);

        // Pause, then author x purely from Godot. The read is frozen, so Bevy must not see it.
        tree(&ctx).set_pause(true);
        app.updates(3).await; // settle pause
        node.set_position(Vector2::new(50.0, 0.0));
        for _ in 0..5 {
            app.physics_update().await;
        }

        let x_paused = app.with_world(|w| w.get::<Transform>(entity).unwrap().translation.x);
        assert!(
            (x_paused - x_before).abs() < 0.1,
            "a Godot move must not read into Bevy while paused, expected ~{x_before:.1}, got {x_paused:.1}"
        );

        // Unpause: the read resumes and picks up the Godot position.
        tree(&ctx).set_pause(false);
        for _ in 0..5 {
            app.physics_update().await;
        }
        let x_resumed = app.with_world(|w| w.get::<Transform>(entity).unwrap().translation.x);
        assert!(
            (x_resumed - 50.0).abs() < 0.5,
            "after unpause the Godot position must sync into Bevy, expected ~50.0, got {x_resumed:.1}"
        );

        app.cleanup().await;
        node.free();
    })
}
