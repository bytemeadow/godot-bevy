use bevy::ecs::resource::Resource as BevyResource;
use godot::{
    classes::{Resource, class_macros::sys},
    obj::{Gd, InstanceId},
};

use super::utils::{maybe_dec_ref, maybe_inc_ref, maybe_inc_ref_opt};

#[derive(Debug, BevyResource)]
pub struct GodotResourceHandle {
    resource_id: InstanceId,
}

impl GodotResourceHandle {
    pub fn get(&mut self) -> Gd<Resource> {
        self.try_get().unwrap()
    }

    pub fn try_get(&mut self) -> Option<Gd<Resource>> {
        Gd::try_from_instance_id(self.resource_id).ok()
    }

    pub fn new(mut reference: Gd<Resource>) -> Self {
        maybe_inc_ref(&mut reference);

        Self {
            resource_id: reference.instance_id(),
        }
    }
}

impl Clone for GodotResourceHandle {
    fn clone(&self) -> Self {
        maybe_inc_ref_opt::<Resource>(&mut Gd::try_from_instance_id(self.resource_id).ok());

        Self {
            resource_id: self.resource_id.clone(),
        }
    }
}

impl Drop for GodotResourceHandle {
    fn drop(&mut self) {
        let mut gd = self.get();
        let is_last = maybe_dec_ref(&mut gd); // may drop
        if is_last {
            unsafe {
                sys::interface_fn!(object_destroy)(gd.obj_sys());
            }
        }
    }
}
