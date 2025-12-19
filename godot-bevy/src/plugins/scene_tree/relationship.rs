//! Custom relationship components for representing Godot's scene tree hierarchy in Bevy ECS.
//!
//! This module defines a custom parent-child relationship that mirrors Godot's node hierarchy
//! without conflicting with other plugins that may use Bevy's primary `ChildOf`/`Children`
//! relationship for different purposes (e.g., physics constraints, AI behavior trees).

use bevy_ecs::component::Component;
use bevy_ecs::entity::Entity;
use bevy_ecs::lifecycle::HookContext;
use bevy_ecs::prelude::ReflectComponent;
use bevy_ecs::relationship::RelationshipTarget;
use bevy_ecs::world::DeferredWorld;
use bevy_reflect::Reflect;

/// Marks an entity as a child of a Godot node parent in the scene tree.
///
/// This relationship mirrors Godot's node hierarchy in Bevy ECS, allowing you to query
/// and traverse the scene tree structure. Unlike Bevy's primary `ChildOf` component, this
/// relationship is specifically for Godot scene tree hierarchy and won't conflict with
/// other plugins that use parent-child relationships for different purposes.
///
/// # Example
///
/// ```ignore
/// fn find_parent_entity(
///     query: Query<&GodotChildOf>,
///     entity: Entity,
/// ) -> Option<Entity> {
///     query.get(entity).ok().map(|child_of| child_of.0)
/// }
/// ```
///
/// # Automatic Cleanup
///
/// By default, when a parent entity is despawned, all children with `GodotChildOf` pointing
/// to it will also be despawned. This can be configured via
/// `GodotSceneTreePlugin::auto_despawn_children` or `SceneTreeConfig::auto_despawn_children`.
#[derive(Component, Reflect, Clone, Copy, Debug, PartialEq, Eq)]
#[reflect(Component)]
#[relationship(relationship_target = GodotChildren)]
pub struct GodotChildOf(pub Entity);

impl GodotChildOf {
    /// Get the parent entity
    #[inline]
    pub fn get(&self) -> Entity {
        self.0
    }
}

/// Tracks which entities are children of a Godot node in the scene tree.
///
/// This is the reverse/target side of the `GodotChildOf` relationship. It's automatically
/// maintained by Bevy's relationship system and accelerates traversal down the hierarchy.
///
/// # Example
///
/// ```ignore
/// fn iterate_children(
///     query: Query<&GodotChildren>,
///     parent_entity: Entity,
/// ) {
///     if let Ok(children) = query.get(parent_entity) {
///         for &child_entity in children.iter() {
///             println!("Child: {:?}", child_entity);
///         }
///     }
/// }
/// ```
#[derive(Component, Reflect, Default, Clone, Debug, PartialEq, Eq)]
#[reflect(Component)]
#[relationship_target(relationship = GodotChildOf)]
#[component(on_despawn = godot_children_on_despawn)]
pub struct GodotChildren(Vec<Entity>);

impl GodotChildren {
    /// Get an iterator over child entities
    #[inline]
    pub fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.0.iter()
    }

    /// Get the number of children
    #[inline]
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if there are no children
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Get a specific child by index
    #[inline]
    pub fn get(&self, index: usize) -> Option<Entity> {
        self.0.get(index).copied()
    }

    /// Check if a specific entity is a child
    #[inline]
    pub fn contains(&self, entity: Entity) -> bool {
        self.0.contains(&entity)
    }
}

fn godot_children_on_despawn(world: DeferredWorld, context: HookContext) {
    let auto_despawn = world
        .get_resource::<super::SceneTreeConfig>()
        .map(|config| config.auto_despawn_children)
        .unwrap_or(true);

    if auto_despawn {
        <GodotChildren as RelationshipTarget>::on_despawn(world, context);
    }
}
