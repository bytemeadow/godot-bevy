use bevy::app::{App, Last, Plugin, PreUpdate};
use bevy::ecs::query::With;

use crate::interop::node_markers::{Node2DMarker, Node3DMarker};
use crate::transform_sync_systems;

// Generate the default transform sync systems using the macro
// This ensures consistency with the user-facing macro system
transform_sync_systems! {
    DefaultAll: 2d = With<Node2DMarker>,
    DefaultAll: 3d = With<Node3DMarker>
}

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

        // Add the generated systems (same behavior as before)
        app.add_systems(Last, (
                post_update_godot_transforms_2d_defaultall,
                post_update_godot_transforms_3d_defaultall,
            ))
            .add_systems(PreUpdate, (
                pre_update_godot_transforms_2d_defaultall,
                pre_update_godot_transforms_3d_defaultall,
            ));
    }
}
