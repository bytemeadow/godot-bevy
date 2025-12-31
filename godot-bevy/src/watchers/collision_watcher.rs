use godot::classes::Node;
use godot::obj::Gd;
use godot::prelude::*;
use std::sync::mpsc::Sender;

use crate::{
    interop::GodotNodeHandle,
    plugins::collisions::{CollisionMessage, CollisionMessageType},
};

#[derive(GodotClass)]
#[class(base=Node)]
pub struct CollisionWatcher {
    base: Base<Node>,
    pub notification_channel: Option<Sender<CollisionMessage>>,
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
        colliding_body: Gd<Node>,
        origin_node: Gd<Node>,
        event_type: CollisionMessageType,
    ) {
        if let Some(channel) = self.notification_channel.as_ref() {
            let _ = channel.send(CollisionMessage {
                event_type,
                origin: GodotNodeHandle::from(origin_node.instance_id()),
                target: GodotNodeHandle::from(colliding_body.instance_id()),
            });
        }
    }
}
