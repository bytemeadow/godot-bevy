use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use godot::classes::Node;
use godot::obj::Inherits;
use godot::prelude::*;
use std::collections::HashMap;

use crate::bridge::GodotNodeHandle;

/// Resource that maintains a registry of entities to their corresponding Godot nodes
#[derive(Resource, Default)]
pub struct NodeRegistry {
    /// Maps entity to its GodotNodeHandle for fast access
    entity_to_handle: HashMap<Entity, GodotNodeHandle>,
}

/// Typed reference to a Godot node that provides ergonomic access
pub struct TypedNodeRef<T: GodotClass + Inherits<Node>> {
    node: Gd<T>,
}

impl<T: GodotClass + Inherits<Node>> TypedNodeRef<T> {
    pub fn new(node: Gd<T>) -> Self {
        Self { node }
    }

    /// Get a reference to the underlying Godot node
    pub fn get(&self) -> &Gd<T> {
        &self.node
    }

    /// Get a mutable reference to the underlying Godot node
    pub fn get_mut(&mut self) -> &mut Gd<T> {
        &mut self.node
    }

    /// Get a bound reference for calling Godot methods
    /// Note: Use try_get() or cast methods for access to Godot methods
    pub fn as_gd(&self) -> &Gd<T> {
        &self.node
    }

    /// Get a mutable bound reference for calling Godot methods
    /// Note: Use try_get() or cast methods for access to Godot methods
    pub fn as_gd_mut(&mut self) -> &mut Gd<T> {
        &mut self.node
    }
}

impl<T: GodotClass + Inherits<Node>> std::ops::Deref for TypedNodeRef<T> {
    type Target = Gd<T>;

    fn deref(&self) -> &Self::Target {
        &self.node
    }
}

impl<T: GodotClass + Inherits<Node>> std::ops::DerefMut for TypedNodeRef<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.node
    }
}

impl NodeRegistry {
    /// Register an entity with its corresponding GodotNodeHandle
    pub fn register_entity(&mut self, entity: Entity, handle: GodotNodeHandle) {
        self.entity_to_handle.insert(entity, handle);
    }

    /// Access a Godot node for a specific entity, panicking if not found or wrong type
    pub fn access<T: GodotClass + Inherits<Node>>(&self, entity: Entity) -> TypedNodeRef<T> {
        let node_handle = self
            .entity_to_handle
            .get(&entity)
            .unwrap_or_else(|| panic!("Entity {:?} not found in NodeRegistry", entity));
        let node = node_handle.clone().get::<T>();
        TypedNodeRef::new(node)
    }

    /// Try to access a Godot node for a specific entity as the given type
    pub fn try_access<T: GodotClass + Inherits<Node>>(
        &self,
        entity: Entity,
    ) -> Option<TypedNodeRef<T>> {
        let node_handle = self.entity_to_handle.get(&entity)?;
        let node = node_handle.clone().try_get::<T>()?;
        Some(TypedNodeRef::new(node))
    }

    /// Remove an entity from the registry (called when entities are despawned)
    pub fn unregister_entity(&mut self, entity: Entity) {
        self.entity_to_handle.remove(&entity);
    }
}

/// System parameter that provides ergonomic access to the node registry
#[derive(SystemParam)]
pub struct NodeRegistryAccess<'w, 's> {
    registry: Res<'w, NodeRegistry>,
    _phantom: std::marker::PhantomData<&'s ()>,
}

impl<'w, 's> NodeRegistryAccess<'w, 's> {
    /// Access a Godot node for a specific entity, panicking if not found or wrong type
    pub fn access<T: GodotClass + Inherits<Node>>(&self, entity: Entity) -> TypedNodeRef<T> {
        self.registry.access::<T>(entity)
    }

    /// Try to access a Godot node for a specific entity as the given type
    pub fn try_access<T: GodotClass + Inherits<Node>>(
        &self,
        entity: Entity,
    ) -> Option<TypedNodeRef<T>> {
        self.registry.try_access::<T>(entity)
    }
}
