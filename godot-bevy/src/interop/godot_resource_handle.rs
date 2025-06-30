use godot::{
    classes::{Resource, class_macros::sys},
    obj::{Gd, InstanceId},
};

use super::utils::{maybe_dec_ref, maybe_inc_ref, maybe_inc_ref_opt};

/// A thread-safe handle to a Godot resource that manages reference counting.
/// This ensures the resource stays alive as long as the handle exists.
///
/// Uses Godot's reference counting system to prevent premature garbage collection
/// while still allowing the resource to be freed when no longer needed.
#[derive(Debug, PartialEq, Eq)]
pub struct GodotResourceHandle {
    resource_id: InstanceId,
}

impl GodotResourceHandle {
    /// Get a reference to the Godot resource
    pub fn get(&mut self) -> Gd<Resource> {
        self.try_get()
            .expect("Godot resource was freed unexpectedly")
    }

    /// Try to get a reference to the Godot resource
    pub fn try_get(&mut self) -> Option<Gd<Resource>> {
        Gd::try_from_instance_id(self.resource_id).ok()
    }

    /// Create a new handle from a Godot resource
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
            resource_id: self.resource_id,
        }
    }
}

impl Drop for GodotResourceHandle {
    fn drop(&mut self) {
        // Use try_get to avoid panicking if the resource was already freed
        if let Some(mut gd) = self.try_get() {
            let is_last = maybe_dec_ref(&mut gd); // may drop
            if is_last {
                unsafe {
                    sys::interface_fn!(object_destroy)(gd.obj_sys());
                }
            }
        }
        // If try_get returns None, the resource was already freed, which is fine
    }
}
