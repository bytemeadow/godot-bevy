#[cfg(test)]
mod tests {
    use bevy::reflect::{Reflect, TypeRegistry, ReflectRef};
    use crate::plugins::collisions::Collisions;
    use crate::plugins::scene_tree::Groups;
    use crate::plugins::transforms::{GodotTransformConfig, TransformSyncMode};

    #[test]
    fn test_collisions_reflection() {
        // Test type registration
        let mut registry = TypeRegistry::default();
        registry.register::<Collisions>();

        // Verify the type is registered
        assert!(registry.get_type_info(std::any::TypeId::of::<Collisions>()).is_some());

        // Test reflection capabilities
        let collisions = Collisions::default();
        let reflected = collisions.as_reflect();

        // Verify we can get the reflect info
        let type_info = reflected.get_represented_type_info().unwrap();
        assert!(type_info.type_path().contains("Collisions"));

        // Test struct reflection
        if let ReflectRef::Struct(struct_ref) = reflected.reflect_ref() {
            // Check fields exist
            assert_eq!(struct_ref.field_len(), 2);
            assert!(struct_ref.field("colliding_entities").is_some());
            assert!(struct_ref.field("recent_collisions").is_some());
        } else {
            panic!("Expected Struct reflection");
        }

        // Test cloning via reflection
        let _cloned = reflected.reflect_clone();
    }

    #[test]
    fn test_groups_reflection() {
        let mut registry = TypeRegistry::default();
        registry.register::<Groups>();

        assert!(registry.get_type_info(std::any::TypeId::of::<Groups>()).is_some());

        // Can't easily create Groups without Godot node, but we can test the type is reflectable
        // The actual reflection would work at runtime
    }

    #[test]
    fn test_transform_config_reflection() {
        let mut registry = TypeRegistry::default();
        registry.register::<GodotTransformConfig>();
        registry.register::<TransformSyncMode>();

        assert!(registry.get_type_info(std::any::TypeId::of::<GodotTransformConfig>()).is_some());
        assert!(registry.get_type_info(std::any::TypeId::of::<TransformSyncMode>()).is_some());

        let config = GodotTransformConfig::default();
        let reflected = config.as_reflect();

        // Check type info
        let type_info = reflected.get_represented_type_info().unwrap();
        assert!(type_info.type_path().contains("GodotTransformConfig"));

        // Check struct fields
        if let ReflectRef::Struct(struct_ref) = reflected.reflect_ref() {
            assert!(struct_ref.field("sync_mode").is_some());
        } else {
            panic!("Expected Struct reflection");
        }

        // Test enum reflection
        let mode = TransformSyncMode::TwoWay;
        let mode_reflected = mode.as_reflect();
        let mode_info = mode_reflected.get_represented_type_info().unwrap();
        assert!(mode_info.type_path().contains("TransformSyncMode"));
    }

    #[test]
    fn test_scene_tree_config_reflection() {
        use crate::plugins::scene_tree::SceneTreeConfig;

        let mut registry = TypeRegistry::default();
        registry.register::<SceneTreeConfig>();

        assert!(registry.get_type_info(std::any::TypeId::of::<SceneTreeConfig>()).is_some());

        let config = SceneTreeConfig { add_child_relationship: false };
        let reflected = config.as_reflect();

        // Check type info
        let type_info = reflected.get_represented_type_info().unwrap();
        assert!(type_info.type_path().contains("SceneTreeConfig"));

        // Check struct fields
        if let ReflectRef::Struct(struct_ref) = reflected.reflect_ref() {
            assert!(struct_ref.field("add_child_relationship").is_some());
        } else {
            panic!("Expected Struct reflection");
        }
    }
}