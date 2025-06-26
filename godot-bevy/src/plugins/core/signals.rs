use bevy::{
    app::{App, First, Plugin},
    ecs::{
        event::{Event, EventWriter, event_update_system},
        schedule::IntoScheduleConfigs,
        system::{NonSendMut, SystemParam},
    },
};
use godot::{
    classes::{Node, Object},
    obj::{Gd, InstanceId},
    prelude::{Callable, Variant},
};
use std::sync::mpsc::Sender;

use crate::bridge::GodotNodeHandle;

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

/// Clean API for connecting Godot signals - hides implementation details from users
#[derive(SystemParam)]
pub struct GodotSignals<'w> {
    signal_sender: NonSendMut<'w, GodotSignalSender>,
}

impl<'w> GodotSignals<'w> {
    /// Connect a Godot signal to be forwarded to Bevy's event system
    pub fn connect(&self, node: &mut GodotNodeHandle, signal_name: &str) {
        connect_godot_signal(node, signal_name, self.signal_sender.0.clone());
    }
}

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

    // TRULY UNIVERSAL closure that handles ANY number of arguments
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
