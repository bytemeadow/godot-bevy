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
    app.add_systems(
        RunFixedMainLoop,
        godot_fixed_driver.in_set(RunFixedMainLoopSystems::FixedMainLoop),
    );
}

/// Exclusive driver: advance `Time<Fixed>` by the stashed Godot delta, swap the
/// generic `Time` to Fixed for the `FixedMain` run, then restore Virtual so the
/// Before/After anchors run under `Res<Time> == Virtual` (mirrors stock
/// `run_fixed_main_schedule`).
fn godot_fixed_driver(world: &mut World) {
    let delta = world.resource::<GodotFixedDelta>().0;
    world.resource_mut::<Time<Fixed>>().advance_by(delta);
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

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::{App, FixedUpdate, PostStartup, PreStartup, PreUpdate, Startup, Update};
    use bevy_ecs::prelude::*;
    use bevy_time::TimePlugin;

    fn hosted_app() -> App {
        let mut app = App::new();
        app.add_plugins(TimePlugin);
        host_fixed_main_loop(&mut app);
        app
    }

    #[test]
    fn host_strips_exactly_one_stock_system() {
        // host_fixed_main_loop asserts removed == 1 internally; a clean build proves it.
        let _app = hosted_app();
    }

    #[test]
    fn driver_runs_fixed_update_once_per_step_with_godot_delta() {
        #[derive(Resource, Default)]
        struct Seen {
            delta: f32,
            runs: u32,
        }

        let mut app = hosted_app();
        app.init_resource::<Seen>();
        app.add_systems(FixedUpdate, |time: Res<Time>, mut seen: ResMut<Seen>| {
            seen.delta = time.delta_secs();
            seen.runs += 1;
        });

        let dt = Duration::from_secs_f64(1.0 / 60.0);
        run_godot_fixed_main(app.world_mut(), dt);

        let seen = app.world().resource::<Seen>();
        assert_eq!(seen.runs, 1, "FixedUpdate runs once per physics step");
        assert!(
            (seen.delta - 1.0 / 60.0).abs() < 1e-6,
            "delta = {}",
            seen.delta
        );

        run_godot_fixed_main(app.world_mut(), dt);
        assert_eq!(app.world().resource::<Seen>().runs, 2);
    }

    #[test]
    fn anchors_run_in_order_once_per_step_under_virtual_time() {
        // `Time<()>` (the generic resource) carries no downcasting info -- `context()`
        // returns `&()`. Instead we distinguish Virtual vs Fixed by delta: Virtual is
        // never advanced in this test (no app.update()), so its delta is 0; Fixed is
        // advanced by the driver to dt = 1/60.0 each step.
        const DT: f32 = 1.0 / 60.0;

        #[derive(Resource, Default)]
        struct Log {
            order: Vec<&'static str>,
            before_virtual: u32,
            fixed_is_fixed: u32,
            after_virtual: u32,
        }

        let mut app = hosted_app();
        app.init_resource::<Log>();
        app.add_systems(
            RunFixedMainLoop,
            (|time: Res<Time>, mut log: ResMut<Log>| {
                log.order.push("before");
                if time.delta_secs() < 1e-6 {
                    log.before_virtual += 1;
                }
            })
            .in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
        );
        app.add_systems(FixedUpdate, |time: Res<Time>, mut log: ResMut<Log>| {
            log.order.push("fixed");
            if (time.delta_secs() - DT).abs() < 1e-6 {
                log.fixed_is_fixed += 1;
            }
        });
        app.add_systems(
            RunFixedMainLoop,
            (|time: Res<Time>, mut log: ResMut<Log>| {
                log.order.push("after");
                if time.delta_secs() < 1e-6 {
                    log.after_virtual += 1;
                }
            })
            .in_set(RunFixedMainLoopSystems::AfterFixedMainLoop),
        );

        let dt = Duration::from_secs_f64(1.0 / 60.0);
        run_godot_fixed_main(app.world_mut(), dt);
        run_godot_fixed_main(app.world_mut(), dt);

        let log = app.world().resource::<Log>();
        assert_eq!(
            log.order,
            vec!["before", "fixed", "after", "before", "fixed", "after"],
            "Before -> FixedMain -> After, once per step, twice over two steps"
        );
        assert_eq!(
            log.before_virtual, 2,
            "Before runs under Res<Time> == Virtual"
        );
        assert_eq!(
            log.after_virtual, 2,
            "After runs under Res<Time> == Virtual"
        );
        assert_eq!(
            log.fixed_is_fixed, 2,
            "FixedUpdate runs under Res<Time> == Fixed"
        );
    }

    #[test]
    fn marker_occupies_run_fixed_main_loop_slot() {
        let app = hosted_app();
        let labels = &app.world().resource::<MainScheduleOrder>().labels;

        assert!(
            !labels.contains(&RunFixedMainLoop.intern()),
            "RunFixedMainLoop should not be in MainScheduleOrder"
        );

        // The marker must sit at index 2 (the slot RunFixedMainLoop occupied in
        // the default order: [First, PreUpdate, RunFixedMainLoop, Update, ...]).
        assert_eq!(
            labels.get(2),
            Some(&GodotFixedMainLoopSplit.intern()),
            "GodotFixedMainLoopSplit should be at index 2"
        );
    }

    #[test]
    fn app_update_does_not_run_the_fixed_loop() {
        #[derive(Resource, Default)]
        struct FixedRuns(u32);
        #[derive(Resource, Default)]
        struct AnchorRuns(u32);

        let mut app = hosted_app();
        app.init_resource::<FixedRuns>();
        app.init_resource::<AnchorRuns>();
        app.add_systems(FixedUpdate, |mut r: ResMut<FixedRuns>| r.0 += 1);
        app.add_systems(
            RunFixedMainLoop,
            (|mut r: ResMut<AnchorRuns>| r.0 += 1)
                .in_set(RunFixedMainLoopSystems::BeforeFixedMainLoop),
        );

        assert!(
            !app.world()
                .resource::<MainScheduleOrder>()
                .labels
                .contains(&RunFixedMainLoop.intern()),
            "RunFixedMainLoop removed from MainScheduleOrder"
        );

        app.update();
        app.update();
        assert_eq!(
            app.world().resource::<FixedRuns>().0,
            0,
            "no fixed loop in app.update()"
        );
        assert_eq!(
            app.world().resource::<AnchorRuns>().0,
            0,
            "no anchors in app.update()"
        );
    }

    // ── split-Main helper tests ───────────────────────────────────────────────

    #[test]
    fn run_startup_runs_startup_labels_once() {
        #[derive(Resource, Default)]
        struct Runs {
            pre: u32,
            startup: u32,
            post: u32,
        }

        let mut app = hosted_app();
        app.init_resource::<Runs>();
        app.add_systems(PreStartup, |mut r: ResMut<Runs>| r.pre += 1);
        app.add_systems(Startup, |mut r: ResMut<Runs>| r.startup += 1);
        app.add_systems(PostStartup, |mut r: ResMut<Runs>| r.post += 1);

        run_startup(app.world_mut());

        let r = app.world().resource::<Runs>();
        assert_eq!(r.pre, 1, "PreStartup ran once");
        assert_eq!(r.startup, 1, "Startup ran once");
        assert_eq!(r.post, 1, "PostStartup ran once");
    }

    #[test]
    fn dynamically_inserted_prefix_schedule_runs_in_prefix() {
        // Prove the split is position-based: a schedule inserted after PreUpdate
        // (before the marker) runs in prefix, never in suffix.
        #[derive(ScheduleLabel, Hash, Eq, PartialEq, Clone, Debug)]
        struct CustomPrefixSchedule;

        #[derive(Resource, Default)]
        struct Counts {
            prefix: u32,
            suffix: u32,
        }

        let mut app = hosted_app();
        app.init_resource::<Counts>();

        // Insert AFTER PreUpdate -- this lands before GodotFixedMainLoopSplit.
        app.world_mut()
            .resource_mut::<MainScheduleOrder>()
            .insert_after(PreUpdate, CustomPrefixSchedule);

        app.add_systems(CustomPrefixSchedule, |mut c: ResMut<Counts>| c.prefix += 1);
        app.add_systems(Update, |mut c: ResMut<Counts>| c.suffix += 1);

        run_main_prefix(app.world_mut());

        let c = app.world().resource::<Counts>();
        assert_eq!(c.prefix, 1, "CustomPrefixSchedule runs in prefix");
        assert_eq!(c.suffix, 0, "Update does not run in prefix");

        run_main_suffix(app.world_mut());

        let c = app.world().resource::<Counts>();
        assert_eq!(
            c.prefix, 1,
            "CustomPrefixSchedule does not re-run in suffix"
        );
        assert_eq!(c.suffix, 1, "Update runs in suffix");
    }

    #[test]
    fn state_transition_runs_in_prefix() {
        // Concrete consumer: bevy_state's StatesPlugin inserts StateTransition after
        // PreUpdate, which lands it in the prefix (before the fixed loop).
        // An OnEnter effect fired in the prefix must be visible to FixedUpdate.
        use bevy_state::prelude::*;

        #[derive(States, Default, Hash, Eq, PartialEq, Clone, Debug)]
        enum GameState {
            #[default]
            Loading,
            Playing,
        }

        #[derive(Resource, Default)]
        struct EnteredPlaying(bool);

        #[derive(Resource, Default)]
        struct FixedSawTransition(bool);

        let mut app = hosted_app();
        app.add_plugins(bevy_state::app::StatesPlugin);
        app.init_state::<GameState>();
        app.init_resource::<EnteredPlaying>();
        app.init_resource::<FixedSawTransition>();
        app.add_systems(
            OnEnter(GameState::Playing),
            |mut r: ResMut<EnteredPlaying>| r.0 = true,
        );
        app.add_systems(
            FixedUpdate,
            |entered: Res<EnteredPlaying>, mut saw: ResMut<FixedSawTransition>| {
                if entered.0 {
                    saw.0 = true;
                }
            },
        );

        // Startup runs StateTransition (initial entry into GameState::Loading).
        run_startup(app.world_mut());

        app.world_mut()
            .resource_mut::<NextState<GameState>>()
            .set(GameState::Playing);

        // Prefix runs StateTransition, which fires OnEnter(Playing).
        run_main_prefix(app.world_mut());

        assert!(
            app.world().resource::<EnteredPlaying>().0,
            "OnEnter(Playing) should have fired in the prefix"
        );

        let dt = Duration::from_secs_f64(1.0 / 60.0);
        run_godot_fixed_main(app.world_mut(), dt);

        assert!(
            app.world().resource::<FixedSawTransition>().0,
            "FixedUpdate should see the state transition applied in the prefix"
        );
    }
}
