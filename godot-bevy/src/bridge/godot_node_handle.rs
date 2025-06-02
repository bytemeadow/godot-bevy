use bevy::{ecs::component::Component, reflect::Reflect, prelude::ReflectDefault};
use godot::{
    classes::Node,
    obj::{Gd, Inherits, InstanceId},
};

#[derive(Debug, Component, Reflect, Clone, PartialEq, Eq)]
#[reflect(Default)]
pub struct GodotNodeHandle {
    #[reflect(ignore)]
    instance_id: InstanceId,
    /// Mirror of instance_id as i64 for reflection
    instance_id_value: i64,
}

impl Default for GodotNodeHandle {
    fn default() -> Self {
        // Create a dummy InstanceId from 1 (since 0 is invalid)
        let instance_id = InstanceId::from_i64(1);
        Self {
            instance_id,
            instance_id_value: 1,
        }
    }
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
        let instance_id = reference.instance_id();
        Self {
            instance_id,
            instance_id_value: instance_id.to_i64(),
        }
    }

    pub fn instance_id(&self) -> InstanceId {
        self.instance_id
    }

    pub fn from_instance_id(instance_id: InstanceId) -> Self {
        Self { 
            instance_id,
            instance_id_value: instance_id.to_i64(),
        }
    }
}
