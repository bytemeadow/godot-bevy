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

pub struct GodotTransformSyncPlugin {
    /// The mode for syncing transforms between Godot and Bevy.
    /// Note: This setting is only relevant when `auto_sync` is true.
    /// When `auto_sync` is false, this value is ignored since no automatic sync systems run.
    pub sync_mode: crate::plugins::core::TransformSyncMode,
    /// When true (default), enables automatic transform syncing systems.
    /// When false, still registers Transform and TransformSyncMetadata components
    /// but allows defining custom sync systems using the add_transform_sync_systems_*! macros.
    pub auto_sync: bool,
}

impl Default for GodotTransformSyncPlugin {
    fn default() -> Self {
        Self {
            sync_mode: crate::plugins::core::TransformSyncMode::default(),
            auto_sync: true,
        }
    }
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

        // Only add automatic sync systems if auto_sync is enabled
        if self.auto_sync {
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
}

fn transform_sync_enabled(config: Res<GodotTransformConfig>) -> bool {
    // aka one way or two way
    config.sync_mode != TransformSyncMode::Disabled
}

fn transform_sync_twoway_enabled(config: Res<GodotTransformConfig>) -> bool {
    config.sync_mode == TransformSyncMode::TwoWay
}
