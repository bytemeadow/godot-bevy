//! Drives Bevy's standard `FixedMain` from Godot's `_physics_process` instead of
//! Bevy's in-`_process` accumulator loop.

use std::time::Duration;

use bevy_app::{App, FixedMain, MainScheduleOrder, RunFixedMainLoop};
use bevy_ecs::schedule::ScheduleLabel;
use bevy_ecs::world::World;
use bevy_time::{Fixed, Time, Virtual};

/// Remove Bevy's in-`_process` fixed-timestep loop so `FixedMain` is driven
/// exclusively from Godot's `_physics_process`. Without this, `FixedUpdate` also
/// ticks (at Bevy's ~64 Hz virtual rate) inside `app.update()`.
pub(crate) fn neutralize_in_process_fixed_loop(app: &mut App) {
    let run_fixed = RunFixedMainLoop.intern();
    app.world_mut()
        .resource_mut::<MainScheduleOrder>()
        .labels
        .retain(|label| *label != run_fixed);
}

/// Drive exactly one Bevy fixed tick from Godot's physics clock.
///
/// Advances `Time<Fixed>` by Godot's per-step `delta`, swaps the generic `Time`
/// to the fixed clock so `Res<Time>` reports Godot's delta inside every `Fixed*`
/// schedule, runs `FixedMain` once, then restores generic `Time` to `Time<Virtual>`
/// for the next `_process`.
pub(crate) fn run_godot_fixed_main(world: &mut World, delta: Duration) {
    world.resource_mut::<Time<Fixed>>().advance_by(delta);
    *world.resource_mut::<Time>() = world.resource::<Time<Fixed>>().as_generic();
    FixedMain::run_fixed_main(world);
    *world.resource_mut::<Time>() = world.resource::<Time<Virtual>>().as_generic();
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::FixedUpdate;
    use bevy_ecs::prelude::*;
    use bevy_time::TimePlugin;

    #[derive(Resource, Default)]
    struct Seen {
        delta: f32,
        runs: u32,
    }

    #[test]
    fn driver_runs_fixed_update_once_with_godot_delta() {
        let mut app = App::new();
        app.add_plugins(TimePlugin);
        neutralize_in_process_fixed_loop(&mut app);
        app.init_resource::<Seen>();
        app.add_systems(FixedUpdate, |time: Res<Time>, mut seen: ResMut<Seen>| {
            seen.delta = time.delta_secs();
            seen.runs += 1;
        });

        let dt = Duration::from_secs_f64(1.0 / 60.0);
        run_godot_fixed_main(app.world_mut(), dt);

        let seen = app.world().resource::<Seen>();
        assert_eq!(
            seen.runs, 1,
            "FixedUpdate should run exactly once per driver call"
        );
        assert!(
            (seen.delta - (1.0 / 60.0)).abs() < 1e-6,
            "Res<Time>.delta in FixedUpdate should equal Godot's delta, got {}",
            seen.delta
        );

        run_godot_fixed_main(app.world_mut(), dt);
        assert_eq!(app.world().resource::<Seen>().runs, 2);
    }

    #[test]
    fn neutralization_stops_in_process_fixed_loop() {
        #[derive(Resource, Default)]
        struct FixedRuns(u32);

        let mut app = App::new();
        app.add_plugins(TimePlugin);
        app.init_resource::<FixedRuns>();
        app.add_systems(FixedUpdate, |mut r: ResMut<FixedRuns>| r.0 += 1);

        neutralize_in_process_fixed_loop(&mut app);

        assert!(
            !app.world()
                .resource::<MainScheduleOrder>()
                .labels
                .contains(&RunFixedMainLoop.intern()),
            "RunFixedMainLoop should be removed from MainScheduleOrder"
        );

        app.update();
        app.update();
        assert_eq!(
            app.world().resource::<FixedRuns>().0,
            0,
            "FixedUpdate must not run during app.update() after neutralization"
        );
    }
}
