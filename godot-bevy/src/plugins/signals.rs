use crate::interop::GodotNodeHandle;
use bevy_app::{App, First, Plugin};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    message::{Message, MessageWriter, message_update_system},
    schedule::IntoScheduleConfigs,
    system::{Commands, NonSend, NonSendMut, Query, SystemParam},
};
use godot::{
    classes::{Node, Object},
    obj::{Gd, InstanceId},
    prelude::{Callable, Variant},
};
use std::fmt::Debug;
use std::sync::Arc;
use std::sync::mpsc::Sender;
use tracing::error;

#[derive(Default)]
pub struct GodotSignalsPlugin;

impl Plugin for GodotSignalsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            First,
            write_godot_signal_messages.before(message_update_system),
        )
        .add_message::<GodotSignal>();
    }
}

#[derive(Debug, Clone)]
pub struct GodotSignalArgument {
    pub type_name: String,
    pub value: String,
    pub instance_id: Option<InstanceId>,
}

#[derive(Debug, Message)]
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

/// Global, type-erased dispatch for typed signal messages
pub(crate) trait TypedDispatch: Send {
    fn write_into_world(self: Box<Self>, world: &mut bevy_ecs::world::World);
}

struct TypedEnvelope<T: Message + Send + 'static>(T);

impl<T: Message + Send + 'static> TypedDispatch for TypedEnvelope<T> {
    fn write_into_world(self: Box<Self>, world: &mut bevy_ecs::world::World) {
        if let Some(mut messages) = world.get_resource_mut::<bevy_ecs::message::Messages<T>>() {
            messages.write(self.0);
        }
    }
}

#[doc(hidden)]
pub(crate) struct GlobalTypedSignalReceiver(pub std::sync::mpsc::Receiver<Box<dyn TypedDispatch>>);

#[doc(hidden)]
pub(crate) struct GlobalTypedSignalSender(pub std::sync::mpsc::Sender<Box<dyn TypedDispatch>>);

/// System parameter for connecting Godot signals to Bevy's message system
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
        /// Connect a Godot signal to be forwarded to Bevy's message system
        pub fn connect(&self, node: &mut GodotNodeHandle, signal_name: &str) {
            connect_godot_signal(node, signal_name, self.signal_sender.0.clone());
        }
    }
}

#[allow(deprecated)]
pub use legacy_signals_param::GodotSignals;

fn write_godot_signal_messages(
    events: NonSendMut<GodotSignalReader>,
    mut message_writer: MessageWriter<GodotSignal>,
) {
    message_writer.write_batch(events.0.try_iter());
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

    let closure = move |args: &[&Variant]| -> Variant {
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

        Variant::nil()
    };

    // Create callable from our universal closure
    let callable = Callable::from_fn("universal_signal_handler", closure);

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

/// Generic plugin to enable typed Godot-signal-to-Bevy-message routing for `T`
pub struct GodotTypedSignalsPlugin<T: Message + Send + 'static> {
    _phantom: std::marker::PhantomData<T>,
}

impl<T: Message + Send + 'static> Default for GodotTypedSignalsPlugin<T> {
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
        }
    }
}

impl<T: Message + Send + 'static> Plugin for GodotTypedSignalsPlugin<T> {
    fn build(&self, app: &mut App) {
        // Ensure the Bevy message type exists
        app.add_message::<T>();

        // Install global typed signal channel and consolidated drain once
        if !app.world().contains_non_send::<GlobalTypedSignalSender>() {
            let (sender, receiver) = std::sync::mpsc::channel::<Box<dyn TypedDispatch>>();
            app.world_mut()
                .insert_non_send_resource(GlobalTypedSignalSender(sender));
            app.world_mut()
                .insert_non_send_resource(GlobalTypedSignalReceiver(receiver));

            // One consolidated drain for all typed messages
            app.add_systems(
                First,
                drain_global_typed_signals.before(message_update_system),
            );
        }

        // Per-T deferred connection processor
        app.add_systems(First, process_typed_deferred_signal_connections::<T>);
    }
}

// Exclusive system to drain type-erased global queue into the correct Messages<T> resources
fn drain_global_typed_signals(world: &mut bevy_ecs::world::World) {
    // Collect first to avoid overlapping mutable borrows of `world`
    let mut pending: Vec<Box<dyn TypedDispatch>> = Vec::new();
    if let Some(receiver) = world.get_non_send_resource_mut::<GlobalTypedSignalReceiver>() {
        pending.extend(receiver.0.try_iter());
    }
    for dispatch in pending.drain(..) {
        dispatch.write_into_world(world);
    }
}

/// SystemParam providing typed connect helpers for a specific Bevy `Message` T
#[derive(SystemParam)]
pub struct TypedGodotSignals<'w, T: Message + Send + 'static> {
    /// Global type-erased sender. Provided by first `GodotTypedSignalsPlugin` added.
    typed_sender: NonSend<'w, GlobalTypedSignalSender>,
    _marker: std::marker::PhantomData<T>,
}

impl<'w, T: Message + Send + 'static> TypedGodotSignals<'w, T> {
    /// Connect a Godot signal and map it to a typed Bevy Message `T` via `mapper`.
    /// Multiple connections are supported; each connection sends a `T` when fired.
    pub fn connect_map<F>(
        &self,
        node: &mut GodotNodeHandle,
        signal_name: &str,
        source_entity: Option<Entity>,
        mut mapper: F,
    ) where
        F: FnMut(&[Variant], &GodotNodeHandle, Option<Entity>) -> Option<T> + Send + 'static,
    {
        let mut node_ref = node.get::<Node>();
        let signal_name_copy = signal_name.to_string();
        let source_node = node.clone();
        let sender_t = self.typed_sender.0.clone();

        let closure = move |args: &[&Variant]| -> Variant {
            // Clone variants to owned values we can inspect
            let owned: Vec<Variant> = args.iter().map(|&v| v.clone()).collect();
            let event = mapper(&owned, &source_node, source_entity);
            if let Some(event) = event {
                let _ = sender_t.send(Box::new(TypedEnvelope::<T>(event)));
            }
            Variant::nil()
        };

        let callable =
            Callable::from_fn(&format!("signal_handler_typed_{signal_name_copy}"), closure);
        node_ref.connect(signal_name, &callable);
    }
}

/// Process typed deferred signal connections for entities that now have GodotNodeHandles
fn process_typed_deferred_signal_connections<T: Message + Send + 'static>(
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

/// A single typed deferred connection item for `T` messages
pub struct TypedDeferredConnection<T: Message + Send + 'static> {
    pub signal_name: String,
    pub mapper: Arc<
        dyn Fn(&[Variant], &GodotNodeHandle, Option<Entity>) -> Option<T> + Send + Sync + 'static,
    >,
}

impl<T: Message + Send + 'static> Debug for TypedDeferredConnection<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "TypedDeferredConnection {{ signal_name: {:?} }}",
            self.signal_name
        )
    }
}

/// Component to defer Godot signal connections until a `GodotNodeHandle` exists on the entity
#[derive(Component, Debug)]
pub struct TypedDeferredSignalConnections<T: Message + Send + 'static> {
    pub connections: Vec<TypedDeferredConnection<T>>,
}

impl<T: Message + Send + 'static> Default for TypedDeferredSignalConnections<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Message + Send + 'static> TypedDeferredSignalConnections<T> {
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
        }
    }

    pub fn with_connection<F>(signal_name: impl Into<String>, mapper: F) -> Self
    where
        F: Fn(&[Variant], &GodotNodeHandle, Option<Entity>) -> Option<T> + Send + Sync + 'static,
    {
        Self {
            connections: vec![TypedDeferredConnection {
                signal_name: signal_name.into(),
                mapper: Arc::new(mapper),
            }],
        }
    }

    pub fn push<F>(&mut self, signal_name: impl Into<String>, mapper: F)
    where
        F: Fn(&[Variant], &GodotNodeHandle, Option<Entity>) -> Option<T> + Send + Sync + 'static,
    {
        self.connections.push(TypedDeferredConnection {
            signal_name: signal_name.into(),
            mapper: Arc::new(mapper),
        });
    }
}

/// Type-erased deferred connections. Allows deferred connections of any Bevy Message type
/// to be processed after a GodotNodeHandle exists.
#[doc(hidden)]
pub(crate) trait DeferredSignalConnection: Send + Sync + Debug {
    /// Connect the deferred signal to the given Godot node.
    fn connect(&self, root_node: &Gd<Node>, entity: Entity, typed_sender: &GlobalTypedSignalSender);
}

/// Deferred connection information for a specific `T` message type.
#[doc(hidden)]
#[derive(Debug)]
pub(crate) struct SignalConnectionSpec<T: Message + Send + 'static> {
    pub(crate) node_path: String,
    pub(crate) signal_name: String,
    pub(crate) connections: TypedDeferredSignalConnections<T>,
}

#[doc(hidden)]
impl<T: Message + Send + Debug + 'static> DeferredSignalConnection for SignalConnectionSpec<T> {
    fn connect(
        &self,
        root_node: &Gd<Node>,
        source_entity: Entity,
        typed_sender: &GlobalTypedSignalSender,
    ) {
        let Some(mut target_node) = root_node.get_node_or_null(self.node_path.as_str()) else {
            error!(
                "Failed to find node at path '{}' for signal connection",
                self.node_path
            );
            return;
        };

        for connection in self.connections.connections.iter() {
            let source_node_handle = GodotNodeHandle::new(target_node.clone());
            let typed_sender_copy = typed_sender.0.clone();
            let mapper = connection.mapper.clone();
            let signal_name = self.signal_name.clone();

            let closure = move |args: &[&Variant]| -> Variant {
                let owned: Vec<Variant> = args.iter().map(|&v| v.clone()).collect();
                if let Some(event) = mapper(&owned, &source_node_handle, Some(source_entity)) {
                    let _ = typed_sender_copy.send(Box::new(TypedEnvelope::<T>(event)));
                }
                Variant::nil()
            };

            target_node.connect(
                &signal_name,
                &Callable::from_fn(&format!("signal_handler_typed_{signal_name}"), closure),
            );
        }
    }
}
