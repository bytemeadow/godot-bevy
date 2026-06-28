//! Deterministic, FFI-free validation of the TwoWay multi-step clobber fix.
//!
//! The bug: the Godot->Bevy read runs once per render frame (PreUpdate), but the
//! whole-transform write runs once per physics step (FixedLast). On a frame with
//! 2+ physics steps, steps 2..N write without a fresh read, so the step's
//! whole-transform write drags a stale Bevy value over any axis a Godot
//! physics-clock author moved between steps -- silently clobbering it.
//!
//! The fix keeps the PreUpdate read (step 1,
//! the Update suffix, idle 0-step frames) and adds the same read to FixedFirst,
//! gated by `not_first_fixed_step` so steps 2..N each get a fresh per-axis read
//! before their FixedLast write. Step 1 and idle frames still do exactly one read.
//!
//! These tests model the Godot node as a `GodotNode(Transform)` component (no FFI)
//! and wire stub systems that call the real merge/value-gate math
//! (`merge_godot_into_bevy`, `write_needed`) and the real run condition
//! (`not_first_fixed_step`), driven per step through the real production path
//! (`run_physics_step`, which publishes `FixedStepFirstOfFrame`), reproducing the
//! production schedule wiring without Godot. The structural test at the bottom
//! asserts the real plugin registers the read in both PreUpdate and FixedFirst.

use bevy_app::{App, FixedFirst, FixedLast, FixedUpdate, PreUpdate};
use bevy_ecs::prelude::*;
use bevy_ecs::schedule::ScheduleLabel;
use bevy_time::TimePlugin;
use bevy_transform::components::Transform;
use std::time::Duration;

use crate::plugins::fixed_schedule::{
    host_fixed_main_loop, not_first_fixed_step, run_main_suffix, run_physics_step, run_preamble,
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

/// Counts read-system invocations (PreUpdate + FixedFirst), to prove the dedup
/// partition: 1-step and idle frames read once, multi-step frames read per step.
#[derive(Resource, Default)]
struct ReadCount(u32);

// Run conditions mirror plugin.rs's private `transform_sync_twoway_enabled` /
// `transform_sync_enabled` so the stub wiring matches production gating exactly.
fn twoway_read_enabled(config: Res<GodotTransformConfig>) -> bool {
    config.sync_mode == TransformSyncMode::TwoWay
}

fn sync_write_enabled(config: Res<GodotTransformConfig>) -> bool {
    config.sync_mode != TransformSyncMode::Disabled
}

/// Godot->Bevy read stub: calls the real per-axis merge. Idempotent via the
/// value-shadow guard, so duplicate reads on 1-step frames don't trip Changed.
fn read_stub(
    mut q: Query<(&mut Transform, &GodotNode, &mut TransformSyncMetadata)>,
    mut count: ResMut<ReadCount>,
) {
    count.0 += 1;
    for (mut transform, node, mut meta) in q.iter_mut() {
        merge_godot_into_bevy(&mut transform, &node.0, &mut meta.shadow);
    }
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

/// Build an app wired like the production TwoWay path: PreUpdate read, FixedFirst
/// read gated by `not_first_fixed_step`, FixedLast write, FixedUpdate author.
/// `with_fixed_first` lets the control test omit the per-step read to prove the
/// bug. Returns the app and the single synced entity.
fn wired_app(mode: TransformSyncMode, with_fixed_first: bool) -> (App, Entity) {
    let mut app = App::new();
    app.add_plugins(TimePlugin);
    host_fixed_main_loop(&mut app);
    app.insert_resource(GodotTransformConfig { sync_mode: mode });
    app.init_resource::<ReadCount>();

    app.add_systems(PreUpdate, read_stub.run_if(twoway_read_enabled));
    if with_fixed_first {
        app.add_systems(
            FixedFirst,
            read_stub
                .run_if(twoway_read_enabled)
                .run_if(not_first_fixed_step),
        );
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
/// - each physics step runs `run_physics_step`, which does the preamble (startup
///   once ever, prefix once per frame), publishes `FixedStepFirstOfFrame` via the
///   production code, then runs the fixed main
/// - the end-of-frame `_process` runs the prefix on idle 0-step frames, then the
///   suffix and `clear_trackers`
///
/// Driving the real publish (not a hand-copied `step == 0`) means a regression in
/// `run_physics_step`'s `f.0 = need_prefix` breaks these tests, not just the itest.
///
/// `godot_author(world, step)` runs before each step's fixed main, letting a test
/// move the Godot-side node between steps like a GDScript `_physics_process`.
fn frame(app: &mut App, n_steps: u32, mut godot_author: impl FnMut(&mut World, u32)) {
    #[derive(Resource)]
    struct StartupRan;

    let world = app.world_mut();
    let need_startup = !world.contains_resource::<StartupRan>();

    // prefix_done mirrors app.rs's prefix_done_this_frame: true only the first
    // physics step runs the prefix and publishes need_prefix == true.
    let mut prefix_done = false;
    for step in 0..n_steps {
        godot_author(world, step);
        run_physics_step(world, need_startup && step == 0, !prefix_done, DT);
        prefix_done = true;
    }

    // End-of-frame _process(): run the prefix if no physics step did (idle frame),
    // then the suffix and clear_trackers.
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
/// node's y between steps. With the FixedFirst read, step 2 pulls the fresh y
/// before its whole-transform write, so the Godot y survives and the Bevy x
/// advances both steps. Fails on pre-fix code (no FixedFirst read), where step 2's
/// write drags a stale y=0 over the Godot move.
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
/// frame, but the per-step FixedFirst read is omitted (PreUpdate-only, i.e. the
/// pre-fix wiring). The Godot y is clobbered back to the step-1 value, confirming
/// the regression test above is not vacuous.
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
/// the value via the PreUpdate fallback read (the FixedFirst read never runs with
/// 0 steps). Guards that adding the per-step read didn't break the idle cadence.
#[test]
fn idle_frame_reads_via_preupdate_fallback() {
    let (mut app, entity) = wired_app(TransformSyncMode::TwoWay, true);

    app.world_mut().get_mut::<GodotNode>(entity).unwrap().0.translation.y = 5.0;
    frame(&mut app, 0, |_, _| {});

    let bevy = bevy_translation(&app, entity);
    assert!(
        close(bevy.y, 5.0),
        "the PreUpdate read must pull the Godot y into Bevy on an idle frame, got {}",
        bevy.y
    );
}

/// Dedup read-count partition: the reads partition the physics steps gap-free and
/// overlap-free. A 1-step frame and an idle frame each read exactly once
/// (PreUpdate only, byte-for-byte today's cost); a 2-step frame reads twice
/// (PreUpdate + one FixedFirst). Locks perf-neutrality on the common cadence.
#[test]
fn read_count_partitions_steps() {
    let (mut app, _entity) = wired_app(TransformSyncMode::TwoWay, true);

    app.world_mut().resource_mut::<ReadCount>().0 = 0;
    frame(&mut app, 1, |_, _| {});
    assert_eq!(
        app.world().resource::<ReadCount>().0,
        1,
        "1-step frame: PreUpdate read only (FixedFirst skipped on step 1)"
    );

    app.world_mut().resource_mut::<ReadCount>().0 = 0;
    frame(&mut app, 0, |_, _| {});
    assert_eq!(
        app.world().resource::<ReadCount>().0,
        1,
        "idle frame: PreUpdate read only"
    );

    app.world_mut().resource_mut::<ReadCount>().0 = 0;
    frame(&mut app, 2, |_, _| {});
    assert_eq!(
        app.world().resource::<ReadCount>().0,
        2,
        "2-step frame: PreUpdate + one FixedFirst read (step 2)"
    );
}

/// OneWay regression: with sync disabled in the Godot->Bevy direction, neither
/// read body runs at any cadence, while the FixedLast write still fires per step.
#[test]
fn oneway_never_reads_but_still_writes_per_step() {
    let (mut app, entity) = wired_app(TransformSyncMode::OneWay, true);

    app.world_mut().resource_mut::<ReadCount>().0 = 0;
    frame(&mut app, 3, |world, _step| {
        // A Godot move that OneWay deliberately ignores.
        world.get_mut::<GodotNode>(entity).unwrap().0.translation.y += 1.0;
    });

    assert_eq!(
        app.world().resource::<ReadCount>().0,
        0,
        "OneWay must never run the Godot->Bevy read"
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
/// FixedFirst, and the Bevy->Godot write in FixedLast. Fails on pre-fix code,
/// which lacks the FixedFirst read -- guarding the per-step registration itself
/// (the deterministic tests above use stub wiring, so this is the production check).
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
        "Godot->Bevy read must be registered in FixedFirst (per-step, steps 2..N)"
    );
    assert!(
        schedule_has_system(&mut app, FixedLast, "post_update_godot_transforms"),
        "Bevy->Godot write must be registered in FixedLast"
    );
}
