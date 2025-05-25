use godot::classes::Node;
use godot::obj::Gd;
use godot::prelude::*;
use std::sync::mpsc::Sender;

use crate::{
    bridge::GodotNodeHandle,
    plugins::core::{CollisionEvent, CollisionEventType},
};

#[derive(GodotClass)]
#[class(base=Node)]
pub struct CollisionWatcher {
    base: Base<Node>,
    pub notification_channel: Option<Sender<CollisionEvent>>,
}

#[godot_api]
impl INode for CollisionWatcher {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            notification_channel: None,
        }
    }
}

#[godot_api]
impl CollisionWatcher {
    #[func]
    pub fn collision_event(
        &self,
        target: Gd<Node>,
        origin: Gd<Node>,
        event_type: CollisionEventType,
    ) {
        if let Some(channel) = self.notification_channel.as_ref() {
            let _ = channel.send(CollisionEvent {
                event_type,
                origin: GodotNodeHandle::from_instance_id(origin.instance_id()),
                target: GodotNodeHandle::from_instance_id(target.instance_id()),
            });
        }
    }
}
