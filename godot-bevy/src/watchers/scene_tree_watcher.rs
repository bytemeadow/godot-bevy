use godot::classes::Node;
use godot::obj::{Gd, InstanceId};
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
        // Check if node is marked to be excluded from scene tree watcher
        // This is used by godot-bevy-inspector and other tools
        if node.has_meta("_bevy_exclude") {
            return;
        }

        if let Some(channel) = self.notification_channel.as_ref() {
            let _ = channel.send(SceneTreeMessage {
                node_id: GodotNodeHandle::from(node.instance_id()),
                message_type,
                node_type: None, // No type optimization in basic method
                node_name: None,
                parent_id: None,
                collision_mask: None,
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
        // Check if node is marked to be excluded from scene tree watcher
        // This is used by godot-bevy-inspector and other tools
        if node.has_meta("_bevy_exclude") {
            return;
        }

        if let Some(channel) = self.notification_channel.as_ref() {
            let _ = channel.send(SceneTreeMessage {
                node_id: GodotNodeHandle::from(node.instance_id()),
                message_type,
                node_type: Some(node_type), // Pre-analyzed type from GDScript
                node_name: None,
                parent_id: None,
                collision_mask: None,
            });
        }
    }

    #[func]
    pub fn scene_tree_event_typed_metadata(
        &self,
        node: Gd<Node>,
        message_type: SceneTreeMessageType,
        node_type: String,
        node_name: String,
        parent_id: i64,
        collision_mask: i64,
    ) {
        if node.has_meta("_bevy_exclude") {
            return;
        }

        let node_type = if node_type.is_empty() {
            None
        } else {
            Some(node_type)
        };
        let node_name = if node_name.is_empty() {
            None
        } else {
            Some(node_name)
        };
        let parent_id = if parent_id > 0 {
            Some(InstanceId::from_i64(parent_id))
        } else {
            None
        };
        let collision_mask = u8::try_from(collision_mask).ok();

        if let Some(channel) = self.notification_channel.as_ref() {
            let _ = channel.send(SceneTreeMessage {
                node_id: GodotNodeHandle::from(node.instance_id()),
                message_type,
                node_type,
                node_name,
                parent_id,
                collision_mask,
            });
        }
    }

    #[func]
    pub fn scene_tree_event_named(
        &self,
        node: Gd<Node>,
        message_type: SceneTreeMessageType,
        node_name: String,
    ) {
        if node.has_meta("_bevy_exclude") {
            return;
        }

        let node_name = if node_name.is_empty() {
            None
        } else {
            Some(node_name)
        };

        if let Some(channel) = self.notification_channel.as_ref() {
            let _ = channel.send(SceneTreeMessage {
                node_id: GodotNodeHandle::from(node.instance_id()),
                message_type,
                node_type: None,
                node_name,
                parent_id: None,
                collision_mask: None,
            });
        }
    }
}
