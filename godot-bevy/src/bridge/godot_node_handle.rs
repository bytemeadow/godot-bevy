use bevy::ecs::component::Component;
use godot::{
    classes::Node,
    obj::{Gd, Inherits, InstanceId},
};

#[derive(Debug, Component, Clone)]
pub struct GodotNodeHandle {
    instance_id: InstanceId,
}

impl GodotNodeHandle {
    pub fn get<T: Inherits<Node>>(&mut self) -> Gd<T> {
        self.try_get().unwrap_or_else(|| {
            panic!(
                "failed to get godot node handle as {}",
                std::any::type_name::<T>()
            )
        })
    }

    /// # SAFETY
    /// The caller must uphold the contract of the constructors to ensure exclusive access
    pub fn try_get<T: Inherits<Node>>(&mut self) -> Option<Gd<T>> {
        Gd::try_from_instance_id(self.instance_id).ok()
    }

    /// # SAFETY
    /// When using GodotNodeHandle as a Bevy Resource or Component, do not create duplicate references
    /// to the same instance because Godot is not completely thread-safe.
    ///
    /// TODO
    /// Could these type bounds be more flexible to accomodate other types that are not ref-counted
    /// but don't inherit Node
    pub fn new<T: Inherits<Node>>(reference: Gd<T>) -> Self {
        Self {
            instance_id: reference.instance_id(),
        }
    }

    pub fn instance_id(&self) -> InstanceId {
        self.instance_id
    }

    pub fn from_instance_id(instance_id: InstanceId) -> Self {
        Self { instance_id }
    }
}
