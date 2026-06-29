//! Deterministic, FFI-free validation of the TwoWay multi-step clobber fix.
//!
//! The bug: the Godot->Bevy read used to run once per render frame (PreUpdate),
//! but the whole-transform write runs once per physics step (FixedLast). On a
//! frame with 2+ physics steps, steps 2..N wrote without a fresh read, so the
//! step's whole-transform write dragged a stale Bevy value over any axis a Godot
//! physics-clock author moved between steps -- silently clobbering it.
//!
//! The fix inverts the read cadence: FixedFirst reads every
//! physics step (matching the FixedLast write), so each step gets a fresh per-axis
//! read before its write. PreUpdate becomes the 0-tick fallback -- it runs only on
//! render frames with zero physics steps, where the Main prefix is the `_process`
//! fallback (gated by `prefix_ran_in_process_fallback`).
//!
//! These tests model the Godot node as a `GodotNode(Transform)` component (no FFI)
//! and wire stub systems that call the real merge/value-gate math
//! (`merge_godot_into_bevy`, `write_needed`) and the real run condition
//! (`prefix_ran_in_process_fallback`), driven per step through the real production
//! path (`run_physics_step`, which clears `ProcessFallbackPrefix`) plus a `frame()`
//! driver that publishes the flag for the `_process` fallback exactly like
//! `app.rs`. The structural test at the bottom asserts the real plugin registers
//! the read in both PreUpdate and FixedFirst.

use bevy_app::{App, FixedFirst, FixedLast, FixedUpdate, PreUpdate};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ScheduleLabel;
use bevy_time::TimePlugin;
use bevy_transform::components::Transform;
use std::time::Duration;

use crate::plugins::fixed_schedule::{
    ProcessFallbackPrefix, host_fixed_main_loop, prefix_ran_in_process_fallback, run_main_suffix,
    run_physics_step, run_preamble,
};
use crate::plugins::transforms::sync_systems::{merge_godot_into_bevy, write_needed};
use crate::plugins::transforms::{
    GodotTransformConfig, GodotTransformSyncPlugin, TransformSyncMetadata, TransformSyncMode,
};

const DT: Duration = Duration::from_nanos(16_666_667); // ~1/60 s
const STEP_X: f32 = 1.0; // Bevy authors x by this much each physics step

/// Stand-in for the Godot-side transform. The read pulls from here, the write
/// pushes to here -- no FFI.
#[derive(Component)]
struct GodotNode(Transform);

/// Counts read-system invocations per schedule, proving the cadence partition:
/// FixedFirst reads every physics step, PreUpdate reads only on a 0-tick
/// (process-fallback) frame.
#[derive(Resource, Default)]
struct ReadCount {
    preupdate: u32,
    fixedfirst: u32,
}

// Run conditions mirror plugin.rs's private `transform_sync_twoway_enabled` /
// `transform_sync_enabled` so the stub wiring matches production gating exactly.
fn twoway_read_enabled(config: Res<GodotTransformConfig>) -> bool {
    config.sync_mode == TransformSyncMode::TwoWay
}

fn sync_write_enabled(config: Res<GodotTransformConfig>) -> bool {
    config.sync_mode != TransformSyncMode::Disabled
}

/// Shared Godot->Bevy merge body for both read stubs: calls the real per-axis
/// merge. Idempotent via the value-shadow guard, so a duplicate read doesn't trip
/// Changed.
fn merge_reads(q: &mut Query<(&mut Transform, &GodotNode, &mut TransformSyncMetadata)>) {
    for (mut transform, node, mut meta) in q.iter_mut() {
        merge_godot_into_bevy(&mut transform, &node.0, &mut meta.shadow);
    }
}

/// PreUpdate read stub: the 0-tick fallback read. Bumps only `preupdate`.
fn read_stub_preupdate(
    mut q: Query<(&mut Transform, &GodotNode, &mut TransformSyncMetadata)>,
    mut count: ResMut<ReadCount>,
) {
    count.preupdate += 1;
    merge_reads(&mut q);
}

/// FixedFirst read stub: the per-step read. Bumps only `fixedfirst`.
fn read_stub_fixedfirst(
    mut q: Query<(&mut Transform, &GodotNode, &mut TransformSyncMetadata)>,
    mut count: ResMut<ReadCount>,
) {
    count.fixedfirst += 1;
    merge_reads(&mut q);
}

/// Bevy->Godot write stub: the real value gate, then a whole-transform push --
/// exactly the shape of the production write (no per-axis write).
fn write_stub(
    mut q: Query<(&Transform, &mut GodotNode, &mut TransformSyncMetadata), Changed<Transform>>,
) {
    for (t, mut node, mut meta) in q.iter_mut() {
        if write_needed(t, &meta.shadow) {
            node.0 = *t;
            meta.shadow = *t;
        }
    }
}

/// Bevy author: advances x every physics step (runs in FixedUpdate, per step).
/// Keeps the write firing each step so a stale-x clobber can manifest.
fn author_stub(mut q: Query<&mut Transform, With<GodotNode>>) {
    for mut t in q.iter_mut() {
        t.translation.x += STEP_X;
    }
}

/// Build an app wired like the production TwoWay path: PreUpdate read gated by
/// `prefix_ran_in_process_fallback` (the 0-tick fallback), FixedFirst read every
/// physics step, FixedLast write, FixedUpdate author. `with_fixed_first`
/// lets the control test omit the per-step read to prove the bug. Returns the app
/// and the single synced entity.
fn wired_app(mode: TransformSyncMode, with_fixed_first: bool) -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(TimePlugin);
    host_fixed_main_loop(&mut app);
    app.insert_resource(GodotTransformConfig { sync_mode: mode });
    app.init_resource::<ReadCount>();

    app.add_systems(
        PreUpdate,
        read_stub_preupdate
            .run_if(twoway_read_enabled)
            .run_if(prefix_ran_in_process_fallback),
    );
    if with_fixed_first {
        app.add_systems(FixedFirst, read_stub_fixedfirst.run_if(twoway_read_enabled));
    }
    app.add_systems(FixedLast, write_stub.run_if(sync_write_enabled));
    app.add_systems(FixedUpdate, author_stub);

    let entity = app
        .world_mut()
        .spawn((
            Transform::default(),
            GodotNode(Transform::default()),
            TransformSyncMetadata::default(),
        ))
        .id();
    (app, entity)
}

/// Drive one render frame with `n_steps` physics steps through the real production
/// per-step path (`run_physics_step`), mirroring `app.rs`:
/// - each physics step runs `run_physics_step`, which clears `ProcessFallbackPrefix`
///   (a step's prefix is never the process fallback) and runs the preamble + fixed
///   main
/// - the end-of-frame `_process` publishes `ProcessFallbackPrefix = !prefix_done`
///   (true only on an idle 0-step frame), runs the prefix fallback, then the suffix
///   and `clear_trackers`
///
/// Publishing the flag in the driver -- exactly as `app.rs::process()` does -- is
/// what lets the PreUpdate read fire on a 0-tick frame and stay suppressed on tick
/// frames; a regression in that publish breaks these tests, not just the itest.
///
/// `godot_author(world, step)` runs before each step's fixed main, letting a test
/// move the Godot-side node between steps like a GDScript `_physics_process`.
fn frame(app: &mut App, n_steps: u32, mut godot_author: impl FnMut(&mut World, u32)) {
    #[derive(Resource)]
    struct StartupRan;

    let world = app.world_mut();
    let need_startup = !world.contains_resource::<StartupRan>();

    // prefix_done mirrors app.rs's prefix_done_this_frame: true once the first
    // physics step runs the prefix.
    let mut prefix_done = false;
    for step in 0..n_steps {
        godot_author(world, step);
        run_physics_step(world, need_startup && step == 0, !prefix_done, DT);
        prefix_done = true;
    }

    // End-of-frame _process(): publish the process-fallback flag (true only on an
    // idle 0-step frame), run the prefix if no physics step did, then the suffix
    // and clear_trackers.
    if let Some(mut f) = world.get_resource_mut::<ProcessFallbackPrefix>() {
        f.0 = !prefix_done;
    }
    run_preamble(world, need_startup && n_steps == 0, !prefix_done);
    run_main_suffix(world);
    world.clear_trackers();

    if need_startup {
        world.insert_resource(StartupRan);
    }
}

fn node_translation(app: &App, entity: Entity) -> bevy_math::Vec3 {
    app.world().get::<GodotNode>(entity).unwrap().0.translation
}

fn bevy_translation(app: &App, entity: Entity) -> bevy_math::Vec3 {
    app.world().get::<Transform>(entity).unwrap().translation
}

fn close(a: f32, b: f32) -> bool {
    (a - b).abs() < 1e-4
}

/// Core regression: a 2-step frame where a Godot physics-clock author moves the
/// node's y between steps. The FixedFirst read runs every step, so step 2
/// pulls the fresh y before its whole-transform write -- the Godot y survives and
/// the Bevy x advances both steps. Fails on pre-fix code (no FixedFirst read),
/// where step 2's write drags a stale y=0 over the Godot move.
#[test]
fn twoway_multistep_does_not_clobber_godot_axis() {
    let (mut app, entity) = wired_app(TransformSyncMode::TwoWay, true);

    frame(&mut app, 2, |world, step| {
        if step >= 1 {
            world.get_mut::<GodotNode>(entity).unwrap().0.translation.y += 1.0;
        }
    });

    let node = node_translation(&app, entity);
    assert!(
        close(node.y, 1.0),
        "Godot-authored y (+1 between steps) must survive the step-2 write, got {}",
        node.y
    );
    assert!(
        close(node.x, 2.0 * STEP_X),
        "Bevy x must advance once per step (2 steps), got {}",
        node.x
    );
}

/// Control that proves the scenario genuinely triggers the bug: the same 2-step
/// frame, but the FixedFirst read is omitted. With PreUpdate now gated to 0-tick
/// frames, a 2-step frame reads nothing, so the step-2 write clobbers the Godot y
/// back to 0 -- confirming the regression test above is not vacuous.
#[test]
fn twoway_multistep_clobbers_without_fixed_first_read() {
    let (mut app, entity) = wired_app(TransformSyncMode::TwoWay, false);

    frame(&mut app, 2, |world, step| {
        if step >= 1 {
            world.get_mut::<GodotNode>(entity).unwrap().0.translation.y += 1.0;
        }
    });

    let node = node_translation(&app, entity);
    assert!(
        close(node.y, 0.0),
        "without the FixedFirst read the step-2 write clobbers the Godot y to 0, got {}",
        node.y
    );
    assert!(
        close(node.x, 2.0 * STEP_X),
        "Bevy x still advances both steps, got {}",
        node.x
    );
}

/// N-step frame with a distinct y delta in every gap between steps: each step's
/// FixedFirst read picks up that gap's move, so the final y equals the sum of all
/// deltas (none clobbered). Fails on pre-fix code, where intermediate moves are
/// dragged away.
#[test]
fn twoway_nstep_accumulates_every_godot_move() {
    let (mut app, entity) = wired_app(TransformSyncMode::TwoWay, true);

    // 4 steps -> 3 gaps; distinct delta per gap.
    let deltas = [2.0_f32, 3.0, 5.0];
    let expected_y: f32 = deltas.iter().sum();

    frame(&mut app, 4, |world, step| {
        if step >= 1 {
            world.get_mut::<GodotNode>(entity).unwrap().0.translation.y +=
                deltas[(step - 1) as usize];
        }
    });

    let node = node_translation(&app, entity);
    assert!(
        close(node.y, expected_y),
        "final y must equal the sum of per-gap Godot moves ({expected_y}), got {}",
        node.y
    );
    assert!(
        close(node.x, 4.0 * STEP_X),
        "Bevy x must advance once per step (4 steps), got {}",
        node.x
    );
}

/// Brownfield idle path: after a Godot move, a 0-step render frame must still pull
/// the value via the PreUpdate fallback read (FixedFirst never runs with 0 steps).
/// This is the test that proves the new `prefix_ran_in_process_fallback` gate fires
/// on a 0-tick frame -- it depends on `frame()` publishing the flag.
#[test]
fn idle_frame_reads_via_preupdate_fallback() {
    let (mut app, entity) = wired_app(TransformSyncMode::TwoWay, true);

    app.world_mut()
        .get_mut::<GodotNode>(entity)
        .unwrap()
        .0
        .translation
        .y = 5.0;
    frame(&mut app, 0, |_, _| {});

    let bevy = bevy_translation(&app, entity);
    assert!(
        close(bevy.y, 5.0),
        "the PreUpdate read must pull the Godot y into Bevy on an idle frame, got {}",
        bevy.y
    );
}

/// Per-schedule read partition: FixedFirst reads once per physics step;
/// PreUpdate reads only on a 0-tick (process-fallback) frame. The `preupdate == 0`
/// on tick frames is the explicit "PreUpdate does not fire on a tick frame" check.
#[test]
fn read_count_partitions_steps() {
    let (mut app, _entity) = wired_app(TransformSyncMode::TwoWay, true);

    *app.world_mut().resource_mut::<ReadCount>() = ReadCount::default();
    frame(&mut app, 1, |_, _| {});
    {
        let rc = app.world().resource::<ReadCount>();
        assert_eq!(
            rc.preupdate, 0,
            "tick frame: PreUpdate read suppressed (gated to 0-tick frames)"
        );
        assert_eq!(
            rc.fixedfirst, 1,
            "tick frame: FixedFirst reads once for the single step"
        );
    }

    *app.world_mut().resource_mut::<ReadCount>() = ReadCount::default();
    frame(&mut app, 0, |_, _| {});
    {
        let rc = app.world().resource::<ReadCount>();
        assert_eq!(
            rc.preupdate, 1,
            "idle frame: PreUpdate 0-tick fallback reads once"
        );
        assert_eq!(rc.fixedfirst, 0, "idle frame: FixedFirst never runs");
    }

    *app.world_mut().resource_mut::<ReadCount>() = ReadCount::default();
    frame(&mut app, 2, |_, _| {});
    {
        let rc = app.world().resource::<ReadCount>();
        assert_eq!(
            rc.preupdate, 0,
            "two-step frame: PreUpdate suppressed on a tick frame"
        );
        assert_eq!(
            rc.fixedfirst, 2,
            "two-step frame: one FixedFirst read per step"
        );
    }
}

/// OneWay regression: with sync disabled in the Godot->Bevy direction, neither
/// read body runs at any cadence, while the FixedLast write still fires per step.
#[test]
fn oneway_never_reads_but_still_writes_per_step() {
    let (mut app, entity) = wired_app(TransformSyncMode::OneWay, true);

    *app.world_mut().resource_mut::<ReadCount>() = ReadCount::default();
    frame(&mut app, 3, |world, _step| {
        // A Godot move that OneWay deliberately ignores.
        world.get_mut::<GodotNode>(entity).unwrap().0.translation.y += 1.0;
    });

    let rc = app.world().resource::<ReadCount>();
    assert_eq!(
        rc.preupdate, 0,
        "OneWay must never run the PreUpdate Godot->Bevy read"
    );
    assert_eq!(
        rc.fixedfirst, 0,
        "OneWay must never run the FixedFirst Godot->Bevy read"
    );
    let node = node_translation(&app, entity);
    assert!(
        close(node.x, 3.0 * STEP_X),
        "the write must still fire per step in OneWay, got x={}",
        node.x
    );
}

// ── structural wiring test (no FFI) ────────────────────────────────────────────

/// Returns true if any system in `label`'s schedule has a name containing `needle`.
/// Initializes the schedule (builds the executable) without running it, so no
/// system body or FFI is touched. Relies on bevy_utils' `debug` feature (enabled
/// by this crate) for real system names.
fn schedule_has_system(app: &mut App, label: impl ScheduleLabel, needle: &str) -> bool {
    // schedule_scope removes only the named schedule (not the whole Schedules
    // resource), so initialize() can run with &mut World without tripping the
    // "Schedules inserted during resource_scope" panic.
    app.world_mut().schedule_scope(label, |world, schedule| {
        schedule
            .initialize(world)
            .expect("schedule should initialize");
        schedule
            .systems()
            .expect("schedule should be initialized")
            .any(|(_, system)| system.name().to_string().contains(needle))
    })
}

/// The real plugin must register the Godot->Bevy read in both PreUpdate and
/// FixedFirst, and the Bevy->Godot write in FixedLast. Guards that the production
/// registration exists in all three schedules (the deterministic tests above use
/// stub wiring); it checks the read is registered, not which run-condition gates
/// which schedule.
#[test]
fn read_registered_in_preupdate_and_fixedfirst_write_in_fixedlast() {
    let mut app = App::new();
    app.add_plugins(TimePlugin);
    host_fixed_main_loop(&mut app);
    app.add_plugins(GodotTransformSyncPlugin {
        sync_mode: TransformSyncMode::TwoWay,
        auto_sync: true,
    });
    // Ensure the schedules exist so a missing registration is a clean assertion
    // failure rather than an `expect` panic on a never-created schedule.
    app.init_schedule(PreUpdate);
    app.init_schedule(FixedFirst);
    app.init_schedule(FixedLast);

    assert!(
        schedule_has_system(&mut app, PreUpdate, "pre_update_godot_transforms"),
        "Godot->Bevy read must be registered in PreUpdate"
    );
    assert!(
        schedule_has_system(&mut app, FixedFirst, "pre_update_godot_transforms"),
        "Godot->Bevy read must be registered in FixedFirst (per-step, every step)"
    );
    assert!(
        schedule_has_system(&mut app, FixedLast, "post_update_godot_transforms"),
        "Bevy->Godot write must be registered in FixedLast"
    );
}
