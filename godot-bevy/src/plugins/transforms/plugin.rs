use bevy::app::{App, Last, Plugin, PreUpdate};

use super::sync_systems::{
    post_update_godot_transforms_2d, post_update_godot_transforms_3d,
    pre_update_godot_transforms_2d, pre_update_godot_transforms_3d,
};

pub struct GodotTransformSyncPlugin {
    pub sync_mode: crate::plugins::core::TransformSyncMode,
}

impl Default for GodotTransformSyncPlugin {
    fn default() -> Self {
        Self {
            sync_mode: crate::plugins::core::TransformSyncMode::OneWay,
        }
    }
}

impl Plugin for GodotTransformSyncPlugin {
    fn build(&self, app: &mut App) {
        // Register the transform configuration resource with the plugin's config
        app.insert_resource(crate::plugins::core::GodotTransformConfig {
            sync_mode: self.sync_mode,
        });

        // Always add writing systems
        app.add_systems(Last, post_update_godot_transforms_3d)
            .add_systems(Last, post_update_godot_transforms_2d);

        // Always add reading systems, but they'll check the config at runtime
        app.add_systems(PreUpdate, pre_update_godot_transforms_3d)
            .add_systems(PreUpdate, pre_update_godot_transforms_2d);
    }
}
