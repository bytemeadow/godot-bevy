use bevy_app::{App, FixedFirst, FixedLast, Plugin, PreUpdate};
use bevy_ecs::{schedule::IntoScheduleConfigs, system::Res};
use bevy_transform::components::Transform;
use godot::classes::{Node2D, Node3D};

use crate::plugins::core::AppSceneTreeExt;
use crate::plugins::fixed_schedule::not_first_fixed_step;
use crate::plugins::transforms::IntoBevyTransform;
use crate::plugins::transforms::{GodotTransformConfig, TransformSyncMode};

use super::change_filter::TransformSyncMetadata;
use super::sync_systems::{post_update_godot_transforms, pre_update_godot_transforms};

pub struct GodotTransformSyncPlugin {
    /// The mode for syncing transforms between Godot and Bevy.
    /// Note: This setting is only relevant when `auto_sync` is true.
    /// When `auto_sync` is false, this value is ignored since no automatic sync systems run.
    pub sync_mode: TransformSyncMode,
    /// When true (default), enables automatic transform syncing systems.
    /// When false, still registers Transform and TransformSyncMetadata components
    /// but allows defining custom sync systems using the add_transform_sync_systems_*! macros.
    pub auto_sync: bool,
}

impl Default for GodotTransformSyncPlugin {
    fn default() -> Self {
        Self {
            sync_mode: TransformSyncMode::default(),
            auto_sync: true,
        }
    }
}

impl Plugin for GodotTransformSyncPlugin {
    fn build(&self, app: &mut App) {
        // Register Transform component with custom initialization that reads from Godot
        app.register_scene_tree_component_with_init::<Transform, _>(|entity, node| {
            if let Some(node3d) = node.try_get::<Node3D>() {
                entity.insert(node3d.get_transform().to_bevy_transform());
            } else if let Some(node2d) = node.try_get::<Node2D>() {
                entity.insert(node2d.get_transform().to_bevy_transform());
            }
        })
        // Seed the shadow from the node at registration so shadow == Transform ==
        // Godot at spawn (written_once == false). This closes the clobber window for
        // a user authoring in Startup/First before the first read, and avoids a
        // spurious frame-1 Changed.
        .register_scene_tree_component_with_init::<TransformSyncMetadata, _>(|entity, node| {
            let shadow = if let Some(node3d) = node.try_get::<Node3D>() {
                node3d.get_transform().to_bevy_transform()
            } else if let Some(node2d) = node.try_get::<Node2D>() {
                node2d.get_transform().to_bevy_transform()
            } else {
                Transform::default()
            };
            entity.insert(TransformSyncMetadata {
                shadow,
                written_once: false,
            });
        });

        // Register the transform configuration resource with the plugin's config
        app.insert_resource(GodotTransformConfig {
            sync_mode: self.sync_mode,
        });

        // Only add automatic sync systems if auto_sync is enabled
        if self.auto_sync {
            // Godot->Bevy read runs once per render frame in PreUpdate (covering
            // step 1, the Update suffix, and idle 0-step frames) and once per
            // physics step in FixedFirst for steps 2..N -- matching the FixedLast
            // write cadence so a Godot physics-clock author moving an axis between
            // steps isn't clobbered by a stale whole-transform write. The
            // value-shadow guard makes the duplicate read idempotent; 1-step and
            // idle frames still do exactly one read.
            app.add_systems(
                PreUpdate,
                pre_update_godot_transforms::<()>.run_if(transform_sync_twoway_enabled),
            );
            app.add_systems(
                FixedFirst,
                pre_update_godot_transforms::<()>
                    .run_if(transform_sync_twoway_enabled)
                    .run_if(not_first_fixed_step),
            );

            // Bevy -> Godot write at physics rate (once per fixed tick). This is
            // the cadence Godot's physics interpolation requires; rendering
            // between ticks is smoothed by the engine when the user enables
            // physics/common/physics_interpolation.
            app.add_systems(
                FixedLast,
                post_update_godot_transforms::<()>.run_if(transform_sync_enabled),
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
