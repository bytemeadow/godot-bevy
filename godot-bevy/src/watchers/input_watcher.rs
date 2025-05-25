use godot::classes::{InputEvent, Node};
use godot::obj::Gd;
use godot::prelude::*;
use std::sync::mpsc::Sender;

use crate::plugins::core::input_event::InputEventType;

#[derive(GodotClass)]
#[class(base=Node)]
pub struct GodotInputWatcher {
    base: Base<Node>,
    pub notification_channel: Option<Sender<(InputEventType, Gd<InputEvent>)>>,
}

#[godot_api]
impl INode for GodotInputWatcher {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            notification_channel: None,
        }
    }
}

#[godot_api]
impl GodotInputWatcher {
    #[func]
    pub fn unhandled_input(&self, input_event: Gd<InputEvent>) {
        if let Some(channel) = self.notification_channel.as_ref() {
            let _ = channel.send((InputEventType::Unhandled, input_event));
        }
    }

    #[func]
    pub fn input(&self, input_event: Gd<InputEvent>) {
        if let Some(channel) = self.notification_channel.as_ref() {
            let _ = channel.send((InputEventType::Normal, input_event));
        }
    }
}
