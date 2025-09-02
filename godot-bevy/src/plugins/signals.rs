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
    pub arguments: Vec<Variant>,
}

/// Event that captures any Godot signal emission - Send + Sync safe
#[derive(Debug, Event, Clone)]
pub struct GodotSignal {
    /// Name of the signal that was emitted
    pub signal_name: String,
    /// The Godot node that emitted the signal
    pub source_node: GodotNodeHandle,
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
        connect_godot_signal(node, signal_name, self.signal_sender.0.clone());
    }

    /// Connect multiple signals at once
    pub fn connect_many(&self, node: &mut GodotNodeHandle, signal_names: &[&str]) {
        for signal_name in signal_names {
            self.connect(node, signal_name);
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
            arguments,
        });

        Ok(Variant::nil())
    };

    let callable = Callable::from_local_fn(&format!("signal_handler_{}", signal_name), closure);

    node_ref.connect(signal_name, &callable);
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
                signal_sender.0.clone(),
            );
        }

        // Remove the component after processing
        commands
            .entity(entity)
            .remove::<DeferredSignalConnections>();
    }
}
