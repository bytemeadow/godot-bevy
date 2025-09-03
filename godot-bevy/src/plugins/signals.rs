use bevy::{
    app::{App, First, Plugin},
    ecs::{
        entity::Entity,
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
    pub target: GodotSignalTarget,
    pub arguments: Vec<GodotSignalArgument>,
}

/// Represents the target of a Godot signal, which can be either a node or an entity.
/// Use with [`GodotSignals::connect_to_target`].
#[derive(Debug, Clone)]
pub enum GodotSignalTarget {
    Node(GodotNodeHandle),
    Entity(Entity),
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
    /// This version connects to the node's signal without a specific target
    /// Use it in cases where the "listener" is Bevy ECS itself and you can handle
    /// routing the event. This is similar to wiring up an event in Godot to a singleton
    /// and letting it handle all events.
    pub fn connect(&self, node: &mut GodotNodeHandle, signal_name: &str) {
        connect_godot_signal(node, signal_name, self.signal_sender.0.clone(), None);
    }

    /// Connect a Godot signal to a specific target in Bevy
    /// This version connects to a `GodotSignalTarget`, which can be a `GodotNodeHandle`
    /// or an `Entity`. This is more akin to the traditional Godot style of wiring up a
    /// specific object as a listener. Use it in cases where you want to make it easier
    /// to manage signal connections for specific entities or nodes.
    pub fn connect_to_target(
        &self,
        node: &mut GodotNodeHandle,
        signal_name: &str,
        target: &GodotSignalTarget,
    ) {
        connect_godot_signal(
            node,
            signal_name,
            self.signal_sender.0.clone(),
            Some(target.clone()),
        );
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
    signal_target: Option<GodotSignalTarget>,
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
        // If no target is specified, use the origin node as the target
        let target_handle = signal_target
            .clone()
            .unwrap_or_else(|| GodotSignalTarget::Node(origin_handle.clone()));

        let _ = signal_sender.send(GodotSignal {
            name: signal_name_copy.clone(),
            origin: origin_handle,
            target: target_handle,
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
