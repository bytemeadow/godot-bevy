use bevy::{
    app::{App, Last, Plugin, PreUpdate},
    ecs::{schedule::IntoScheduleConfigs, system::Res},
};

use crate::prelude::{GodotTransformConfig, TransformSyncMode};

use super::sync_systems::{
    post_update_godot_transforms_2d, post_update_godot_transforms_3d,
    pre_update_godot_transforms_2d, pre_update_godot_transforms_3d,
};

#[derive(Default)]
pub struct GodotTransformSyncPlugin {
    pub sync_mode: crate::plugins::core::TransformSyncMode,
}

impl Plugin for GodotTransformSyncPlugin {
    fn build(&self, app: &mut App) {
        // Register the transform configuration resource with the plugin's config
        app.insert_resource(GodotTransformConfig {
            sync_mode: self.sync_mode,
        });

        // Add systems that sync bevy -> godot transforms when one or two-way syncing enabled
        app.add_systems(
            Last,
            (
                post_update_godot_transforms_3d,
                post_update_godot_transforms_2d,
            )
                .run_if(transform_sync_enabled),
        );

        // Add systems that sync godot -> bevy transforms when two-way syncing enabled
        app.add_systems(
            PreUpdate,
            (
                pre_update_godot_transforms_3d,
                pre_update_godot_transforms_2d,
            )
                .run_if(transform_sync_twoway_enabled),
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
