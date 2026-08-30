//! Hosts Bevy's standard `RunFixedMainLoop` schedule, driven once per Godot
//! `_physics_process` step instead of Bevy's in-`_process` accumulator loop.
//! The stock accumulator (`run_fixed_main_schedule`) is stripped and replaced
//! by a single-tick driver so the `BeforeFixedMainLoop`/`AfterFixedMainLoop`
//! anchor sets stay live for ecosystem plugins (e.g. leafwing's fixed input swap).

use std::time::Duration;

use bevy_app::{App, FixedMain, MainScheduleOrder, RunFixedMainLoop, RunFixedMainLoopSystems};
use bevy_ecs::resource::Resource;
use bevy_ecs::schedule::{IntoScheduleConfigs, ScheduleCleanupPolicy, ScheduleLabel};
use bevy_ecs::world::World;
use bevy_time::{Fixed, Time, Virtual};

/// Marker that occupies the slot `RunFixedMainLoop` held in `MainScheduleOrder`.
/// Not registered as a real schedule -- `app.update()` calls `try_run_schedule`
/// on it, which no-ops silently. This is the active split point: schedules
/// before it run in `_physics_process` (prefix), schedules after it run in
/// `_process` (suffix).
#[derive(ScheduleLabel, Debug, Hash, PartialEq, Eq, Clone)]
struct GodotFixedMainLoopSplit;

/// Per-step delta handed from `run_godot_fixed_main` to `godot_fixed_driver`.
#[derive(Resource, Default)]
pub(crate) struct GodotFixedDelta(pub Duration);

/// Set true only when the Main prefix is the `_process` fallback run on a
/// 0-physics-step render frame. Gates the PreUpdate Godot->Bevy read so it
/// fires only when no FixedFirst read ran this frame.
#[doc(hidden)]
#[derive(Resource, Default)]
pub struct ProcessFallbackPrefix(pub bool);

/// Run-condition for the PreUpdate read: true only on a 0-tick frame's
/// process-fallback prefix. Absent resource -> true, the safe default for
/// standalone / benchmark apps that bypass `host_fixed_main_loop` and call
/// `run_schedule(PreUpdate)` directly (so those apps still exercise the read).
#[doc(hidden)]
pub fn prefix_ran_in_process_fallback(
    flag: Option<bevy_ecs::system::Res<ProcessFallbackPrefix>>,
) -> bool {
    flag.is_none_or(|f| f.0)
}

/// Take over Bevy's fixed-timestep loop: drive `RunFixedMainLoop` from Godot's
/// physics clock instead of Bevy's in-`_process` accumulator.
///
/// Must run after `TimePlugin::build` (which registers `run_fixed_main_schedule`).
pub(crate) fn host_fixed_main_loop(app: &mut App) {
    // Stop `app.update()` from running the fixed loop -- we drive it from physics.
    // Replace it with an unregistered marker; split_idx() reads its position to
    // divide the label list into prefix (before) and suffix (after) each frame.
    let run_fixed = RunFixedMainLoop.intern();
    let mut order = app.world_mut().resource_mut::<MainScheduleOrder>();
    let idx = order
        .labels
        .iter()
        .position(|l| *l == run_fixed)
        .expect("RunFixedMainLoop in MainScheduleOrder once TimePlugin is added");
    order.labels[idx] = GodotFixedMainLoopSplit.intern();

    // Strip bevy_time's accumulator -- the only stock member of FixedMainLoop --
    // and substitute our single-tick driver. `remove_systems_in_set` auto-inits
    // the schedule, so this is safe at plugin-build time.
    let removed = app
        .remove_systems_in_set(
            RunFixedMainLoop,
            RunFixedMainLoopSystems::FixedMainLoop,
            ScheduleCleanupPolicy::RemoveSystemsOnly,
        )
        .expect("RunFixedMainLoop exists once TimePlugin is added");
    assert_eq!(
        removed, 1,
        "expected exactly bevy_time's run_fixed_main_schedule in FixedMainLoop, removed {removed}"
    );

    app.init_resource::<GodotFixedDelta>();
    app.init_resource::<ProcessFallbackPrefix>();
    app.add_systems(
        RunFixedMainLoop,
        godot_fixed_driver.in_set(RunFixedMainLoopSystems::FixedMainLoop),
    );
}

/// Exclusive driver: set `Time<Fixed>`'s timestep to the stashed Godot delta and
/// advance by it, swap the generic `Time` to Fixed for the `FixedMain` run, then
/// restore Virtual so the Before/After anchors run under `Res<Time> == Virtual`
/// (mirrors stock `run_fixed_main_schedule`).
///
/// Setting the timestep each step keeps `delta()`, `elapsed()`, and `timestep()`
/// all tracking Godot's physics clock -- a runtime physics-rate change is picked
/// up automatically. `overstep_fraction()` is 0 because each step advances exactly
/// one timestep: Godot owns fixed-step interpolation, not Bevy's accumulator.
fn godot_fixed_driver(world: &mut World) {
    // Freeze FixedMain under pause. Keyed on Time<Virtual>::is_paused() (a tree-pause or a
    // user's own pause()), not delta==0, so a time_scale==0 hitstop still steps. The early
    // return leaves the input-clock pair balanced and generic Time on Virtual.
    if world.resource::<Time<Virtual>>().is_paused() {
        return;
    }
    let delta = world.resource::<GodotFixedDelta>().0;
    let mut fixed = world.resource_mut::<Time<Fixed>>();
    // Godot passes delta 0 when Engine.time_scale == 0 (freeze/hitstop); set_timestep
    // panics on zero, so skip it -- the timestep keeps its last value and advance_by(0)
    // is a no-op.
    if !delta.is_zero() {
        fixed.set_timestep(delta);
    }
    fixed.advance_by(delta);
    *world.resource_mut::<Time>() = world.resource::<Time<Fixed>>().as_generic();
    // Set active=Physics + refresh physics snapshot (no-op if GodotActions absent).
    crate::plugins::input::actions::poll_physics_actions(world);
    FixedMain::run_fixed_main(world);
    // Restore active=Process so subsequent Update reads see the process snapshot.
    crate::plugins::input::actions::restore_process_clock(world);
    *world.resource_mut::<Time>() = world.resource::<Time<Virtual>>().as_generic();
}

/// Drive one Godot physics step: stash the delta and run the hosted
/// `RunFixedMainLoop` schedule (Before -> driver(FixedMain) -> After) once.
///
/// Never calls `clear_trackers` -- that happens exactly once per render frame,
/// at the end of `_process` in `app.rs` (after the suffix).
pub(crate) fn run_godot_fixed_main(world: &mut World, delta: Duration) {
    world.resource_mut::<GodotFixedDelta>().0 = delta;
    world.try_run_schedule(RunFixedMainLoop).ok();
}

// ── split-Main helpers ────────────────────────────────────────────────────────
// Mirror `Main::run_main` (bevy_app main_schedule.rs): resource_scope over the
// live MainScheduleOrder, try_run_schedule for each label, ignoring missing ones.
// The split point is the GodotFixedMainLoopSplit marker inserted by
// `host_fixed_main_loop`; we read it from the live label list each call so
// plugin-inserted prefix schedules (e.g. StateTransition after PreUpdate) route
// correctly without hardcoding First/PreUpdate/StateTransition by name.

fn split_idx(order: &MainScheduleOrder) -> usize {
    let marker = GodotFixedMainLoopSplit.intern();
    order
        .labels
        .iter()
        .position(|l| *l == marker)
        .expect("split marker installed by host_fixed_main_loop")
}

/// Run the startup schedules (PreStartup/Startup/PostStartup and any extras).
/// Idempotency is the caller's responsibility (`started` flag in `app.rs`).
pub(crate) fn run_startup(world: &mut World) {
    world.resource_scope(|world, order: bevy_ecs::world::Mut<MainScheduleOrder>| {
        for &label in &order.startup_labels {
            let _ = world.try_run_schedule(label);
        }
    });
}

/// Run all schedules before the split marker (First, PreUpdate, StateTransition, …).
/// Never calls `clear_trackers`.
pub(crate) fn run_main_prefix(world: &mut World) {
    world.resource_scope(|world, order: bevy_ecs::world::Mut<MainScheduleOrder>| {
        let i = split_idx(&order);
        for &label in &order.labels[..i] {
            let _ = world.try_run_schedule(label);
        }
    });
}

/// Run all schedules after the split marker (Update, PostUpdate, Last, …).
/// Never calls `clear_trackers` -- the caller does that after this returns.
pub(crate) fn run_main_suffix(world: &mut World) {
    world.resource_scope(|world, order: bevy_ecs::world::Mut<MainScheduleOrder>| {
        let i = split_idx(&order);
        for &label in &order.labels[i + 1..] {
            let _ = world.try_run_schedule(label);
        }
    });
}

/// Per-frame preamble shared by `_process` and `_physics_process`: startup
/// (once ever) then the Main prefix. Idempotency for both flags is the caller's
/// responsibility (the `started`/`prefix_done_this_frame` flags in `app.rs`).
pub(crate) fn run_preamble(world: &mut World, need_startup: bool, need_prefix: bool) {
    if need_startup {
        run_startup(world);
    }
    if need_prefix {
        run_main_prefix(world);
    }
}

/// One physics step of the hosted fixed loop, shared by `BevyApp::physics_process`
/// and the deterministic multi-step tests so the `ProcessFallbackPrefix` publish
/// can't drift between production and the test driver.
pub(crate) fn run_physics_step(
    world: &mut World,
    need_startup: bool,
    need_prefix: bool,
    delta: Duration,
) {
    // A physics step's prefix is never the process fallback, so the PreUpdate
    // read stays gated off this step (the FixedFirst read covers it). Published
    // before run_preamble because the flag gates the read inside the prefix.
    // get_resource_mut so a torn-down/partial world never panics.
    if let Some(mut f) = world.get_resource_mut::<ProcessFallbackPrefix>() {
        f.0 = false;
    }
    run_preamble(world, need_startup, need_prefix);
    run_godot_fixed_main(world, delta);
}

#[cfg(test)]
include!("fixed_schedule_tests.rs");
