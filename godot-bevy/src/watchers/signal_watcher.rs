use godot::classes::{InputEvent, Node};
use godot::obj::Gd;
use godot::prelude::*;
use std::sync::mpsc::Sender;

use crate::bridge::GodotNodeHandle;
use crate::plugins::core::GodotSignal;

#[derive(GodotClass)]
#[class(base=Node)]
pub struct GodotSignalWatcher {
    base: Base<Node>,
    pub notification_channel: Option<Sender<GodotSignal>>,
}

#[godot_api]
impl INode for GodotSignalWatcher {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            notification_channel: None,
        }
    }
}

#[godot_api]
impl GodotSignalWatcher {
    #[func]
    pub fn event(&self, origin: Gd<Node>, target: Gd<Node>, signal_name: GString) {
        if let Some(channel) = self.notification_channel.as_ref() {
            let _ = channel.send(GodotSignal {
                name: signal_name.to_string(),
                origin: GodotNodeHandle::from_instance_id(origin.instance_id()),
                target: GodotNodeHandle::from_instance_id(target.instance_id()),
                arguments: vec![],
            });
        }
    }

    #[func]
    pub fn body_entered(&self, body: Gd<Node>, source_node: Gd<Node>) {
        if let Some(channel) = self.notification_channel.as_ref() {
            let args = vec![crate::plugins::core::variant_to_signal_argument(
                &body.to_variant(),
            )];

            let source_handle = GodotNodeHandle::from_instance_id(source_node.instance_id());

            let _ = channel.send(GodotSignal {
                name: "body_entered".to_string(),
                origin: source_handle.clone(),
                target: source_handle,
                arguments: args,
            });
        }
    }

    #[func]
    pub fn area_entered(&self, area: Gd<Node>, source_node: Gd<Node>) {
        if let Some(channel) = self.notification_channel.as_ref() {
            let args = vec![crate::plugins::core::variant_to_signal_argument(
                &area.to_variant(),
            )];

            let source_handle = GodotNodeHandle::from_instance_id(source_node.instance_id());

            let _ = channel.send(GodotSignal {
                name: "area_entered".to_string(),
                origin: source_handle.clone(),
                target: source_handle,
                arguments: args,
            });
        }
    }

    #[func]
    pub fn handle_input_event(
        &self,
        viewport: Gd<Node>,
        event: Gd<InputEvent>,
        shape_idx: i32,
        source_node: Gd<Node>,
    ) {
        if let Some(channel) = self.notification_channel.as_ref() {
            let args = vec![
                crate::plugins::core::variant_to_signal_argument(&viewport.to_variant()),
                crate::plugins::core::variant_to_signal_argument(&event.to_variant()),
                crate::plugins::core::variant_to_signal_argument(&shape_idx.to_variant()),
            ];

            // Use the bound source node as the origin
            let source_handle = GodotNodeHandle::from_instance_id(source_node.instance_id());

            let _ = channel.send(GodotSignal {
                name: "input_event".to_string(),
                origin: source_handle.clone(),
                target: source_handle,
                arguments: args,
            });
        }
    }

    #[func]
    pub fn collision_event(&self, target: Gd<Node>, origin: Gd<Node>, signal_name: GString) {
        if let Some(channel) = self.notification_channel.as_ref() {
            let _ = channel.send(GodotSignal {
                name: signal_name.to_string(),
                origin: GodotNodeHandle::from_instance_id(origin.instance_id()),
                target: GodotNodeHandle::from_instance_id(target.instance_id()),
                arguments: vec![],
            });
        }
    }
}
