use godot::classes::Node;
use godot::obj::Gd;
use godot::prelude::*;
use std::sync::mpsc::Sender;

use crate::{
    interop::GodotNodeHandle,
    plugins::scene_tree::{SceneTreeMessage, SceneTreeMessageType},
};

#[derive(GodotClass)]
#[class(base=Node)]
pub struct SceneTreeWatcher {
    base: Base<Node>,
    pub notification_channel: Option<Sender<SceneTreeMessage>>,
}

#[godot_api]
impl INode for SceneTreeWatcher {
    fn init(base: Base<Node>) -> Self {
        Self {
            base,
            notification_channel: None,
        }
    }
}

#[godot_api]
impl SceneTreeWatcher {
    #[func]
    pub fn scene_tree_event(&self, node: Gd<Node>, message_type: SceneTreeMessageType) {
        if let Some(channel) = self.notification_channel.as_ref() {
            let _ = channel.send(SceneTreeMessage {
                node: GodotNodeHandle::from_instance_id(node.instance_id()),
                message_type,
                node_type: None, // No type optimization in basic method
            });
        }
    }

    #[func]
    pub fn scene_tree_event_typed(
        &self,
        node: Gd<Node>,
        message_type: SceneTreeMessageType,
        node_type: String,
    ) {
        if let Some(channel) = self.notification_channel.as_ref() {
            let _ = channel.send(SceneTreeMessage {
                node: GodotNodeHandle::from_instance_id(node.instance_id()),
                message_type,
                node_type: Some(node_type), // Pre-analyzed type from GDScript
            });
        }
    }
}
