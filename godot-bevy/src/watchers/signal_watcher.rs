use godot::classes::Node;
use godot::prelude::*;
use std::sync::mpsc::Sender;

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
