//! Runtime inspection over Godot's debugger channel.

pub mod edit;
mod legacy;
mod service;
pub mod value;
mod wire;

pub use value::{InspectorRange, InspectorReadOnly};
pub use wire::Wire;

use std::{
    rc::Rc,
    sync::mpsc::{Receiver, Sender, channel},
};

use bevy_app::{App, First, Plugin, Update};
use bevy_ecs::{
    prelude::*,
    schedule::{IntoScheduleConfigs, SystemSet},
};
use bevy_time::Time;
use godot::{
    classes::EngineDebugger,
    prelude::{VarDictionary as Dictionary, *},
};

use crate::plugins::scene_tree::plugin::SceneTreeSet;
use service::{Subscription, SummaryChanges};
use value::ValueLimits;

/// Configuration for the debugger plugin.
#[derive(Resource)]
pub struct DebuggerConfig {
    /// Whether the debugger is enabled.
    pub enabled: bool,
    /// Legacy dock refresh interval, in seconds.
    pub update_interval: f32,
    /// Bounds on collection elements and nesting depth when reading values.
    pub value_limits: ValueLimits,
    /// Maximum requests drained per run of `First`.
    pub max_requests_per_frame: usize,
}

impl Default for DebuggerConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            update_interval: 0.5,
            value_limits: ValueLimits::default(),
            max_requests_per_frame: 64,
        }
    }
}

/// The main-thread endpoint used by the message capture and other Godot callers.
/// Cloning it never binds `BevyApp`. Submission only enqueues; `First` owns execution.
#[derive(Clone)]
pub struct DebuggerEndpoint(Rc<Sender<Dictionary>>);

impl DebuggerEndpoint {
    /// Returns false after this app has shut down. Pass an owned frame; do not mutate shared copies after submission.
    pub fn submit(&self, frame: Dictionary) -> bool {
        self.0.send(frame).is_ok()
    }
}

/// Responses and notifications from the runtime service, before conversion to Godot Variants.
#[derive(Resource)]
pub struct DebuggerTransport {
    sink: Box<dyn Fn(Wire) + Send + Sync>,
    legacy_sink: Box<dyn Fn(VarArray) + Send + Sync>,
    debugger_attached: fn() -> bool,
}

impl DebuggerTransport {
    /// Uses an owned Rust frame sink for responses and subscription notifications.
    pub fn new(sink: impl Fn(Wire) + Send + Sync + 'static) -> Self {
        Self {
            sink: Box::new(sink),
            legacy_sink: Box::new(|entities| {
                EngineDebugger::singleton().send_message("bevy:entities", &entities)
            }),
            debugger_attached: || EngineDebugger::singleton().is_active(),
        }
    }

    /// Overrides the temporary legacy dock transport and its connection check.
    pub fn with_legacy_sink(
        mut self,
        attached: fn() -> bool,
        sink: impl Fn(VarArray) + Send + Sync + 'static,
    ) -> Self {
        self.debugger_attached = attached;
        self.legacy_sink = Box::new(sink);
        self
    }

    fn send(&self, frame: Wire) {
        (self.sink)(frame);
    }
}

impl Default for DebuggerTransport {
    fn default() -> Self {
        Self::new(|frame| {
            let mut debugger = EngineDebugger::singleton();
            if debugger.is_active() {
                let mut data = VarArray::new();
                data.push(&frame.to_variant());
                debugger.send_message("bevy:rpc", &data);
            }
        })
    }
}

/// Orders request handling after scene-tree changes and before gameplay schedules.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum DebuggerSet {
    Drain,
}

struct Runtime {
    receiver: Receiver<Dictionary>,
    subscription: Option<Subscription>,
    capture_registered: bool,
    legacy_elapsed: f32,
}

impl Drop for Runtime {
    fn drop(&mut self) {
        if self.capture_registered {
            EngineDebugger::singleton().unregister_message_capture("bevy");
        }
    }
}

/// Plugin that enables Bevy entity inspection in Godot's debugger.
#[derive(Default)]
pub struct GodotDebuggerPlugin;

impl Plugin for GodotDebuggerPlugin {
    fn build(&self, app: &mut App) {
        let (sender, receiver) = channel();
        let endpoint = DebuggerEndpoint(Rc::new(sender));
        let mut debugger = EngineDebugger::singleton();
        let capture_registered = !debugger.has_capture("bevy");
        if capture_registered {
            let endpoint = endpoint.clone();
            let capture = Callable::from_fn("bevy_rpc", move |args: &[&Variant]| {
                let Some(message) = args.first().and_then(|arg| arg.try_to::<GString>().ok())
                else {
                    return false;
                };
                if message != "rpc" {
                    return false;
                }
                let Some(data) = args.get(1).and_then(|arg| arg.try_to::<VarArray>().ok()) else {
                    return false;
                };
                if data.len() != 1 {
                    return false;
                }
                let Some(frame) = data
                    .get(0)
                    .and_then(|frame| frame.try_to::<Dictionary>().ok())
                else {
                    return false;
                };
                endpoint.submit(frame)
            });
            debugger.register_message_capture("bevy", &capture);
        } else {
            godot_warn!(
                "godot-bevy: the bevy debugger capture is already registered; this app's endpoint will not receive editor requests"
            );
        }
        app.init_resource::<DebuggerConfig>()
            .init_resource::<DebuggerTransport>()
            .init_resource::<SummaryChanges>()
            .insert_non_send(endpoint)
            .insert_non_send(Runtime {
                receiver,
                subscription: None,
                capture_registered,
                legacy_elapsed: 0.0,
            })
            .add_systems(
                First,
                (service::track_summary_changes, drain)
                    .chain()
                    .after(SceneTreeSet::Apply)
                    .in_set(DebuggerSet::Drain),
            )
            .add_systems(Update, legacy_update);
    }
}

fn drain(world: &mut World) {
    let mut runtime = world.remove_non_send::<Runtime>().unwrap();
    let count = world
        .resource::<DebuggerConfig>()
        .max_requests_per_frame
        .max(1);
    // Collect before handling requests so re-entrant submissions wait until the next First.
    let requests = runtime.receiver.try_iter().take(count).collect::<Vec<_>>();
    for request in requests {
        let id = request
            .get("id")
            .and_then(|id| Wire::from_variant(&id, 0, &mut 8192).ok())
            .unwrap_or(Wire::Null);
        let request = Wire::from_variant(&request.to_variant(), 0, &mut 8192)
            .map_err(service::RpcError::invalid);
        let notification = request.as_ref().is_ok_and(|request| {
            request.get("id").is_none() && service::request_method(request).is_ok()
        });
        let result = request
            .and_then(|request| service::dispatch(world, &mut runtime.subscription, &request));
        if notification {
            service::publish(world, &mut runtime.subscription);
            continue;
        }
        let response = match result {
            Ok(result) => Wire::object([
                ("jsonrpc", Wire::String("2.0".into())),
                ("id", id),
                ("result", result),
            ]),
            Err(error) => Wire::object([
                ("jsonrpc", Wire::String("2.0".into())),
                ("id", id),
                ("error", error.to_wire()),
            ]),
        };
        world.resource::<DebuggerTransport>().send(response);
        service::publish(world, &mut runtime.subscription);
    }
    service::publish(world, &mut runtime.subscription);
    world.insert_non_send(runtime);
}

fn legacy_update(world: &mut World) {
    let config = world.resource::<DebuggerConfig>();
    if !config.enabled || world.non_send::<Runtime>().subscription.is_some() {
        return;
    }
    let interval = config.update_interval;
    let delta = world.get_resource::<Time>().map_or(0.0, Time::delta_secs);
    let mut runtime = world.non_send_mut::<Runtime>();
    runtime.legacy_elapsed += delta;
    if runtime.legacy_elapsed < interval {
        return;
    }
    runtime.legacy_elapsed = 0.0;
    if !(world.resource::<DebuggerTransport>().debugger_attached)() {
        return;
    }
    let entities = legacy::entities(world);
    (world.resource::<DebuggerTransport>().legacy_sink)(entities);
}

#[cfg(test)]
mod tests {
    #[test]
    fn public_exports_keep_their_paths() {
        let config: crate::prelude::DebuggerConfig = crate::plugins::DebuggerConfig::default();
        let _: crate::prelude::GodotDebuggerPlugin = crate::plugins::GodotDebuggerPlugin;
        let _: crate::prelude::InspectorReadOnly = crate::plugins::InspectorReadOnly;
        let range: crate::prelude::InspectorRange = crate::plugins::InspectorRange::new(0.0, 10.0);
        assert_eq!((range.min, range.max), (0.0, 10.0));
        let _: crate::prelude::DebuggerSet = crate::plugins::DebuggerSet::Drain;
        let _: crate::prelude::SceneTreeSet = crate::plugins::SceneTreeSet::Apply;
        let _: crate::plugins::scene_tree::SceneTreeSet = crate::plugins::SceneTreeSet::Apply;
        assert!(config.enabled);
        assert_eq!(config.update_interval, 0.5);
    }
}
