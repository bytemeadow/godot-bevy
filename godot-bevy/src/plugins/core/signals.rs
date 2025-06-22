use bevy::{
    app::{App, First, Plugin},
    ecs::{
        event::{Event, EventWriter, event_update_system},
        schedule::IntoScheduleConfigs,
        system::NonSendMut,
    },
};
use godot::{
    classes::Node,
    prelude::{Callable, Variant},
};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex, OnceLock};

use crate::bridge::GodotNodeHandle;
use crate::watchers::signal_watcher::GodotSignalWatcher;

use super::SceneTreeRef;

// Global channel for closure-based signal handling
static GLOBAL_SIGNAL_CHANNEL: OnceLock<Arc<Mutex<Option<Sender<GodotSignal>>>>> = OnceLock::new();

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

fn write_godot_signal_events(
    events: NonSendMut<GodotSignalReader>,
    mut event_writer: EventWriter<GodotSignal>,
) {
    event_writer.write_batch(events.0.try_iter());
}

// Initialize the global signal channel for closure-based signal handling
pub fn set_global_signal_channel(sender: Sender<GodotSignal>) {
    let channel = Arc::new(Mutex::new(Some(sender)));
    let _ = GLOBAL_SIGNAL_CHANNEL.set(channel);
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
        .get_node_as::<GodotSignalWatcher>("/root/BevyAppSingleton/SignalWatcher");

    let node_clone = node.clone();
    let signal_name_copy = signal_name.to_string();
    let node_id = node_clone.instance_id();

    // Set up the global channel if we have access to the watcher's channel
    let watcher_ref = signal_watcher.bind();
    if let Some(channel) = &watcher_ref.notification_channel {
        set_global_signal_channel(channel.clone());
    }

    // TRULY UNIVERSAL closure that handles ANY number of arguments
    let closure = move |args: &[&Variant]| -> Result<Variant, ()> {
        // Access the global signal channel
        if let Some(global_channel) = GLOBAL_SIGNAL_CHANNEL.get() {
            if let Ok(channel_guard) = global_channel.lock() {
                if let Some(ref sender) = *channel_guard {
                    // Convert all arguments to our signal argument format
                    let arguments: Vec<GodotSignalArgument> = args
                        .iter()
                        .map(|&arg| variant_to_signal_argument(arg))
                        .collect();

                    let origin_handle = GodotNodeHandle::from_instance_id(node_id);

                    let _ = sender.send(GodotSignal {
                        name: signal_name_copy.clone(),
                        origin: origin_handle.clone(),
                        target: origin_handle,
                        arguments,
                    });
                }
            }
        }

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

    GodotSignalArgument { type_name, value }
}
