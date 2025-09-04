use bevy::{
    app::{App, First, Plugin},
    ecs::{
        component::Component,
        entity::Entity,
        event::{Event, EventWriter, event_update_system},
        schedule::IntoScheduleConfigs,
        system::{Commands, NonSendMut, Query, SystemParam},
    },
};
use godot::{
    classes::Node,
    prelude::{Callable, Variant},
};

use crate::interop::GodotNodeHandle;

#[derive(Default)]
pub struct GodotSignalsPlugin;

impl Plugin for GodotSignalsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            First,
            (
                write_godot_signal_events.before(event_update_system),
                process_deferred_signal_connections,
            ),
        )
        .add_event::<GodotSignal>();
    }
}

/// Raw signal data from Godot - contains non-Send Variant data
#[derive(Debug, Clone)]
pub struct RawGodotSignal {
    pub signal_name: String,
    pub source_node: GodotNodeHandle,
    pub source_entity: Option<Entity>,
    pub arguments: Vec<Variant>,
}

/// Event that captures any Godot signal emission - Send + Sync safe
#[derive(Debug, Event, Clone)]
pub struct GodotSignal {
    /// Name of the signal that was emitted
    pub signal_name: String,
    /// The Godot node that emitted the signal
    pub source_node: GodotNodeHandle,
    /// Optional Bevy entity associated with the source node
    pub source_entity: Option<Entity>,
    /// String representations of arguments for debugging
    pub argument_strings: Vec<String>,
}

impl GodotSignal {
    pub fn is_from(&self, signal_name: &str) -> bool {
        self.signal_name == signal_name
    }

    pub fn get_arg_string(&self, index: usize) -> Option<&str> {
        self.argument_strings.get(index).map(|s| s.as_str())
    }

    /// Check if this signal came from a specific node
    pub fn is_from_node(&self, node: &GodotNodeHandle) -> bool {
        self.source_node == *node
    }

    /// Check if this signal is from a specific signal name AND node
    pub fn is_from_node_signal(&self, node: &GodotNodeHandle, signal_name: &str) -> bool {
        self.signal_name == signal_name && self.source_node == *node
    }

    /// Find the entity that owns this signal's source node
    pub fn find_entity(&self, query: &Query<(Entity, &GodotNodeHandle)>) -> Option<Entity> {
        query.iter().find_map(|(entity, handle)| {
            if *handle == self.source_node {
                Some(entity)
            } else {
                None
            }
        })
    }
}

/// Component for deferred signal connections - applied to entities
/// that need signals connected once their GodotNodeHandle is available
#[derive(Component)]
pub struct DeferredSignalConnections {
    pub connections: Vec<String>,
}

impl DeferredSignalConnections {
    pub fn new(signals: Vec<String>) -> Self {
        Self {
            connections: signals,
        }
    }

    pub fn single(signal: impl Into<String>) -> Self {
        Self {
            connections: vec![signal.into()],
        }
    }
}

#[doc(hidden)]
pub struct GodotSignalReader(pub std::sync::mpsc::Receiver<RawGodotSignal>);

#[doc(hidden)]
pub struct GodotSignalSender(pub std::sync::mpsc::Sender<RawGodotSignal>);

/// System parameter for connecting Godot signals to Bevy's event system
#[derive(SystemParam)]
pub struct GodotSignals<'w> {
    signal_sender: NonSendMut<'w, GodotSignalSender>,
}

impl<'w> GodotSignals<'w> {
    /// Connect to a Godot signal from an existing node
    pub fn connect(&self, node: &mut GodotNodeHandle, signal_name: &str) {
        connect_godot_signal(node, signal_name, None, self.signal_sender.0.clone());
    }

    /// Connect and associate with a Bevy entity for easier querying
    pub fn connect_with_entity(
        &self,
        node: &mut GodotNodeHandle,
        signal_name: &str,
        entity: Entity,
    ) {
        connect_godot_signal(
            node,
            signal_name,
            Some(entity),
            self.signal_sender.0.clone(),
        );
    }

    /// Connect multiple signals at once
    pub fn connect_many(&self, node: &mut GodotNodeHandle, signal_names: &[&str]) {
        for signal_name in signal_names {
            self.connect(node, signal_name);
        }
    }

    /// Connect multiple signals at once and associate with a Bevy entity
    pub fn connect_many_with_entity(&self, node: &mut GodotNodeHandle, signal_names: &[&str], entity: Entity) {
        for signal_name in signal_names {
            self.connect_with_entity(node, signal_name, entity);
        }
    }
}

fn write_godot_signal_events(
    events: NonSendMut<GodotSignalReader>,
    mut event_writer: EventWriter<GodotSignal>,
) {
    for raw_signal in events.0.try_iter() {
        // Convert raw signal to Send-safe event
        let signal = GodotSignal {
            signal_name: raw_signal.signal_name,
            source_node: raw_signal.source_node,
            source_entity: raw_signal.source_entity,
            argument_strings: raw_signal
                .arguments
                .iter()
                .map(|v| v.stringify().to_string())
                .collect(),
        };
        event_writer.write(signal);
    }
}

pub fn connect_godot_signal(
    node: &mut GodotNodeHandle,
    signal_name: &str,
    source_entity: Option<Entity>,
    signal_sender: std::sync::mpsc::Sender<RawGodotSignal>,
) {
    let mut node_ref = node.get::<Node>();
    let signal_name_copy = signal_name.to_string();
    let source_node = node.clone();

    let closure = move |args: &[&Variant]| -> Result<Variant, ()> {
        let arguments: Vec<Variant> = args.iter().map(|&v| v.clone()).collect();

        let _ = signal_sender.send(RawGodotSignal {
            signal_name: signal_name_copy.clone(),
            source_node: source_node.clone(),
            source_entity,
            arguments,
        });

        Ok(Variant::nil())
    };

    let callable = Callable::from_local_fn(&format!("signal_handler_{}", signal_name), closure);

    node_ref.connect(signal_name, &callable);
}

/// Extension trait for EventReader<GodotSignal> to add syntax sugar
pub trait GodotSignalReaderExt {
    /// Process signals of any type
    fn handle_signal(&mut self, signal_name: &str) -> SignalMatcher<'_>;

    /// Process signals matching a custom predicate
    fn handle_matching<F>(&mut self, predicate: F) -> SignalMatcher<'_>
    where
        F: Fn(&GodotSignal) -> bool;

    /// Process all signals (no filtering)
    fn handle_all(&mut self) -> SignalMatcher<'_>;

    /// Process multiple signal types at once
    fn handle_signals(&mut self, signal_names: &[&str]) -> SignalMatcher<'_>;
}

impl GodotSignalReaderExt for bevy::ecs::event::EventReader<'_, '_, GodotSignal> {
    fn handle_signal(&mut self, signal_name: &str) -> SignalMatcher<'_> {
        SignalMatcher::from_signals(self.read().filter(|s| s.is_from(signal_name)).collect())
    }

    fn handle_matching<F>(&mut self, predicate: F) -> SignalMatcher<'_>
    where
        F: Fn(&GodotSignal) -> bool,
    {
        SignalMatcher::from_signals(self.read().filter(|s| predicate(s)).collect())
    }

    fn handle_all(&mut self) -> SignalMatcher<'_> {
        SignalMatcher::from_signals(self.read().collect())
    }

    fn handle_signals(&mut self, signal_names: &[&str]) -> SignalMatcher<'_> {
        SignalMatcher::from_signals(
            self.read()
                .filter(|s| signal_names.iter().any(|name| s.is_from(name)))
                .collect(),
        )
    }
}

/// Helper for chaining signal handling operations
pub struct SignalMatcher<'a> {
    signals: Vec<&'a GodotSignal>,
}

impl<'a> SignalMatcher<'a> {
    /// Create a new SignalMatcher from a collection of signals
    pub fn from_signals(signals: Vec<&'a GodotSignal>) -> Self {
        Self { signals }
    }

    /// Create an empty SignalMatcher
    pub fn empty() -> Self {
        Self {
            signals: Vec::new(),
        }
    }

    /// Create a SignalMatcher from a slice of signals
    pub fn from_slice(signals: &[&'a GodotSignal]) -> Self {
        Self {
            signals: signals.to_vec(),
        }
    }
    /// Handle signals from a specific node
    pub fn from_node<F>(self, node: &GodotNodeHandle, mut handler: F) -> Self
    where
        F: FnMut(&GodotSignal),
    {
        for signal in &self.signals {
            if signal.is_from_node(node) {
                handler(signal);
            }
        }
        self
    }

    /// Handle signals from any of the provided nodes
    pub fn from_any_node<F>(self, nodes: &[&GodotNodeHandle], mut handler: F) -> Self
    where
        F: FnMut(&GodotSignal),
    {
        for signal in &self.signals {
            if nodes.iter().any(|node| signal.is_from_node(node)) {
                handler(signal);
            }
        }
        self
    }

    /// Handle all remaining signals (catch-all)
    pub fn any<F>(self, mut handler: F)
    where
        F: FnMut(&GodotSignal),
    {
        for signal in &self.signals {
            handler(signal);
        }
    }

    /// Filter signals by a custom predicate and continue chaining
    pub fn matching<F>(self, predicate: F) -> Self
    where
        F: Fn(&GodotSignal) -> bool,
    {
        SignalMatcher::from_signals(self.signals.into_iter().filter(|s| predicate(s)).collect())
    }

    /// Handle the first signal that matches, then stop processing
    pub fn first<F>(self, mut handler: F)
    where
        F: FnMut(&GodotSignal),
    {
        if let Some(signal) = self.signals.first() {
            handler(signal);
        }
    }

    /// Get the count of signals in this matcher
    pub fn count(&self) -> usize {
        self.signals.len()
    }

    /// Check if there are any signals in this matcher
    pub fn is_empty(&self) -> bool {
        self.signals.is_empty()
    }

    /// Get an iterator over the signals
    pub fn iter(&self) -> impl Iterator<Item = &GodotSignal> {
        self.signals.iter().copied()
    }
}

/// Process deferred signal connections for entities that now have GodotNodeHandles
fn process_deferred_signal_connections(
    mut commands: Commands,
    mut query: Query<(Entity, &mut GodotNodeHandle, &DeferredSignalConnections)>,
    signal_sender: NonSendMut<GodotSignalSender>,
) {
    for (entity, mut handle, deferred) in query.iter_mut() {
        for signal_name in &deferred.connections {
            connect_godot_signal(
                &mut handle,
                signal_name,
                Some(entity),
                signal_sender.0.clone(),
            );
        }

        // Remove the component after processing
        commands
            .entity(entity)
            .remove::<DeferredSignalConnections>();
    }
}
