use crate::interop::{GodotAccess, GodotNodeHandle};
use bevy_app::{App, First, Last, Plugin};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    message::{Message, message_update_system},
    prelude::Resource,
    schedule::IntoScheduleConfigs,
    system::{Commands, Query, Res, SystemParam},
};
use crossbeam_channel::Sender;
use godot::{
    classes::Node,
    obj::Gd,
    prelude::{Callable, Variant},
};
use parking_lot::Mutex;
use std::fmt::Debug;
use std::sync::Arc;
use tracing::error;

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

/// Resource for receiving typed signal dispatches.
/// Wrapped in Mutex to be Send+Sync, allowing it to be a regular Bevy Resource.
#[derive(Resource)]
pub(crate) struct GlobalTypedSignalReceiver(
    pub Mutex<crossbeam_channel::Receiver<Box<dyn TypedDispatch>>>,
);

impl GlobalTypedSignalReceiver {
    pub fn new(receiver: crossbeam_channel::Receiver<Box<dyn TypedDispatch>>) -> Self {
        Self(Mutex::new(receiver))
    }
}

#[doc(hidden)]
#[derive(Resource)]
pub(crate) struct GlobalTypedSignalSender(pub crossbeam_channel::Sender<Box<dyn TypedDispatch>>);

#[derive(Resource, Default)]
struct PendingSignalConnections {
    queue: Mutex<Vec<Box<dyn PendingSignalConnection>>>,
}

trait PendingSignalConnection: Send {
    fn connect(self: Box<Self>, godot: &mut GodotAccess);
}

impl PendingSignalConnections {
    fn push(&self, connection: Box<dyn PendingSignalConnection>) {
        self.queue.lock().push(connection);
    }

    fn drain(&self) -> Vec<Box<dyn PendingSignalConnection>> {
        self.queue.lock().drain(..).collect()
    }
}

fn ensure_signal_connection_queue(app: &mut App) {
    if !app.world().contains_resource::<PendingSignalConnections>() {
        app.init_resource::<PendingSignalConnections>()
            // Process pending connections at end of frame so connections made
            // during Update are applied same-frame (ready for next frame's signals)
            .add_systems(Last, process_pending_signal_connections);
    }
}

fn process_pending_signal_connections(
    pending: Res<PendingSignalConnections>,
    mut godot: GodotAccess,
) {
    for connection in pending.drain() {
        connection.connect(&mut godot);
    }
}

fn connect_typed_signal<T: Message + Send + 'static>(
    godot: &mut GodotAccess,
    node: GodotNodeHandle,
    signal_name: &str,
    source_entity: Option<Entity>,
    mapper: Box<
        dyn FnMut(&[Variant], GodotNodeHandle, Option<Entity>) -> Option<T> + Send + 'static,
    >,
    sender: Sender<Box<dyn TypedDispatch>>,
) {
    let mut node_ref = godot.get::<Node>(node);
    let signal_name_copy = signal_name.to_string();
    let source_node_handle = node;
    let mut mapper = mapper;

    let closure = move |args: &[&Variant]| -> Variant {
        // Clone variants to owned values we can inspect
        let owned: Vec<Variant> = args.iter().map(|&v| v.clone()).collect();
        let event = mapper(&owned, source_node_handle, source_entity);
        if let Some(event) = event {
            let _ = sender.send(Box::new(TypedEnvelope::<T>(event)));
        }
        Variant::nil()
    };

    let callable = Callable::from_fn(&format!("signal_handler_typed_{signal_name_copy}"), closure);
    node_ref.connect(signal_name, &callable);
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

        ensure_signal_connection_queue(app);

        // Install global typed signal channel and consolidated drain once
        if !app.world().contains_resource::<GlobalTypedSignalSender>() {
            let (sender, receiver) = crossbeam_channel::unbounded::<Box<dyn TypedDispatch>>();
            app.world_mut()
                .insert_resource(GlobalTypedSignalSender(sender));
            app.world_mut()
                .insert_resource(GlobalTypedSignalReceiver::new(receiver));

            // One consolidated drain for all typed messages
            app.add_systems(
                First,
                drain_global_typed_signals.before(message_update_system),
            );
        }

        // Per-T deferred connection processor - runs in First to enqueue connections,
        // which are then processed at end of frame in Last
        app.add_systems(First, process_typed_deferred_signal_connections::<T>);
    }
}

// Exclusive system to drain type-erased global queue into the correct Messages<T> resources
fn drain_global_typed_signals(world: &mut bevy_ecs::world::World) {
    // Collect first to avoid overlapping mutable borrows of `world`
    let mut pending: Vec<Box<dyn TypedDispatch>> = Vec::new();
    if let Some(receiver) = world.get_resource::<GlobalTypedSignalReceiver>() {
        let guard = receiver.0.lock();
        pending.extend(guard.try_iter());
    }
    for dispatch in pending.drain(..) {
        dispatch.write_into_world(world);
    }
}

/// SystemParam providing typed connect helpers for a specific Bevy `Message` T
#[derive(SystemParam)]
pub struct TypedGodotSignals<'w, T: Message + Send + 'static> {
    /// Global type-erased sender. Provided by first `GodotTypedSignalsPlugin` added.
    typed_sender: Res<'w, GlobalTypedSignalSender>,
    pending: Res<'w, PendingSignalConnections>,
    _marker: std::marker::PhantomData<T>,
}

impl<'w, T: Message + Send + 'static> TypedGodotSignals<'w, T> {
    /// Connect a Godot signal and map it to a typed Bevy Message `T` via `mapper`.
    /// Multiple connections are supported; each connection sends a `T` when fired.
    /// Connections are batched and applied at end of frame, ready for next frame's signals.
    pub fn connect_map<F>(
        &self,
        node: GodotNodeHandle,
        signal_name: &str,
        source_entity: Option<Entity>,
        mapper: F,
    ) where
        F: FnMut(&[Variant], GodotNodeHandle, Option<Entity>) -> Option<T> + Send + 'static,
    {
        self.pending.push(Box::new(PendingTypedSignalConnection {
            node,
            signal_name: signal_name.to_string(),
            source_entity,
            mapper: Box::new(mapper),
            sender: self.typed_sender.0.clone(),
        }));
    }
}

struct PendingTypedSignalConnection<T: Message + Send + 'static> {
    node: GodotNodeHandle,
    signal_name: String,
    source_entity: Option<Entity>,
    mapper:
        Box<dyn FnMut(&[Variant], GodotNodeHandle, Option<Entity>) -> Option<T> + Send + 'static>,
    sender: Sender<Box<dyn TypedDispatch>>,
}

impl<T: Message + Send + 'static> PendingSignalConnection for PendingTypedSignalConnection<T> {
    fn connect(self: Box<Self>, godot: &mut GodotAccess) {
        let PendingTypedSignalConnection {
            node,
            signal_name,
            source_entity,
            mapper,
            sender,
        } = *self;
        connect_typed_signal(godot, node, &signal_name, source_entity, mapper, sender);
    }
}

/// Process typed deferred signal connections for entities that now have GodotNodeHandles
fn process_typed_deferred_signal_connections<T: Message + Send + 'static>(
    mut commands: Commands,
    mut query: Query<(
        Entity,
        &GodotNodeHandle,
        &mut TypedDeferredSignalConnections<T>,
    )>,
    typed: TypedGodotSignals<T>,
) {
    for (entity, handle, mut deferred) in query.iter_mut() {
        for conn in deferred.connections.drain(..) {
            let signal = conn.signal_name;
            let mapper = conn.mapper;
            typed.connect_map(
                *handle,
                &signal,
                Some(entity),
                move |args, node_handle, ent| (mapper)(args, node_handle, ent),
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
        dyn Fn(&[Variant], GodotNodeHandle, Option<Entity>) -> Option<T> + Send + Sync + 'static,
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
        F: Fn(&[Variant], GodotNodeHandle, Option<Entity>) -> Option<T> + Send + Sync + 'static,
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
        F: Fn(&[Variant], GodotNodeHandle, Option<Entity>) -> Option<T> + Send + Sync + 'static,
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
            let source_node_id = GodotNodeHandle::from(target_node.instance_id());
            let typed_sender_copy = typed_sender.0.clone();
            let mapper = connection.mapper.clone();
            let signal_name = self.signal_name.clone();

            let closure = move |args: &[&Variant]| -> Variant {
                let owned: Vec<Variant> = args.iter().map(|&v| v.clone()).collect();
                if let Some(event) = mapper(&owned, source_node_id, Some(source_entity)) {
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
