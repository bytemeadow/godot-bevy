#[cfg(test)]
mod tests {
    use crate::plugins::scene_tree::{
        GodotChildOf, GodotChildren, ProtectedNodeEntity, SceneTreeConfig,
    };
    use bevy_ecs::entity::Entity;
    use bevy_ecs::prelude::World;
    use bevy_reflect::{Reflect, TypeRegistry};

    #[test]
    fn test_scene_tree_relationship_reflection() {
        let mut registry = TypeRegistry::default();
        registry.register::<GodotChildOf>();
        registry.register::<GodotChildren>();

        assert!(
            registry
                .get_type_info(std::any::TypeId::of::<GodotChildOf>())
                .is_some()
        );
        assert!(
            registry
                .get_type_info(std::any::TypeId::of::<GodotChildren>())
                .is_some()
        );

        let child_of = GodotChildOf(Entity::from_bits(1));
        let reflected = child_of.as_reflect();
        let type_info = reflected.get_represented_type_info().unwrap();
        assert!(type_info.type_path().contains("GodotChildOf"));
    }

    #[test]
    fn test_children_auto_despawn_enabled() {
        let mut world = World::new();
        world.insert_resource(SceneTreeConfig {
            auto_despawn_children: true,
        });

        let parent = world.spawn_empty().id();
        let child_a = world.spawn(GodotChildOf(parent)).id();
        let child_b = world.spawn(GodotChildOf(parent)).id();

        world.entity_mut(parent).despawn();
        world.flush();

        assert!(world.get_entity(child_a).is_err());
        assert!(world.get_entity(child_b).is_err());
    }

    #[test]
    fn test_children_auto_despawn_disabled() {
        let mut world = World::new();
        world.insert_resource(SceneTreeConfig {
            auto_despawn_children: false,
        });

        let parent = world.spawn_empty().id();
        let child = world.spawn(GodotChildOf(parent)).id();

        world.entity_mut(parent).despawn();
        world.flush();

        assert!(world.get_entity(child).is_ok());
    }

    #[test]
    fn test_protected_children_are_not_despawned() {
        let mut world = World::new();
        world.insert_resource(SceneTreeConfig {
            auto_despawn_children: true,
        });

        let parent = world.spawn_empty().id();
        let child_unprotected = world.spawn(GodotChildOf(parent)).id();
        let child_protected = world
            .spawn((GodotChildOf(parent), ProtectedNodeEntity))
            .id();

        world.entity_mut(parent).despawn();
        world.flush();

        assert!(world.get_entity(child_unprotected).is_err());
        assert!(world.get_entity(child_protected).is_ok());
    }
}
