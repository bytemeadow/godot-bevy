use bevy::{
    app::{App, First, Plugin},
    ecs::{
        component::Component,
        entity::Entity,
        event::{Event, EventWriter, event_update_system},
        schedule::IntoScheduleConfigs,
        system::{Commands, NonSend, NonSendMut, Query, SystemParam},
    },
};
use godot::{
    classes::{Node, Object},
    obj::{Gd, InstanceId},
    prelude::{Callable, Variant},
};
use std::sync::mpsc::Sender;

use crate::interop::GodotNodeHandle;

#[derive(Default)]
pub struct GodotSignalsPlugin;

impl Plugin for GodotSignalsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(First, write_godot_signal_events.before(event_update_system))
            .add_event::<GodotSignal>();
    }
}

#[derive(Debug, Clone)]
pub struct GodotSignalArgument {
    pub type_name: String,
    pub value: String,
    pub instance_id: Option<InstanceId>,
}

#[derive(Debug, Event)]
pub struct GodotSignal {
    pub name: String,
    pub origin: GodotNodeHandle,
    pub target: GodotNodeHandle,
    pub arguments: Vec<GodotSignalArgument>,
}

#[doc(hidden)]
pub struct GodotSignalReader(pub std::sync::mpsc::Receiver<GodotSignal>);

#[doc(hidden)]
pub struct GodotSignalSender(pub std::sync::mpsc::Sender<GodotSignal>);

/// Global, type-erased dispatch for typed signal events
pub(crate) trait TypedDispatch: Send {
    fn write_into_world(self: Box<Self>, world: &mut bevy::ecs::world::World);
}

struct TypedEnvelope<T: Event + Send + 'static>(T);

impl<T: Event + Send + 'static> TypedDispatch for TypedEnvelope<T> {
    fn write_into_world(self: Box<Self>, world: &mut bevy::ecs::world::World) {
        if let Some(mut events) = world.get_resource_mut::<bevy::ecs::event::Events<T>>() {
            events.send(self.0);
        }
    }
}

#[doc(hidden)]
pub(crate) struct GlobalTypedSignalReceiver(pub std::sync::mpsc::Receiver<Box<dyn TypedDispatch>>);

#[doc(hidden)]
pub(crate) struct GlobalTypedSignalSender(pub std::sync::mpsc::Sender<Box<dyn TypedDispatch>>);

/// System parameter for connecting Godot signals to Bevy's event system
/// Legacy SystemParam (deprecated) wrapped in a narrow module-level allow
mod legacy_signals_param {
    #![allow(deprecated)]
    use super::*;

    /// Clean API for connecting Godot signals - hides implementation details from users
    #[derive(SystemParam)]
    #[deprecated(
        note = "Legacy signal bus. Prefer TypedGodotSignals<T> with GodotTypedSignalsPlugin<T>."
    )]
    pub struct GodotSignals<'w> {
        pub(super) signal_sender: NonSendMut<'w, GodotSignalSender>,
    }

    impl<'w> GodotSignals<'w> {
        /// Connect a Godot signal to be forwarded to Bevy's event system
        pub fn connect(&self, node: &mut GodotNodeHandle, signal_name: &str) {
            connect_godot_signal(node, signal_name, self.signal_sender.0.clone());
        }
    }
}

#[allow(deprecated)]
pub use legacy_signals_param::GodotSignals;

fn write_godot_signal_events(
    events: NonSendMut<GodotSignalReader>,
    mut event_writer: EventWriter<GodotSignal>,
) {
    event_writer.write_batch(events.0.try_iter());
}

pub fn connect_godot_signal(
    node: &mut GodotNodeHandle,
    signal_name: &str,
    signal_sender: Sender<GodotSignal>,
) {
    let mut node = node.get::<Node>();
    let node_clone = node.clone();
    let signal_name_copy = signal_name.to_string();
    let node_id = node_clone.instance_id();

    let closure = move |args: &[&Variant]| -> Result<Variant, ()> {
        // Use captured sender directly - no global state needed!
        let arguments: Vec<GodotSignalArgument> = args
            .iter()
            .map(|&arg| variant_to_signal_argument(arg))
            .collect();

        let origin_handle = GodotNodeHandle::from_instance_id(node_id);

        let _ = signal_sender.send(GodotSignal {
            name: signal_name_copy.clone(),
            origin: origin_handle.clone(),
            target: origin_handle,
            arguments,
        });

        Ok(Variant::nil())
    };

    // Create callable from our universal closure
    let callable = Callable::from_local_fn("universal_signal_handler", closure);

    // Connect the signal - this will work with ANY number of arguments!
    node.connect(signal_name, &callable);
}

pub fn variant_to_signal_argument(variant: &Variant) -> GodotSignalArgument {
    let type_name = match variant.get_type() {
        godot::prelude::VariantType::NIL => "Nil",
        godot::prelude::VariantType::BOOL => "Bool",
        godot::prelude::VariantType::INT => "Int",
        godot::prelude::VariantType::FLOAT => "Float",
        godot::prelude::VariantType::STRING => "String",
        godot::prelude::VariantType::VECTOR2 => "Vector2",
        godot::prelude::VariantType::VECTOR3 => "Vector3",
        godot::prelude::VariantType::OBJECT => "Object",
        _ => "Unknown",
    }
    .to_string();

    let value = variant.stringify().to_string();

    // Extract instance ID for objects
    let instance_id = if variant.get_type() == godot::prelude::VariantType::OBJECT {
        variant
            .try_to::<Gd<Object>>()
            .ok()
            .map(|obj| obj.instance_id())
    } else {
        None
    };

    GodotSignalArgument {
        type_name,
        value,
        instance_id,
    }
}

/// Generic plugin to enable typed Godot-signal-to-Bevy-event routing for `T`
pub struct GodotTypedSignalsPlugin<T: Event + Send + 'static> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Event + Send + 'static> Default for GodotTypedSignalsPlugin<T> {
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
        }
    }
}

impl<T: Event + Send + 'static> Plugin for GodotTypedSignalsPlugin<T> {
    fn build(&self, app: &mut App) {
        // Ensure the Bevy event type exists
        app.add_event::<T>();

        // Install global typed signal channel and consolidated drain once
        if !app.world().contains_non_send::<GlobalTypedSignalSender>() {
            let (sender, receiver) = std::sync::mpsc::channel::<Box<dyn TypedDispatch>>();
            app.world_mut()
                .insert_non_send_resource(GlobalTypedSignalSender(sender));
            app.world_mut()
                .insert_non_send_resource(GlobalTypedSignalReceiver(receiver));

            // One consolidated drain for all typed events
            app.add_systems(
                First,
                drain_global_typed_signals.before(event_update_system),
            );
        }

        // Per-T deferred connection processor
        app.add_systems(First, process_typed_deferred_signal_connections::<T>);
    }
}

// Exclusive system to drain type-erased global queue into the correct Events<T> resources
fn drain_global_typed_signals(world: &mut bevy::ecs::world::World) {
    // Collect first to avoid overlapping mutable borrows of `world`
    let mut pending: Vec<Box<dyn TypedDispatch>> = Vec::new();
    if let Some(receiver) = world.get_non_send_resource_mut::<GlobalTypedSignalReceiver>() {
        pending.extend(receiver.0.try_iter());
    }
    for dispatch in pending.drain(..) {
        dispatch.write_into_world(world);
    }
}

/// SystemParam providing typed connect helpers for a specific Bevy `Event` T
#[derive(SystemParam)]
pub struct TypedGodotSignals<'w, T: Event + Send + 'static> {
    /// Global type-erased sender. Provided by first `GodotTypedSignalsPlugin` added.
    typed_sender: NonSend<'w, GlobalTypedSignalSender>,
    _marker: std::marker::PhantomData<T>,
}

impl<'w, T: Event + Send + 'static> TypedGodotSignals<'w, T> {
    /// Connect a Godot signal and map it to a typed Bevy Event `T` via `mapper`.
    /// Multiple connections are supported; each connection sends a `T` when fired.
    pub fn connect_map<F>(
        &self,
        node: &mut GodotNodeHandle,
        signal_name: &str,
        source_entity: Option<Entity>,
        mut mapper: F,
    ) where
        F: FnMut(&[Variant], &GodotNodeHandle, Option<Entity>) -> T + Send + 'static,
    {
        let mut node_ref = node.get::<Node>();
        let signal_name_copy = signal_name.to_string();
        let source_node = node.clone();
        let sender_t = self.typed_sender.0.clone();

        let closure = move |args: &[&Variant]| -> Result<Variant, ()> {
            // Clone variants to owned values we can inspect
            let owned: Vec<Variant> = args.iter().map(|&v| v.clone()).collect();
            let event = mapper(&owned, &source_node, source_entity);
            let _ = sender_t.send(Box::new(TypedEnvelope::<T>(event)));
            Ok(Variant::nil())
        };

        let callable =
            Callable::from_local_fn(&format!("signal_handler_typed_{signal_name_copy}"), closure);
        node_ref.connect(signal_name, &callable);
    }
}

/// Process typed deferred signal connections for entities that now have GodotNodeHandles
fn process_typed_deferred_signal_connections<T: Event + Send + 'static>(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &mut GodotNodeHandle,
        &mut TypedDeferredSignalConnections<T>,
    )>,
    typed: TypedGodotSignals<T>,
) {
    for (entity, mut handle, mut deferred) in query.iter_mut() {
        for conn in deferred.connections.drain(..) {
            let signal = conn.signal_name;
            let mapper = conn.mapper;
            typed.connect_map(
                &mut handle,
                &signal,
                Some(entity),
                move |args, node, ent| (mapper)(args, node, ent),
            );
        }
        // Remove marker after wiring all deferred connections
        commands
            .entity(entity)
            .remove::<TypedDeferredSignalConnections<T>>();
    }
}

// ====================
// Typed Deferred Connections
// ====================

/// A single typed deferred connection item for `T` events
pub struct TypedDeferredConnection<T: Event + Send + 'static> {
    pub signal_name: String,
    pub mapper:
        Box<dyn Fn(&[Variant], &GodotNodeHandle, Option<Entity>) -> T + Send + Sync + 'static>,
}

/// Component to defer Godot signal connections until a `GodotNodeHandle` exists on the entity
#[derive(Component)]
pub struct TypedDeferredSignalConnections<T: Event + Send + 'static> {
    pub connections: Vec<TypedDeferredConnection<T>>,
}

impl<T: Event + Send + 'static> Default for TypedDeferredSignalConnections<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Event + Send + 'static> TypedDeferredSignalConnections<T> {
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
        }
    }

    pub fn with_connection<F>(signal_name: impl Into<String>, mapper: F) -> Self
    where
        F: Fn(&[Variant], &GodotNodeHandle, Option<Entity>) -> T + Send + Sync + 'static,
    {
        Self {
            connections: vec![TypedDeferredConnection {
                signal_name: signal_name.into(),
                mapper: Box::new(mapper),
            }],
        }
    }

    pub fn push<F>(&mut self, signal_name: impl Into<String>, mapper: F)
    where
        F: Fn(&[Variant], &GodotNodeHandle, Option<Entity>) -> T + Send + Sync + 'static,
    {
        self.connections.push(TypedDeferredConnection {
            signal_name: signal_name.into(),
            mapper: Box::new(mapper),
        });
    }
}
