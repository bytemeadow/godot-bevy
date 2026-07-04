use crate::app::BevyApp;
use crate::plugins::signals::{SignalDispatch, SignalEnvelope};
use bevy_app::{App, First};
use bevy_ecs::event::Event;
use bevy_ecs::prelude::Resource;
use bevy_ecs::schedule::{IntoScheduleConfigs, SystemSet};
use crossbeam_channel::{Receiver, Sender};
use godot::obj::Gd;
use godot::prelude::{FromGodot, Variant};
use parking_lot::Mutex;
use std::collections::HashMap;

/// The event drain runs in this set, in `First`. Order a system around delivery
/// with `.after(EventBridgeSet::Drain)`; observers themselves fire inside it.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum EventBridgeSet {
    Drain,
}

/// The handle every Rust fire goes through. A `Resource` (so a system can take
/// `Res<GodotEventSender>`) and `Clone + Send + Sync` (so it moves to a worker
/// thread). Per-app: it wraps this world's channel.
///
/// The inner `Sender` is `pub(crate)` so the GDScript `#[func]` can push an
/// already-erased `Box<dyn SignalDispatch>` (the registry path) through it; the
/// public surface stays the typed `send<T>`.
#[derive(Resource, Clone)]
pub struct GodotEventSender(pub(crate) Sender<Box<dyn SignalDispatch>>);

impl GodotEventSender {
    /// Enqueue a typed event. It reaches `On<T>` observers on the next `First`
    /// drain — it does not trigger synchronously, so code already inside a system
    /// wants `Commands::trigger` instead.
    pub fn send<T>(&self, event: T)
    where
        T: Event + Clone + Send + 'static,
        for<'a> T::Trigger<'a>: Default,
    {
        let boxed: Box<dyn SignalDispatch> = Box::new(SignalEnvelope { event });
        if self.0.send(boxed).is_err() {
            tracing::warn!("GodotEventSender::send: channel receiver gone; event dropped");
        }
    }
}

#[derive(Resource)]
struct GodotEventReceiver(Mutex<Receiver<Box<dyn SignalDispatch>>>);

/// Installs the event channel + its drain, once per App (idempotent — guarded on
/// `GodotEventSender`, so core and `add_godot_event` can both call it). A
/// separate channel from `signals.rs`'s: events and signals don't share a queue.
pub(crate) fn ensure_event_channel(app: &mut App) {
    if app.world().contains_resource::<GodotEventSender>() {
        return;
    }
    let (tx, rx) = crossbeam_channel::unbounded::<Box<dyn SignalDispatch>>();
    app.world_mut().insert_resource(GodotEventSender(tx));
    app.world_mut()
        .insert_resource(GodotEventReceiver(Mutex::new(rx)));
    app.add_systems(
        First,
        drain_and_trigger_events.in_set(EventBridgeSet::Drain),
    );
}

/// Mirror of `drain_and_trigger_signals`: collect via `try_iter` (consume-once)
/// to avoid overlapping `world` borrows, then trigger each box.
fn drain_and_trigger_events(world: &mut bevy_ecs::world::World) {
    let mut pending: Vec<Box<dyn SignalDispatch>> = Vec::new();
    if let Some(receiver) = world.get_resource::<GodotEventReceiver>() {
        pending.extend(receiver.0.lock().try_iter());
    }
    for dispatch in pending {
        dispatch.trigger_in_world(world);
    }
}

/// Per-name log gate so one mis-wired producer can't flood the log: a name logs
/// on power-of-two counts (1, 2, 4, 8, ...) and is swallowed in between.
#[derive(Default)]
pub(crate) struct RateLimitedWarner {
    seen: HashMap<String, u64>,
}

impl RateLimitedWarner {
    pub(crate) fn should_log(&mut self, name: &str) -> bool {
        // Allocate the key only the first time a name is seen, not every call.
        if let Some(count) = self.seen.get_mut(name) {
            *count += 1;
            count.is_power_of_two()
        } else {
            self.seen.insert(name.to_string(), 1);
            true
        }
    }
}

/// Name → decoder, filled by `add_godot_event` and read by the GDScript
/// `send_event` func. `warner` is a `Mutex` so the `&self` func can rate-limit.
#[derive(Resource, Default)]
pub(crate) struct GodotEventRegistry {
    pub(crate) mappers:
        HashMap<String, Box<dyn Fn(Variant) -> Option<Box<dyn SignalDispatch>> + Send + Sync>>,
    pub(crate) warner: Mutex<RateLimitedWarner>,
}

/// Registers `name -> event` decoders for the GDScript `send_event`.
pub trait AddGodotEventAppExt {
    /// Registers a decoder for `send_event("name", payload)`. Re-registering a
    /// `name` replaces it (last-wins).
    fn add_godot_event<T>(
        &mut self,
        name: &str,
        mapper: impl Fn(Variant) -> Option<T> + Send + Sync + 'static,
    ) -> &mut Self
    where
        T: Event + Clone + Send + 'static,
        for<'a> T::Trigger<'a>: Default;

    /// Like `add_godot_event`, but for `FromGodot` types (transparent newtypes) —
    /// the decode is `try_to::<T>()`, so you skip the mapper closure.
    fn add_godot_event_from<T>(&mut self, name: &str) -> &mut Self
    where
        T: Event + Clone + Send + 'static + FromGodot,
        for<'a> T::Trigger<'a>: Default;
}

impl AddGodotEventAppExt for App {
    fn add_godot_event<T>(
        &mut self,
        name: &str,
        mapper: impl Fn(Variant) -> Option<T> + Send + Sync + 'static,
    ) -> &mut Self
    where
        T: Event + Clone + Send + 'static,
        for<'a> T::Trigger<'a>: Default,
    {
        ensure_event_channel(self);
        self.init_resource::<GodotEventRegistry>();
        let erased: Box<dyn Fn(Variant) -> Option<Box<dyn SignalDispatch>> + Send + Sync> =
            Box::new(move |payload| {
                mapper(payload)
                    .map(|event| Box::new(SignalEnvelope { event }) as Box<dyn SignalDispatch>)
            });
        let mut registry = self.world_mut().resource_mut::<GodotEventRegistry>();
        if registry.mappers.insert(name.to_string(), erased).is_some() {
            tracing::debug!("add_godot_event overwrote existing mapper for {name:?}");
        }
        self
    }

    fn add_godot_event_from<T>(&mut self, name: &str) -> &mut Self
    where
        T: Event + Clone + Send + 'static + FromGodot,
        for<'a> T::Trigger<'a>: Default,
    {
        self.add_godot_event::<T>(name, |payload| payload.try_to::<T>().ok())
    }
}

/// Send a typed event into a specific `BevyApp`'s ECS from Godot Rust code that
/// holds a `Gd<BevyApp>`. It reaches `On<T>` observers on the next `First` drain
/// — it enqueues, it doesn't `trigger` synchronously, so code already inside a
/// system wants `Commands::trigger` instead. No-op (with a `warn!`) if the app
/// isn't live. Resolve `app` with `BevyApp::try_singleton()`, or pass a specific
/// instance.
///
/// Call this from the main thread, from a node callback between frames — it
/// binds the app to reach its world. Do NOT fire it while that app's own frame
/// is running (from a system, or a signal a system emitted synchronously): the
/// `bind()` panics inside gdext and the frame's `catch_unwind` tears the app
/// down. Off-thread, hold a cloned `GodotEventSender` and send through that.
pub fn send_event<T>(app: &Gd<BevyApp>, event: T)
where
    T: Event + Clone + Send + 'static,
    for<'a> T::Trigger<'a>: Default,
{
    app.bind().send_event(event);
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::prelude::*;

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
