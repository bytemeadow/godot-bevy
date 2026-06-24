use crate::app::BevyApp;
use crate::plugins::signals::{
    SignalDispatch, SignalEnvelope, SignalSender, ensure_signal_channel,
};
use bevy_app::{App, Plugin};
use bevy_ecs::event::Event;
use bevy_ecs::resource::Resource;
use godot::obj::Gd;
use godot::prelude::Variant;
use parking_lot::Mutex;
use std::collections::HashMap;

/// Decodes a GDScript payload into a typed event; return `None` to reject it.
pub type GodotEventMapper<T> = Box<dyn Fn(Variant) -> Option<T> + Send + Sync>;

/// Per-name log gate so one mis-wired producer can't flood the log: a name logs
/// on power-of-two counts (1, 2, 4, 8, ...) and is swallowed in between.
#[derive(Default)]
pub(crate) struct RateLimitedWarner {
    seen: HashMap<String, u64>,
}

impl RateLimitedWarner {
    pub(crate) fn should_log(&mut self, name: &str) -> bool {
        let count = self.seen.entry(name.to_string()).or_insert(0);
        *count += 1;
        count.is_power_of_two()
    }
}

/// Name → mapper, filled by `add_godot_event` and read by the GDScript
/// `send_event` func. `warner` is a `Mutex` so that `&self` func can still
/// rate-limit its warnings.
#[derive(Resource, Default)]
pub(crate) struct GodotEventRegistry {
    pub(crate) mappers:
        HashMap<String, Box<dyn Fn(Variant) -> Option<Box<dyn SignalDispatch>> + Send + Sync>>,
    pub(crate) warner: Mutex<RateLimitedWarner>,
}

/// Registers `name -> event` decoders for the GDScript `send_event`.
pub trait AddGodotEventAppExt {
    /// Registers a decoder for `send_event("name", payload)`, and installs the
    /// channel + drains. Re-registering a `name` replaces it (last-wins).
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
        T: Event + Clone + Send + 'static + godot::prelude::FromGodot,
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
        ensure_signal_channel(self);
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
        T: Event + Clone + Send + 'static + godot::prelude::FromGodot,
        for<'a> T::Trigger<'a>: Default,
    {
        self.add_godot_event::<T>(name, |payload| payload.try_to::<T>().ok())
    }
}

/// Installs the signal channel + drains and the event registry. The Rust
/// `send_event(&app, ..)` only needs the channel (this or `GodotSignalsPlugin`
/// provides it); the GDScript `send_event` also needs this plugin's registry,
/// which you fill with `add_godot_event`.
#[derive(Default)]
pub struct GodotEventBridgePlugin;

impl Plugin for GodotEventBridgePlugin {
    fn build(&self, app: &mut App) {
        ensure_signal_channel(app);
        app.init_resource::<GodotEventRegistry>();
    }
}

/// Sends a typed event into a specific `BevyApp`'s ECS from Godot Rust code. It
/// reaches `On<T>` observers on the next drain — it enqueues, it doesn't
/// `trigger` synchronously, so code already inside a system wants
/// `Commands::trigger` instead. No-op (with a `warn!`) if the app isn't live or
/// has no channel. Resolve `app` with `BevyApp::try_singleton()`, or pass a
/// specific instance.
pub fn send_event<T>(app: &Gd<BevyApp>, event: T)
where
    T: Event + Clone + Send + 'static,
    for<'a> T::Trigger<'a>: Default,
{
    let binding = app.bind();
    let Some(bevy_app) = binding.get_app() else {
        tracing::warn!(
            "godot_bevy::send_event called on a BevyApp whose App is not live \
             (pre-init / torn down / editor); event dropped"
        );
        return;
    };
    let Some(sender) = bevy_app.world().get_resource::<SignalSender>() else {
        tracing::warn!(
            "godot_bevy::send_event: this BevyApp has no signal channel; add \
             GodotEventBridgePlugin or GodotSignalsPlugin. Event dropped"
        );
        return;
    };
    let boxed: Box<dyn SignalDispatch> = Box::new(SignalEnvelope { event });
    if sender.0.send(boxed).is_err() {
        tracing::warn!("godot_bevy::send_event: channel receiver gone; event dropped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_app::App;

    #[test]
    fn plugin_installs_channel_and_registry() {
        let mut app = App::new();
        app.add_plugins(GodotEventBridgePlugin);
        assert!(
            app.world()
                .contains_resource::<crate::plugins::signals::SignalSender>(),
            "plugin must install the signal channel via ensure_signal_channel"
        );
        assert!(
            app.world().contains_resource::<GodotEventRegistry>(),
            "plugin must init the GodotEventRegistry resource"
        );
    }

    #[test]
    fn rate_limited_warner_decays_per_name() {
        let mut w = RateLimitedWarner::default();
        let mut logged = Vec::new();
        for _ in 0..8 {
            logged.push(w.should_log("damage"));
        }
        // counts 1,2,4,8 log; 3,5,6,7 do not
        assert_eq!(
            logged,
            vec![true, true, false, true, false, false, false, true]
        );
    }

    #[test]
    fn rate_limited_warner_tracks_names_independently() {
        let mut w = RateLimitedWarner::default();
        assert!(w.should_log("a")); // count 1
        assert!(w.should_log("b")); // count 1
        assert!(w.should_log("a")); // count 2 -> logs
        assert!(!w.should_log("a")); // count 3 -> suppressed
        assert!(w.should_log("b")); // count 2 -> logs
    }

    use bevy_ecs::event::Event;

    #[derive(Event, Clone)]
    struct Damage {
        #[allow(dead_code)]
        amount: i32,
    }

    #[test]
    fn add_godot_event_registers_named_mapper() {
        let mut app = App::new();
        app.add_godot_event::<Damage>("damage", |_payload| Some(Damage { amount: 0 }));

        let registry = app.world().resource::<GodotEventRegistry>();
        assert!(
            registry.mappers.contains_key("damage"),
            "mapper for \"damage\" should be registered"
        );
    }

    #[test]
    fn re_registering_same_name_overwrites_last_wins() {
        let mut app = App::new();
        app.add_godot_event::<Damage>("x", |_p| Some(Damage { amount: 1 }));
        app.add_godot_event::<Damage>("x", |_p| Some(Damage { amount: 2 }));

        let registry = app.world().resource::<GodotEventRegistry>();
        assert_eq!(
            registry.mappers.len(),
            1,
            "same name must not create a 2nd entry"
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
}
