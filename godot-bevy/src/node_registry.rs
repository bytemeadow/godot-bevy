use bevy::prelude::*;
use bevy::ecs::system::SystemParam;
use godot::prelude::*;
use godot::obj::Inherits;
use godot::classes::Node;
use std::collections::HashMap;

use crate::bridge::GodotNodeHandle;

/// Resource that maintains a registry of entities to their corresponding Godot nodes
#[derive(Resource, Default)]
pub struct NodeRegistry {
    /// Maps entity to its GodotNodeHandle for fast access
    entity_to_handle: HashMap<Entity, GodotNodeHandle>,
    /// Optional caching for frequently accessed nodes
    node_cache: HashMap<Entity, Option<InstanceId>>,
}

/// Trait that enables typed access to Godot nodes through marker components
pub trait NodeAccess: Component + 'static {
    /// The Godot node type this marker corresponds to
    type GodotType: GodotClass + Inherits<Node>;
    
    /// Access the Godot node through the registry for a specific entity
    fn access_node_for_entity(registry: &NodeRegistry, entity: Entity) -> Option<Gd<Self::GodotType>> {
        let node_handle = registry.entity_to_handle.get(&entity)?;
        node_handle.clone().try_get::<Self::GodotType>()
    }
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
    
    /// Access a Godot node for a specific entity through its marker component type
    pub fn access<T: NodeAccess>(&self, entity: Entity) -> Option<TypedNodeRef<T::GodotType>> {
        T::access_node_for_entity(self, entity).map(TypedNodeRef::new)
    }
    
    /// Remove an entity from the registry (called when entities are despawned)
    pub fn unregister_entity(&mut self, entity: Entity) {
        self.entity_to_handle.remove(&entity);
        self.node_cache.remove(&entity);
    }
    
    /// Clear the node cache (useful when nodes are destroyed)
    pub fn clear_cache(&mut self) {
        self.node_cache.clear();
    }
    
}

/// System parameter that provides ergonomic access to the node registry
#[derive(SystemParam)]
pub struct NodeRegistryAccess<'w, 's> {
    registry: Res<'w, NodeRegistry>,
    _phantom: std::marker::PhantomData<&'s ()>,
}

impl<'w, 's> NodeRegistryAccess<'w, 's> {
    /// Access a Godot node for a specific entity through its marker component type
    pub fn access<T: NodeAccess>(&self, entity: Entity) -> Option<TypedNodeRef<T::GodotType>> {
        self.registry.access::<T>(entity)
    }
}