use bevy::{
    app::{App, First, Plugin},
    ecs::{
        event::{Event, EventWriter, event_update_system},
        schedule::IntoScheduleConfigs,
        system::NonSendMut,
    },
};
use godot::{classes::Node, meta::ToGodot, prelude::Variant};

use crate::bridge::GodotNodeHandle;

use super::SceneTreeRef;

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

    // Revert to working approach but use correct binding for input_event
    match signal_name {
        "input_event" => {
            // For input_event, use a callable that directly connects the signal arguments
            node.connect(
                signal_name,
                &signal_watcher
                    .callable("handle_input_event")
                    .bind(&[node_clone.to_variant()]),
            );
        }
        _ => {
            // For other signals, use the standard event handler with binding
            node.connect(
                signal_name,
                &signal_watcher.callable("event").bind(&[
                    signal_watcher.to_variant(),
                    node_clone.to_variant(),
                    signal_name.to_variant(),
                ]),
            );
        }
    }
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
