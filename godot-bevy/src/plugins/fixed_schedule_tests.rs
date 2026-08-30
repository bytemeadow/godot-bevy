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
    fn zero_delta_step_does_not_panic() {
        // Godot passes delta 0 when Engine.time_scale == 0 (freeze/hitstop). The driver
        // must skip set_timestep (which panics on zero), not tear the app down.
        let mut app = hosted_app();
        run_godot_fixed_main(app.world_mut(), Duration::from_secs_f64(1.0 / 60.0));
        let before = app.world().resource::<Time<Fixed>>().timestep();
        run_godot_fixed_main(app.world_mut(), Duration::ZERO);
        assert_eq!(
            app.world().resource::<Time<Fixed>>().timestep(),
            before,
            "timestep retains its last value across a time_scale==0 frame"
        );
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
    fn suffix_starts_after_the_split_marker() {
        #[derive(Resource, Default)]
        struct MarkerRuns(u32);

        let mut app = hosted_app();
        app.init_resource::<MarkerRuns>();
        app.add_systems(GodotFixedMainLoopSplit, |mut runs: ResMut<MarkerRuns>| {
            runs.0 += 1;
        });

        run_main_suffix(app.world_mut());

        assert_eq!(app.world().resource::<MarkerRuns>().0, 0);
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
