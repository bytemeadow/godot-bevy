#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::prelude::*;
    use std::any::Any;

    #[derive(Event, Clone)]
    struct Damage {
        amount: i32,
    }

    #[derive(Resource, Default)]
    struct Hits(Vec<i32>);

    fn build() -> App {
        let mut app = App::new();
        ensure_event_channel(&mut app);
        app.init_resource::<Hits>();
        app.add_observer(|t: On<Damage>, mut hits: ResMut<Hits>| {
            hits.0.push(t.event().amount);
        });
        app
    }

    fn enqueue(app: &App, amount: i32) {
        app.world()
            .resource::<GodotEventSender>()
            .send(Damage { amount });
    }

    #[derive(Default)]
    struct RecordingTarget(Mutex<Vec<i32>>);

    impl EventBridgeTarget for RecordingTarget {
        fn enqueue_event<T>(&self, event: T) -> EventBridgeReceipt
        where
            T: Event + Clone + Send + 'static,
            for<'a> T::Trigger<'a>: Default,
        {
            let damage = (&event as &dyn Any)
                .downcast_ref::<Damage>()
                .expect("Damage event");
            self.0.lock().push(damage.amount);
            EventBridgeReceipt(())
        }
    }

    #[test]
    fn public_send_event_delegates_the_exact_event() {
        let target = RecordingTarget::default();
        send_event(&target, Damage { amount: 37 });
        assert_eq!(*target.0.lock(), vec![37]);
    }

    #[test]
    fn channel_round_trip_triggers_observer_once() {
        let mut app = build();
        enqueue(&app, 7);
        app.world_mut().run_schedule(First);
        assert_eq!(app.world().resource::<Hits>().0, vec![7]);
    }

    #[test]
    fn drain_is_fifo() {
        let mut app = build();
        enqueue(&app, 1);
        enqueue(&app, 2);
        enqueue(&app, 3);
        app.world_mut().run_schedule(First);
        assert_eq!(app.world().resource::<Hits>().0, vec![1, 2, 3]);
    }

    #[test]
    fn add_godot_event_installs_channel_and_registry() {
        let mut app = App::new();
        app.add_godot_event::<Damage>("damage", |_p| Some(Damage { amount: 0 }));
        assert!(app.world().contains_resource::<GodotEventSender>());
        assert!(app.world().contains_resource::<GodotEventRegistry>());
    }

    #[test]
    fn add_godot_event_registers_named_mapper() {
        let mut app = App::new();
        app.add_godot_event::<Damage>("damage", |_p| Some(Damage { amount: 0 }));
        assert!(
            app.world()
                .resource::<GodotEventRegistry>()
                .mappers
                .contains_key("damage")
        );
    }

    #[test]
    fn re_registering_same_name_is_last_wins() {
        let mut app = App::new();
        app.add_godot_event::<Damage>("x", |_p| Some(Damage { amount: 1 }));
        app.add_godot_event::<Damage>("x", |_p| Some(Damage { amount: 2 }));
        assert_eq!(
            app.world().resource::<GodotEventRegistry>().mappers.len(),
            1
        );
    }

    #[derive(Event, Clone, godot::prelude::GodotConvert)]
    #[godot(transparent)]
    struct Volume(f64);

    #[test]
    fn add_godot_event_from_registers_named_mapper() {
        let mut app = App::new();
        app.add_godot_event_from::<Volume>("volume");
        assert!(
            app.world()
                .resource::<GodotEventRegistry>()
                .mappers
                .contains_key("volume")
        );
    }

    #[test]
    fn rate_limited_warner_decays_per_name() {
        let mut w = RateLimitedWarner::default();
        let logged: Vec<bool> = (0..8).map(|_| w.should_log("damage")).collect();
        // counts 1,2,4,8 log; 3,5,6,7 do not
        assert_eq!(
            logged,
            vec![true, true, false, true, false, false, false, true]
        );
    }

    #[test]
    fn rate_limited_warner_tracks_names_independently() {
        let mut w = RateLimitedWarner::default();
        assert!(w.should_log("a")); // 1
        assert!(w.should_log("b")); // 1
        assert!(w.should_log("a")); // 2 -> logs
        assert!(!w.should_log("a")); // 3 -> suppressed
        assert!(w.should_log("b")); // 2 -> logs
    }
}
