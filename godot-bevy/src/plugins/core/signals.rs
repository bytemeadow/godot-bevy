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

    // Route to specific handlers for signals with arguments, generic handler for others
    match signal_name {
        "input_event" => {
            // For input_event, use specific handler that captures all 3 signal arguments
            node.connect(
                signal_name,
                &signal_watcher
                    .callable("handle_input_event")
                    .bind(&[node_clone.to_variant()]),
            );
        }
        "body_entered" => {
            // For body_entered, use specific handler that captures the body argument
            node.connect(
                signal_name,
                &signal_watcher
                    .callable("body_entered")
                    .bind(&[node_clone.to_variant()]),
            );
        }
        "area_entered" => {
            // For area_entered, use specific handler that captures the area argument
            node.connect(
                signal_name,
                &signal_watcher
                    .callable("area_entered")
                    .bind(&[node_clone.to_variant()]),
            );
        }
        _ => {
            // For other signals without arguments, use generic event handler
            node.connect(
                signal_name,
                &signal_watcher.callable("event").bind(&[
                    node_clone.to_variant(),  // origin
                    node_clone.to_variant(),  // target
                    signal_name.to_variant(), // signal name
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
