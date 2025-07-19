use bevy::{
    app::{App, Last, Plugin, PreUpdate},
    ecs::{schedule::IntoScheduleConfigs, system::Res},
    prelude::Transform,
};
use godot::classes::{Node2D, Node3D};

use crate::plugins::core::AppSceneTreeExt;
use crate::plugins::transforms::IntoBevyTransform;
use crate::prelude::{GodotTransformConfig, TransformSyncMode};

use super::change_filter::TransformSyncMetadata;
use super::sync_systems::{post_update_godot_transforms, pre_update_godot_transforms};

#[derive(Default)]
pub struct GodotTransformSyncPlugin {
    pub sync_mode: crate::plugins::core::TransformSyncMode,
}

impl Plugin for GodotTransformSyncPlugin {
    fn build(&self, app: &mut App) {
        // Register Transform component with custom initialization that reads from Godot
        app.register_scene_tree_component_with_init::<Transform, _>(|entity, node| {
            let mut node_handle = node.clone(); // Clone to get mutable access
            if let Some(node3d) = node_handle.try_get::<Node3D>() {
                entity.insert(node3d.get_transform().to_bevy_transform());
            } else if let Some(node2d) = node_handle.try_get::<Node2D>() {
                entity.insert(node2d.get_transform().to_bevy_transform());
            } else {
                // Fallback to default for non-spatial nodes
                entity.insert(Transform::default());
            }
        })
        // Register metadata component with default - this avoids the 1-frame delay
        .register_scene_tree_component::<TransformSyncMetadata>();

        // Register the transform configuration resource with the plugin's config
        app.insert_resource(GodotTransformConfig {
            sync_mode: self.sync_mode,
        });

        // Add systems that sync godot -> bevy transforms when two-way syncing enabled
        app.add_systems(
            PreUpdate,
            pre_update_godot_transforms.run_if(transform_sync_twoway_enabled),
        );

        // Add systems that sync bevy -> godot transforms when one or two-way syncing enabled
        app.add_systems(
            Last,
            post_update_godot_transforms.run_if(transform_sync_enabled),
        );
    }
}

fn transform_sync_enabled(config: Res<GodotTransformConfig>) -> bool {
    // aka one way or two way
    config.sync_mode != TransformSyncMode::Disabled
}

fn transform_sync_twoway_enabled(config: Res<GodotTransformConfig>) -> bool {
    config.sync_mode == TransformSyncMode::TwoWay
}
