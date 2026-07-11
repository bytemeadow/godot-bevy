use crossbeam_channel::Sender;
use godot::classes::Node;
use godot::obj::{Gd, InstanceId};
use godot::prelude::*;

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
        // Fallback direct-signal entry. The optimized GDScript watcher filters excluded
        // subtrees before they cross FFI, so only this path re-checks in Rust.
        if matches!(message_type, SceneTreeMessageType::NodeAdded) && is_excluded_from_mirror(&node)
        {
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
                groups: None,
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
                node_id: GodotNodeHandle::from(node.instance_id()),
                message_type,
                node_type: Some(node_type), // Pre-analyzed type from GDScript
                node_name: None,
                parent_id: None,
                collision_mask: None,
                groups: None,
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
                groups: None,
            });
        }
    }

    #[func]
    #[allow(clippy::too_many_arguments)] // FFI boundary function - arguments match GDScript call
    pub fn scene_tree_event_typed_metadata_groups(
        &self,
        node: Gd<Node>,
        message_type: SceneTreeMessageType,
        node_type: String,
        node_name: String,
        parent_id: i64,
        collision_mask: i64,
        groups: PackedStringArray,
    ) {
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
        let groups = groups
            .as_slice()
            .iter()
            .map(|s| s.to_string())
            .collect::<Vec<_>>();

        if let Some(channel) = self.notification_channel.as_ref() {
            let _ = channel.send(SceneTreeMessage {
                node_id: GodotNodeHandle::from(node.instance_id()),
                message_type,
                node_type,
                node_name,
                parent_id,
                collision_mask,
                groups: Some(groups),
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
                groups: None,
            });
        }
    }
}

/// True if `node` or any ancestor carries the `_bevy_exclude` meta. Exclusion is
/// subtree-wide -- a node under an excluded root is never mirrored -- and only the
/// mirror-in decision (`NodeAdded`) consults it; removals stay unconditional so an
/// already-mirrored node is never leaked.
fn is_excluded_from_mirror(node: &Gd<Node>) -> bool {
    let mut current = Some(node.clone());
    while let Some(n) = current {
        if n.has_meta("_bevy_exclude") {
            return true;
        }
        current = n.get_parent();
    }
    false
}
