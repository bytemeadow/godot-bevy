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

/// Mapper from a GDScript `send_event` payload `Variant` to a typed event.
/// Return `None` to reject/skip a malformed payload (drain logs a warning).
pub type GodotEventMapper<T> = Box<dyn Fn(Variant) -> Option<T> + Send + Sync>;

/// Per-name decaying log gate. A mis-wired high-frequency producer
/// (e.g. a GDScript `_process` calling `send_event` with a typo every frame)
/// must not flood the log. `should_log` returns `true` the first time a name is
/// seen and then only on power-of-two counts (1, 2, 4, 8, ...).
#[derive(Default)]
pub(crate) struct RateLimitedWarner {
    seen: HashMap<String, u64>,
}

impl RateLimitedWarner {
    /// Record one occurrence of `name`; return whether this occurrence should be
    /// logged. True when the post-increment count is a power of two (1, 2, 4, 8, ...).
    pub(crate) fn should_log(&mut self, name: &str) -> bool {
        let count = self.seen.entry(name.to_string()).or_insert(0);
        *count += 1;
        count.is_power_of_two()
    }
}

/// Name → type-erased mapper registry. Populated by `AddGodotEventAppExt::add_godot_event`;
/// read by the GDScript `send_event` func. `warner` rate-limits unknown-name /
/// rejected-payload log noise even under `&self` (immutable) access.
#[derive(Resource, Default)]
pub(crate) struct GodotEventRegistry {
    pub(crate) mappers:
        HashMap<String, Box<dyn Fn(Variant) -> Option<Box<dyn SignalDispatch>> + Send + Sync>>,
    pub(crate) warner: Mutex<RateLimitedWarner>,
}

/// Register typed Godot→Bevy event mappings on a Bevy [`App`].
pub trait AddGodotEventAppExt {
    /// Register a `name -> event` mapping for the GDScript `send_event(name, payload)`.
    /// Ensures the signal channel + drains exist. Re-registering the same `name`
    /// overwrites the prior mapper (last-wins; logged at `debug!`).
    fn add_godot_event<T>(
        &mut self,
        name: &str,
        mapper: impl Fn(Variant) -> Option<T> + Send + Sync + 'static,
    ) -> &mut Self
    where
        T: Event + Clone + Send + 'static,
        for<'a> T::Trigger<'a>: Default;

    /// Convenience for events implementing [`godot::prelude::FromGodot`] (transparent newtypes).
    /// Registers `|payload| payload.try_to::<T>().ok()` — no closure needed.
    /// Re-registering the same `name` overwrites the prior mapper (last-wins).
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

/// Enables the Godot -> Bevy event bridge: installs the shared signal channel +
/// drains and inserts the `GodotEventRegistry`. The Rust `send_event(&app, ..)`
/// free fn works with any plugin that installs the channel (this or
/// `GodotSignalsPlugin`); the GDScript `#[func] send_event` additionally needs
/// this plugin's registry plus `add_godot_event` registrations.
#[derive(Default)]
pub struct GodotEventBridgePlugin;

impl Plugin for GodotEventBridgePlugin {
    fn build(&self, app: &mut App) {
        ensure_signal_channel(app);
        app.init_resource::<GodotEventRegistry>();
    }
}

/// Send a Bevy event into the ECS of a specific `BevyApp` node, from any Godot
/// custom-node Rust code, without access to the App/World. Delivered to `On<T>`
/// observers on the next channel drain of THAT app (next physics tick or next
/// process frame, whichever comes first).
///
/// Use `BevyApp::try_singleton()` for the autoload/single-app case, or pass a
/// specific instance for multi-app setups.
///
/// No-op + `warn!` if the node's App is not live (pre-init, after `teardown`,
/// after an in-frame panic, or in the editor) or has no signal channel.
///
/// NOTE ON ORDERING: this is for callers OUTSIDE the ECS. It enqueues; it is
/// NOT `commands.trigger()` — the event is delivered on the NEXT drain, one
/// drain later. In-system code should prefer `Commands::trigger`.
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
