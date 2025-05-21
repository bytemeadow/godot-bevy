use bevy::{
    app::{App, First, Plugin},
    ecs::{
        event::{Event, EventWriter, event_update_system},
        schedule::IntoScheduleConfigs,
        system::NonSendMut,
    },
};
use godot::{builtin::{Variant, VariantArray}, classes::Node, meta::ToGodot, obj::{Gd, InstanceId}};

use crate::bridge::GodotNodeHandle;

use super::SceneTreeRef;

pub struct GodotSignalsPlugin;

impl Plugin for GodotSignalsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(First, write_godot_signal_events.before(event_update_system))
            .add_event::<GodotSignal>();
    }
}

#[derive(Debug, Event)]
pub struct GodotSignal {
    name: String,
    origin: GodotNodeHandle,
    // Instead of storing Vec<Variant> directly, we'll store the signal information as serialized strings
    // which are thread-safe
    serialized_args: Vec<String>,
}

impl GodotSignal {
    #[doc(hidden)]
    pub fn new(name: impl ToString, origin: Gd<Node>, args: Vec<Variant>) -> Self {
        // Convert each Variant to a string representation
        let serialized_args = args.into_iter()
            .map(|v| v.stringify().to_string())
            .collect();
        
        Self {
            name: name.to_string(),
            origin: GodotNodeHandle::from_instance_id(origin.instance_id()),
            serialized_args,
        }
    }
    
    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn origin(&self) -> GodotNodeHandle {
        self.origin.clone()
    }

    pub fn serialized_args(&self) -> &[String] {
        &self.serialized_args
    }
}

#[doc(hidden)]
pub struct GodotSignalReader(pub std::sync::mpsc::Receiver<GodotSignal>);

fn write_godot_signal_events(
    events: NonSendMut<GodotSignalReader>,
    mut event_writer: EventWriter<GodotSignal>,
) {
    event_writer.write_batch(events.0.try_iter());
}

pub fn connect_godot_signal(
    node: &mut GodotNodeHandle,
    signal_name: &str,
    scene_tree: &mut SceneTreeRef,
) {
    let mut node = node.get::<Node>();
    let signal_watcher = scene_tree
        .get()
        .get_root()
        .unwrap()
        .get_node_as::<Node>("/root/BevyAppSingleton/SignalWatcher");

    let node_clone = node.clone();

    node.connect(
        signal_name,
        &signal_watcher.callable("event").bind(&[
            signal_watcher.to_variant(),
            node_clone.to_variant(),
            signal_name.to_variant(),
        ]),
    );
}