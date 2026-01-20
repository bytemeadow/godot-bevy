use bevy_ecs::prelude::Component;
use godot::{
    classes::Node,
    obj::{Gd, Inherits, InstanceId},
};

/// Opaque identifier for a Godot node (safe to pass across threads).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Component)]
pub struct GodotNodeHandle {
    instance_id: InstanceId,
}

impl GodotNodeHandle {
    pub fn instance_id(self) -> InstanceId {
        self.instance_id
    }

    /// Create a handle from a live Godot node.
    pub fn new<T: Inherits<Node>>(reference: Gd<T>) -> Self {
        Self {
            instance_id: reference.instance_id(),
        }
    }

    pub fn from_instance_id(instance_id: InstanceId) -> Self {
        Self { instance_id }
    }
}

impl From<InstanceId> for GodotNodeHandle {
    fn from(instance_id: InstanceId) -> Self {
        Self { instance_id }
    }
}

impl From<GodotNodeHandle> for InstanceId {
    fn from(handle: GodotNodeHandle) -> Self {
        handle.instance_id
    }
}

impl<T: Inherits<Node>> From<Gd<T>> for GodotNodeHandle {
    fn from(node: Gd<T>) -> Self {
        Self::new(node)
    }
}
