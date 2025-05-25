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

    fn ready(&mut self) {
        // Enable input processing for this node
        self.base_mut().set_process_input(true);
        self.base_mut().set_process_unhandled_input(true);
    }

    fn input(&mut self, event: Gd<InputEvent>) {
        if let Some(channel) = self.notification_channel.as_ref() {
            let _ = channel.send((InputEventType::Normal, event));
        }
    }

    fn unhandled_input(&mut self, event: Gd<InputEvent>) {
        if let Some(channel) = self.notification_channel.as_ref() {
            let _ = channel.send((InputEventType::Unhandled, event));
        }
    }
}
