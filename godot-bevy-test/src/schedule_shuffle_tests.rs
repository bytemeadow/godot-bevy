#[cfg(test)]
mod tests {
    use super::*;
    use bevy::ecs::schedule::{LogLevel, ScheduleBuildSettings, ScheduleLabel};

    #[test]
    fn per_test_seeds_are_stable() {
        for (base, name, expected) in [
            (0, "src/alpha.rs::test_order", 2256669775921386235),
            (42, "src/alpha.rs::test_order", 4776846410360907870),
            (42, "src/beta.rs::test_order", 1568956225976230934),
            (u64::MAX, "src/alpha.rs::test_order", 16618023767007689011),
        ] {
            assert_eq!(test_schedule_seed(base, name), expected);
        }
    }

    #[derive(ScheduleLabel, Clone, Debug, Eq, Hash, PartialEq)]
    struct LateSchedule;

    struct LateSchedulePlugin;

    impl Plugin for LateSchedulePlugin {
        fn build(&self, _app: &mut App) {}

        fn finish(&self, app: &mut App) {
            let mut schedule = Schedule::new(LateSchedule);
            schedule.set_build_settings(ScheduleBuildSettings {
                ambiguity_detection: LogLevel::Error,
                hierarchy_detection: LogLevel::Ignore,
                auto_insert_apply_deferred: false,
                use_shortnames: false,
                report_sets: false,
                ..Default::default()
            });
            app.add_schedule(schedule);
        }
    }

    #[test]
    fn cleanup_shuffles_late_schedules_and_preserves_build_settings() {
        let mut app = App::new();
        app.add_plugins((LateSchedulePlugin, ScheduleShufflePlugin(42)));
        app.finish();
        app.cleanup();

        let schedules = app.world().resource::<Schedules>();
        for (_, schedule) in schedules.iter() {
            assert_eq!(schedule.get_build_settings().shuffle_seed, Some(42));
        }
        let settings = schedules.get(LateSchedule).unwrap().get_build_settings();
        assert_eq!(settings.ambiguity_detection, LogLevel::Error);
        assert_eq!(settings.hierarchy_detection, LogLevel::Ignore);
        assert!(!settings.auto_insert_apply_deferred);
        assert!(!settings.use_shortnames);
        assert!(!settings.report_sets);
    }

    #[derive(Resource, Default)]
    struct Order(Vec<usize>);

    fn record<const N: usize>(mut order: ResMut<Order>) {
        order.0.push(N);
    }

    fn shuffled_order(seed: u64) -> Vec<usize> {
        let mut app = App::new();
        app.init_resource::<Order>();
        app.add_systems(Update, (record::<0>, record::<1>, record::<2>, record::<3>));
        app.add_plugins(ScheduleShufflePlugin(seed));
        app.finish();
        app.cleanup();
        app.world_mut().run_schedule(Update);
        app.world_mut().remove_resource::<Order>().unwrap().0
    }

    #[test]
    fn shuffled_schedules_reproduce_and_vary_with_seed() {
        let first = shuffled_order(42);
        assert_eq!(first, shuffled_order(42));
        assert!((0..4).any(|seed| shuffled_order(seed) != first));
    }
}
