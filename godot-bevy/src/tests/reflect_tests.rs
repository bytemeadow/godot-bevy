#[cfg(test)]
mod tests {
    use crate::plugins::scene_tree::Groups;
    use crate::plugins::transforms::{
        GodotTransformConfig, TransformSyncMetadata, TransformSyncMode,
    };
    use bevy_reflect::{Reflect, ReflectRef, TypeRegistry};

    #[test]
    fn test_groups_reflection() {
        let mut registry = TypeRegistry::default();
        registry.register::<Groups>();

        assert!(
            registry
                .get_type_info(std::any::TypeId::of::<Groups>())
                .is_some()
        );

        // Can't easily create Groups without Godot node, but we can test the type is reflectable
        // The actual reflection would work at runtime
    }

    #[test]
    fn test_transform_config_reflection() {
        let mut registry = TypeRegistry::default();
        registry.register::<GodotTransformConfig>();
        registry.register::<TransformSyncMode>();

        assert!(
            registry
                .get_type_info(std::any::TypeId::of::<GodotTransformConfig>())
                .is_some()
        );
        assert!(
            registry
                .get_type_info(std::any::TypeId::of::<TransformSyncMode>())
                .is_some()
        );

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

        assert!(
            registry
                .get_type_info(std::any::TypeId::of::<SceneTreeConfig>())
                .is_some()
        );

        let config = SceneTreeConfig {
            auto_despawn_children: false,
        };
        let reflected = config.as_reflect();

        // Check type info
        let type_info = reflected.get_represented_type_info().unwrap();
        assert!(type_info.type_path().contains("SceneTreeConfig"));

        // Check struct fields
        if let ReflectRef::Struct(struct_ref) = reflected.reflect_ref() {
            assert!(struct_ref.field("auto_despawn_children").is_some());
        } else {
            panic!("Expected Struct reflection");
        }
    }

    #[test]
    fn test_transform_sync_metadata_reflection() {
        let mut registry = TypeRegistry::default();
        registry.register::<TransformSyncMetadata>();

        assert!(
            registry
                .get_type_info(std::any::TypeId::of::<TransformSyncMetadata>())
                .is_some()
        );

        let metadata = TransformSyncMetadata::default();
        let reflected = metadata.as_reflect();

        // Check type info
        let type_info = reflected.get_represented_type_info().unwrap();
        assert!(type_info.type_path().contains("TransformSyncMetadata"));

        // Since last_sync_tick is marked with #[reflect(ignore)], it won't be accessible
        // through reflection, but the struct itself is still reflectable
        if let ReflectRef::Struct(struct_ref) = reflected.reflect_ref() {
            // The field is ignored, so field_len should be 0
            assert_eq!(struct_ref.field_len(), 0);
            // And the field should not be accessible
            assert!(struct_ref.field("last_sync_tick").is_none());
        } else {
            panic!("Expected Struct reflection");
        }
    }
}
