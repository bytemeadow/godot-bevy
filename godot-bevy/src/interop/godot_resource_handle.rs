use godot::{
    classes::{Object, Resource},
    meta::ToGodot,
    obj::{Gd, InstanceId, NewAlloc},
};

/// A `Send + Sync` handle that keeps a Godot resource alive.
///
/// Instance IDs avoid storing a non-`Send` [`Gd`]. Each handle owns a separate
/// [`Object`] whose metadata retains one resource reference until the handle drops.
/// This uses native reference counting even if a resource script shadows `unreference`.
///
/// Worker-thread use requires `experimental-threads`. Resource access still follows
/// Godot's threading rules, and handles must be dropped before gdext shuts down.
#[derive(Debug, Eq)]
pub struct GodotResourceHandle {
    resource_id: InstanceId,
    owner_id: InstanceId,
}

impl GodotResourceHandle {
    pub fn get(&mut self) -> Gd<Resource> {
        self.try_get()
            .expect("Godot resource was freed unexpectedly")
    }

    pub fn try_get(&mut self) -> Option<Gd<Resource>> {
        Gd::try_from_instance_id(self.resource_id).ok()
    }

    pub fn new(reference: Gd<Resource>) -> Self {
        let resource_id = reference.instance_id();
        let mut owner = Object::new_alloc();
        owner.set_meta("_resource", &reference.to_variant());

        Self {
            resource_id,
            owner_id: owner.instance_id(),
        }
    }
}

impl Clone for GodotResourceHandle {
    fn clone(&self) -> Self {
        match Gd::try_from_instance_id(self.resource_id) {
            Ok(resource) => Self::new(resource),
            Err(_) => Self {
                resource_id: self.resource_id,
                owner_id: self.owner_id,
            },
        }
    }
}

impl Drop for GodotResourceHandle {
    fn drop(&mut self) {
        // Shutdown can destroy the owner before the handle.
        if let Ok(owner) = Gd::<Object>::try_from_instance_id(self.owner_id) {
            owner.free();
        }
    }
}

impl PartialEq for GodotResourceHandle {
    fn eq(&self, other: &Self) -> bool {
        self.resource_id == other.resource_id
    }
}
