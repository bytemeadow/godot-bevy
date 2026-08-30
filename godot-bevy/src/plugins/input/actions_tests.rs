#[cfg(test)]
mod tests {
    use super::*;

    fn fake_poll(ga: &mut GodotActions, clock: Clock) {
        let snapshot = match clock {
            Clock::Process => &mut ga.process,
            Clock::Physics => &mut ga.physics,
        };
        snapshot.actions.insert(
            "polled".to_string(),
            make_state_full(true, true, false, 0.625, 0.375),
        );
    }

    fn make_state_full(
        pressed: bool,
        just_pressed: bool,
        just_released: bool,
        strength: f32,
        raw_strength: f32,
    ) -> ActionState {
        ActionState {
            pressed,
            just_pressed,
            just_released,
            strength,
            raw_strength,
        }
    }

    #[test]
    fn active_clock_flip_divergent_snapshots() {
        let mut ga = GodotActions::default();

        // process: "a" pressed+just_pressed, "b" all-false
        ga.process
            .actions
            .insert("a".to_owned(), make_state_full(true, true, false, 0.0, 0.0));
        ga.process
            .actions
            .insert("b".to_owned(), ActionState::default());

        // physics: "a" all-false, "b" pressed+just_pressed
        ga.physics
            .actions
            .insert("a".to_owned(), ActionState::default());
        ga.physics
            .actions
            .insert("b".to_owned(), make_state_full(true, true, false, 0.0, 0.0));

        assert!(ga.pressed("a"), "process: a should be pressed");
        assert!(!ga.pressed("b"), "process: b should not be pressed");

        ga.set_active(Clock::Physics);
        assert!(!ga.pressed("a"), "physics: a should not be pressed");
        assert!(ga.pressed("b"), "physics: b should be pressed");

        ga.set_active(Clock::Process);
        assert!(ga.pressed("a"), "reverted: a should be pressed again");
        assert!(!ga.pressed("b"), "reverted: b should not be pressed again");
    }

    fn read_a(ga: &GodotActions) -> bool {
        ga.pressed("a")
    }

    #[test]
    fn shared_helper_sees_active_clock() {
        let mut ga = GodotActions::default();

        ga.process.actions.insert(
            "a".to_owned(),
            make_state_full(true, false, false, 0.0, 0.0),
        );
        ga.physics.actions.insert(
            "a".to_owned(),
            make_state_full(false, false, false, 0.0, 0.0),
        );

        assert!(read_a(&ga), "helper under Process should see pressed=true");
        ga.set_active(Clock::Physics);
        assert!(
            !read_a(&ga),
            "helper under Physics should see pressed=false"
        );
    }

    #[test]
    fn edge_independence_no_aliasing() {
        let mut ga = GodotActions::default();

        ga.process.actions.insert(
            "held".to_owned(),
            make_state_full(true, false, false, 0.0, 0.0),
        );
        ga.process.actions.insert(
            "rising".to_owned(),
            make_state_full(true, true, false, 0.0, 0.0),
        );
        ga.process.actions.insert(
            "falling".to_owned(),
            make_state_full(false, false, true, 0.0, 0.0),
        );

        assert!(ga.pressed("held"));
        assert!(!ga.just_pressed("held"));
        assert!(!ga.just_released("held"));

        assert!(ga.pressed("rising"));
        assert!(ga.just_pressed("rising"));
        assert!(!ga.just_released("rising"));

        assert!(!ga.pressed("falling"));
        assert!(!ga.just_pressed("falling"));
        assert!(ga.just_released("falling"));
    }

    #[test]
    fn strength_raw_strength_axis_vector() {
        let mut ga = GodotActions::default();

        ga.process.actions.insert(
            "left".to_owned(),
            make_state_full(true, false, false, 0.8, 0.375),
        );
        ga.process.actions.insert(
            "right".to_owned(),
            make_state_full(true, false, false, 0.6, 0.9),
        );
        ga.process.actions.insert(
            "up".to_owned(),
            make_state_full(true, false, false, 0.4, 0.5),
        );
        ga.process.actions.insert(
            "down".to_owned(),
            make_state_full(true, false, false, 0.3, 0.7),
        );

        // strength != raw_strength
        assert_eq!(ga.strength("left"), 0.8);
        assert_eq!(ga.raw_strength("left"), 0.375);

        // axis == pos - neg
        let ax = ga.axis("left", "right");
        assert_eq!(ax, 0.6_f32 - 0.8_f32);

        // vector componentwise
        let v = ga.vector("left", "right", "up", "down");
        assert_eq!(v.x, 0.6_f32 - 0.8_f32);
        assert_eq!(v.y, 0.3_f32 - 0.4_f32);
    }

    #[test]
    fn poll_physics_actions_without_resource_is_noop() {
        let mut world = bevy_ecs::world::World::new();
        // Neither call should panic; the resource must remain absent.
        poll_physics_actions(&mut world);
        restore_process_clock(&mut world);
        assert!(
            world.get_resource::<GodotActions>().is_none(),
            "GodotActions must not be inserted by the driver helpers"
        );
    }

    #[test]
    fn driver_helpers_poll_the_selected_clock_and_restore_process() {
        let mut world = World::new();
        let actions = GodotActions {
            active: Clock::Process,
            poll_override: Some(fake_poll),
            ..Default::default()
        };
        world.insert_resource(actions);

        poll_physics_actions(&mut world);
        let actions = world.resource::<GodotActions>();
        assert_eq!(actions.active, Clock::Physics);
        assert_eq!(actions.raw_strength("polled"), 0.375);

        restore_process_clock(&mut world);
        assert_eq!(world.resource::<GodotActions>().active, Clock::Process);
    }

    #[test]
    fn plugin_registers_and_polls_process_actions() {
        let mut app = App::new();
        app.add_plugins(GodotActionsPlugin);
        {
            let mut actions = app.world_mut().resource_mut::<GodotActions>();
            actions.active = Clock::Physics;
            actions.poll_override = Some(fake_poll);
        }

        app.world_mut().run_schedule(Update);

        let actions = app.world().resource::<GodotActions>();
        assert_eq!(actions.active, Clock::Process);
        assert_eq!(actions.strength("polled"), 0.625);
        assert_eq!(actions.raw_strength("polled"), 0.375);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn typed_unknown_action_does_not_enter_the_warning_gate() {
        let actions = GodotActions::default();
        let state = actions.lookup(ActionRef {
            key: "typed_missing",
            warn_if_unknown: false,
        });

        assert!(!state.pressed);
        assert!(actions.warned.lock().is_empty());
    }

    #[test]
    fn unknown_action_returns_defaults_no_panic() {
        let ga = GodotActions::default();

        assert!(!ga.pressed("does_not_exist"));
        assert_eq!(ga.strength("does_not_exist"), 0.0);

        // Second call -- warn-once must not panic even if already warned.
        assert!(!ga.pressed("does_not_exist"));
        assert_eq!(ga.strength("does_not_exist"), 0.0);
    }
}
