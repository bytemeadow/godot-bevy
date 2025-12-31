use bevy_ecs::prelude::Resource;
use bevy_ecs::system::{NonSendMut, SystemParam};
use godot::{
    classes::Node,
    obj::{Gd, Inherits, InstanceId, Singleton},
};

use crate::interop::GodotNodeHandle;

/// Non-send marker resource that pins systems to the main thread.
#[derive(Resource, Default, Debug)]
pub struct GodotMainThread;

/// Capability to access Godot APIs on the main thread.
#[derive(SystemParam)]
pub struct GodotAccess<'w> {
    _main_thread: NonSendMut<'w, GodotMainThread>,
}

impl<'w> std::fmt::Debug for GodotAccess<'w> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GodotAccess").finish_non_exhaustive()
    }
}

impl<'w> GodotAccess<'w> {
    pub fn try_get<T: Inherits<Node>>(&mut self, handle: GodotNodeHandle) -> Option<Gd<T>> {
        Gd::try_from_instance_id(handle.instance_id()).ok()
    }

    pub fn get<T: Inherits<Node>>(&mut self, handle: GodotNodeHandle) -> Gd<T> {
        self.try_get(handle).unwrap_or_else(|| {
            panic!(
                "failed to get godot node handle as {}",
                std::any::type_name::<T>()
            )
        })
    }

    pub fn try_get_instance_id<T: Inherits<Node>>(
        &mut self,
        instance_id: InstanceId,
    ) -> Option<Gd<T>> {
        Gd::try_from_instance_id(instance_id).ok()
    }

    pub fn get_instance_id<T: Inherits<Node>>(&mut self, instance_id: InstanceId) -> Gd<T> {
        self.try_get_instance_id(instance_id).unwrap_or_else(|| {
            panic!(
                "failed to get godot node handle as {}",
                std::any::type_name::<T>()
            )
        })
    }

    /// Access a Godot singleton. Requires main-thread access.
    pub fn singleton<T: Singleton>(&mut self) -> Gd<T> {
        T::singleton()
    }

    /// Create a scoped node accessor tied to this main-thread guard.
    pub fn node<'a>(&'a mut self, handle: GodotNodeHandle) -> GodotNode<'a, 'w> {
        GodotNode { godot: self, handle }
    }
}

/// Scoped accessor that ties a Godot node handle to main-thread access.
pub struct GodotNode<'a, 'w> {
    godot: &'a mut GodotAccess<'w>,
    handle: GodotNodeHandle,
}

impl<'a, 'w> GodotNode<'a, 'w> {
    pub fn handle(&self) -> GodotNodeHandle {
        self.handle
    }

    pub fn instance_id(&self) -> InstanceId {
        self.handle.instance_id()
    }

    pub fn try_get<T: Inherits<Node>>(&mut self) -> Option<Gd<T>> {
        self.godot.try_get(self.handle)
    }

    pub fn get<T: Inherits<Node>>(&mut self) -> Gd<T> {
        self.godot.get(self.handle)
    }
}
