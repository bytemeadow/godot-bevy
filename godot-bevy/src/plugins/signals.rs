use crate::interop::{GodotAccess, GodotNodeHandle};
use bevy_app::{App, First, Last, Plugin};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    event::Event,
    prelude::Resource,
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

/// Trait for type-erased signal dispatch that triggers observers
pub(crate) trait SignalDispatch: Send {
    fn trigger_in_world(self: Box<Self>, world: &mut bevy_ecs::world::World);
}

/// Envelope that carries a signal event for observer triggering
struct SignalEnvelope<T: Event + Clone + Send + 'static> {
    event: T,
}

impl<T: Event + Clone + Send + 'static> SignalDispatch for SignalEnvelope<T>
where
    for<'a> T::Trigger<'a>: Default,
{
    fn trigger_in_world(self: Box<Self>, world: &mut bevy_ecs::world::World) {
        world.trigger(self.event);
    }
}

/// Resource for receiving signal dispatches from Godot callbacks.
/// Wrapped in Mutex to be Send+Sync, allowing it to be a regular Bevy Resource.
#[derive(Resource)]
pub(crate) struct SignalReceiver(pub Mutex<crossbeam_channel::Receiver<Box<dyn SignalDispatch>>>);

impl SignalReceiver {
    pub fn new(receiver: crossbeam_channel::Receiver<Box<dyn SignalDispatch>>) -> Self {
        Self(Mutex::new(receiver))
    }
}

#[doc(hidden)]
#[derive(Resource)]
pub(crate) struct SignalSender(pub crossbeam_channel::Sender<Box<dyn SignalDispatch>>);

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

fn connect_signal<T>(
    godot: &mut GodotAccess,
    node: GodotNodeHandle,
    signal_name: &str,
    source_entity: Option<Entity>,
    mapper: Box<
        dyn FnMut(&[Variant], GodotNodeHandle, Option<Entity>) -> Option<T> + Send + 'static,
    >,
    sender: Sender<Box<dyn SignalDispatch>>,
) where
    T: Event + Clone + Send + 'static,
    for<'a> T::Trigger<'a>: Default,
{
    let mut node_ref = godot.get::<Node>(node);
    let signal_name_copy = signal_name.to_string();
    let source_node_handle = node;
    let mut mapper = mapper;

    let closure = move |args: &[&Variant]| -> Variant {
        // Clone variants to owned values we can inspect
        let owned: Vec<Variant> = args.iter().map(|&v| v.clone()).collect();
        let event = mapper(&owned, source_node_handle, source_entity);
        if let Some(event) = event {
            let _ = sender.send(Box::new(SignalEnvelope { event }));
        }
        Variant::nil()
    };

    let callable = Callable::from_fn(&format!("signal_handler_{signal_name_copy}"), closure);
    node_ref.connect(signal_name, &callable);
}

/// Plugin to enable Godot signal to Bevy observer routing for event type `T`.
///
/// When a Godot signal is connected via [`GodotSignals`], it triggers Bevy observers
/// for the event type `T`. This provides a reactive, entity-targeted way to handle
/// Godot signals.
///
/// # Example
///
/// ```ignore
/// use bevy::prelude::*;
/// use godot_bevy::prelude::*;
///
/// #[derive(Event, Clone)]
/// struct ButtonPressed;
///
/// fn setup_app(app: &mut App) {
///     app.add_plugins(GodotSignalsPlugin::<ButtonPressed>::default());
///
///     // React to button presses with a global observer
///     app.add_observer(|_trigger: Trigger<ButtonPressed>| {
///         println!("A button was pressed!");
///     });
/// }
///
/// fn connect_button(
///     button_handle: GodotNodeHandle,
///     signals: GodotSignals<ButtonPressed>,
/// ) {
///     signals.connect(button_handle, "pressed", None, |_, _, _| {
///         Some(ButtonPressed)
///     });
/// }
/// ```
pub struct GodotSignalsPlugin<T>
where
    T: Event + Clone + Send + 'static,
    for<'a> T::Trigger<'a>: Default,
{
    _phantom: std::marker::PhantomData<T>,
}

impl<T> Default for GodotSignalsPlugin<T>
where
    T: Event + Clone + Send + 'static,
    for<'a> T::Trigger<'a>: Default,
{
    fn default() -> Self {
        Self {
            _phantom: Default::default(),
        }
    }
}

impl<T> Plugin for GodotSignalsPlugin<T>
where
    T: Event + Clone + Send + 'static,
    for<'a> T::Trigger<'a>: Default,
{
    fn build(&self, app: &mut App) {
        ensure_signal_connection_queue(app);

        // Install global signal channel and drain system once
        if !app.world().contains_resource::<SignalSender>() {
            let (sender, receiver) = crossbeam_channel::unbounded::<Box<dyn SignalDispatch>>();
            app.world_mut().insert_resource(SignalSender(sender));
            app.world_mut()
                .insert_resource(SignalReceiver::new(receiver));

            // Drain signals and trigger observers
            app.add_systems(First, drain_and_trigger_signals);
        }

        // Per-T deferred connection processor
        app.add_systems(First, process_deferred_signal_connections::<T>);
    }
}

/// Exclusive system to drain signal queue and trigger observers
fn drain_and_trigger_signals(world: &mut bevy_ecs::world::World) {
    // Collect first to avoid overlapping mutable borrows of `world`
    let mut pending: Vec<Box<dyn SignalDispatch>> = Vec::new();
    if let Some(receiver) = world.get_resource::<SignalReceiver>() {
        let guard = receiver.0.lock();
        pending.extend(guard.try_iter());
    }
    for dispatch in pending.drain(..) {
        dispatch.trigger_in_world(world);
    }
}

/// SystemParam for connecting Godot signals to Bevy observers.
///
/// Use this to connect a Godot node's signal to trigger a Bevy event `T`.
/// The event will be delivered to observers registered with `app.add_observer()`
/// or entity-specific observers added with `commands.entity(e).observe()`.
///
/// # Example
///
/// ```ignore
/// fn connect_signals(
///     button: Query<&GodotNodeHandle, With<MyButton>>,
///     signals: GodotSignals<ButtonPressed>,
/// ) {
///     if let Ok(handle) = button.single() {
///         // Connect the Godot "pressed" signal to trigger ButtonPressed event
///         signals.connect(*handle, "pressed", None, |_, _, _| {
///             Some(ButtonPressed)
///         });
///     }
/// }
/// ```
#[derive(SystemParam)]
pub struct GodotSignals<'w, T>
where
    T: Event + Clone + Send + 'static,
    for<'a> T::Trigger<'a>: Default,
{
    sender: Res<'w, SignalSender>,
    pending: Res<'w, PendingSignalConnections>,
    _marker: std::marker::PhantomData<T>,
}

impl<'w, T> GodotSignals<'w, T>
where
    T: Event + Clone + Send + 'static,
    for<'a> T::Trigger<'a>: Default,
{
    /// Connect a Godot signal to trigger a Bevy event `T`.
    ///
    /// When the signal fires, the `mapper` function is called with the signal arguments.
    /// If it returns `Some(event)`, that event is triggered for observers.
    ///
    /// # Arguments
    ///
    /// * `node` - The Godot node to connect the signal from
    /// * `signal_name` - The name of the Godot signal (e.g., "pressed", "body_entered")
    /// * `source_entity` - Optional entity to include in the mapper callback
    /// * `mapper` - Function to convert signal arguments to the event type
    pub fn connect<F>(
        &self,
        node: GodotNodeHandle,
        signal_name: &str,
        source_entity: Option<Entity>,
        mapper: F,
    ) where
        F: FnMut(&[Variant], GodotNodeHandle, Option<Entity>) -> Option<T> + Send + 'static,
    {
        self.pending.push(Box::new(PendingSignalConnectionImpl {
            node,
            signal_name: signal_name.to_string(),
            source_entity,
            mapper: Box::new(mapper),
            sender: self.sender.0.clone(),
            _marker: std::marker::PhantomData,
        }));
    }
}

/// Backwards compatibility alias
#[deprecated(note = "Use GodotSignals instead")]
pub type TypedGodotSignals<'w, T> = GodotSignals<'w, T>;

struct PendingSignalConnectionImpl<T>
where
    T: Event + Clone + Send + 'static,
    for<'a> T::Trigger<'a>: Default,
{
    node: GodotNodeHandle,
    signal_name: String,
    source_entity: Option<Entity>,
    mapper:
        Box<dyn FnMut(&[Variant], GodotNodeHandle, Option<Entity>) -> Option<T> + Send + 'static>,
    sender: Sender<Box<dyn SignalDispatch>>,
    _marker: std::marker::PhantomData<T>,
}

impl<T> PendingSignalConnection for PendingSignalConnectionImpl<T>
where
    T: Event + Clone + Send + 'static,
    for<'a> T::Trigger<'a>: Default,
{
    fn connect(self: Box<Self>, godot: &mut GodotAccess) {
        let PendingSignalConnectionImpl {
            node,
            signal_name,
            source_entity,
            mapper,
            sender,
            _marker: _,
        } = *self;
        connect_signal(godot, node, &signal_name, source_entity, mapper, sender);
    }
}

/// Process deferred signal connections for entities that now have GodotNodeHandles
fn process_deferred_signal_connections<T>(
    mut commands: Commands,
    mut query: Query<(Entity, &GodotNodeHandle, &mut DeferredSignalConnections<T>)>,
    signals: GodotSignals<T>,
) where
    T: Event + Clone + Send + 'static,
    for<'a> T::Trigger<'a>: Default,
{
    for (entity, handle, mut deferred) in query.iter_mut() {
        for conn in deferred.connections.drain(..) {
            let signal = conn.signal_name;
            let mapper = conn.mapper;
            signals.connect(
                *handle,
                &signal,
                Some(entity),
                move |args, node_handle, ent| (mapper)(args, node_handle, ent),
            );
        }
        // Remove marker after wiring all deferred connections
        commands
            .entity(entity)
            .remove::<DeferredSignalConnections<T>>();
    }
}

// ====================
// Deferred Connections
// ====================

/// A single deferred signal connection for event type `T`
pub struct DeferredConnection<T: Event + Clone + Send + 'static> {
    /// The signal name to connect to
    pub signal_name: String,
    /// Mapper function to convert signal arguments to the event
    pub mapper: Arc<
        dyn Fn(&[Variant], GodotNodeHandle, Option<Entity>) -> Option<T> + Send + Sync + 'static,
    >,
}

impl<T: Event + Clone + Send + 'static> Debug for DeferredConnection<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "DeferredConnection {{ signal_name: {:?} }}",
            self.signal_name
        )
    }
}

/// Component to defer Godot signal connections until a `GodotNodeHandle` exists on the entity.
///
/// Add this component to an entity when you want to connect signals but the entity
/// doesn't have a `GodotNodeHandle` yet. Once the handle is available, the connections
/// will be automatically established.
#[derive(Component, Debug)]
pub struct DeferredSignalConnections<T: Event + Clone + Send + 'static> {
    /// The pending connections to establish
    pub connections: Vec<DeferredConnection<T>>,
}

impl<T: Event + Clone + Send + 'static> Default for DeferredSignalConnections<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Event + Clone + Send + 'static> DeferredSignalConnections<T> {
    /// Create an empty deferred connections component
    pub fn new() -> Self {
        Self {
            connections: Vec::new(),
        }
    }

    /// Create with a single connection
    pub fn with_connection<F>(signal_name: impl Into<String>, mapper: F) -> Self
    where
        F: Fn(&[Variant], GodotNodeHandle, Option<Entity>) -> Option<T> + Send + Sync + 'static,
    {
        Self {
            connections: vec![DeferredConnection {
                signal_name: signal_name.into(),
                mapper: Arc::new(mapper),
            }],
        }
    }

    /// Add a connection to establish once the node handle is available
    pub fn push<F>(&mut self, signal_name: impl Into<String>, mapper: F)
    where
        F: Fn(&[Variant], GodotNodeHandle, Option<Entity>) -> Option<T> + Send + Sync + 'static,
    {
        self.connections.push(DeferredConnection {
            signal_name: signal_name.into(),
            mapper: Arc::new(mapper),
        });
    }
}

/// Backwards compatibility alias
#[deprecated(note = "Use DeferredSignalConnections instead")]
pub type TypedDeferredSignalConnections<T> = DeferredSignalConnections<T>;

/// Backwards compatibility alias
#[deprecated(note = "Use DeferredConnection instead")]
pub type TypedDeferredConnection<T> = DeferredConnection<T>;

/// Type-erased deferred connections for internal use
#[doc(hidden)]
pub(crate) trait DeferredSignalConnectionTrait: Send + Sync + Debug {
    fn connect(&self, root_node: &Gd<Node>, entity: Entity, sender: &SignalSender);
}

/// Deferred connection specification for packed scenes
#[doc(hidden)]
#[derive(Debug)]
pub(crate) struct SignalConnectionSpec<T>
where
    T: Event + Clone + Send + 'static,
    for<'a> T::Trigger<'a>: Default,
{
    pub(crate) node_path: String,
    pub(crate) signal_name: String,
    pub(crate) connections: DeferredSignalConnections<T>,
}

#[doc(hidden)]
impl<T> DeferredSignalConnectionTrait for SignalConnectionSpec<T>
where
    T: Event + Clone + Send + Debug + 'static,
    for<'a> T::Trigger<'a>: Default,
{
    fn connect(&self, root_node: &Gd<Node>, source_entity: Entity, sender: &SignalSender) {
        let Some(mut target_node) = root_node.get_node_or_null(self.node_path.as_str()) else {
            error!(
                "Failed to find node at path '{}' for signal connection",
                self.node_path
            );
            return;
        };

        for connection in self.connections.connections.iter() {
            let source_node_id = GodotNodeHandle::from(target_node.instance_id());
            let sender_copy = sender.0.clone();
            let mapper = connection.mapper.clone();
            let signal_name = self.signal_name.clone();

            let closure = move |args: &[&Variant]| -> Variant {
                let owned: Vec<Variant> = args.iter().map(|&v| v.clone()).collect();
                if let Some(event) = mapper(&owned, source_node_id, Some(source_entity)) {
                    let _ = sender_copy.send(Box::new(SignalEnvelope { event }));
                }
                Variant::nil()
            };

            target_node.connect(
                &signal_name,
                &Callable::from_fn(&format!("signal_handler_{signal_name}"), closure),
            );
        }
    }
}
