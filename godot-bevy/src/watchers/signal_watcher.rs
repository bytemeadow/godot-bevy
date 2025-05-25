use godot::classes::Node;
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
            });
        }
    }
}
