use bevy::app::{App, Last, Plugin, PreUpdate};
use bevy::ecs::change_detection::DetectChanges;
use bevy::ecs::query::{Added, Changed, Or, With};
use bevy::ecs::system::Query;
use bevy::prelude::Res;
use godot::builtin::Transform2D as GodotTransform2D;
use godot::classes::{Node2D, Node3D};

use crate::interop::GodotNodeHandle;
use crate::interop::node_markers::{Node2DMarker, Node3DMarker};
use crate::plugins::core::GodotDefaultTransformSyncConfig;
use crate::plugins::transforms::components::{Transform2D, Transform3D};
use crate::prelude::main_thread_system;

#[main_thread_system]
pub fn default_post_update_godot_transforms_3d(
    config: Res<GodotDefaultTransformSyncConfig>,
    mut entities: Query<
        (&Transform3D, &mut GodotNodeHandle),
        (
            Or<(Added<Transform3D>, Changed<Transform3D>)>,
            With<Node3DMarker>,
        ),
    >,
) {
    if config.sync_mode == crate::plugins::core::TransformSyncMode::Disabled {
        return;
    }

    for (transform, mut reference) in entities.iter_mut() {
        if let Some(mut obj) = reference.try_get::<Node3D>() {
            if obj.get_transform() != *transform.as_godot() {
                obj.set_transform(*transform.as_godot());
            }
        }
    }
}

pub fn default_pre_update_godot_transforms_3d(
    config: Res<GodotDefaultTransformSyncConfig>,
    mut entities: Query<(&mut Transform3D, &mut GodotNodeHandle), With<Node3DMarker>>,
) {
    if config.sync_mode != crate::plugins::core::TransformSyncMode::TwoWay {
        return;
    }

    for (mut transform, mut reference) in entities.iter_mut() {
        if transform.is_changed() {
            continue;
        }

        if let Some(godot_node) = reference.try_get::<Node3D>() {
            let godot_transform = godot_node.get_transform();
            if *transform.as_godot() != godot_transform {
                *transform.as_godot_mut() = godot_transform;
            }
        }
    }
}

#[main_thread_system]
pub fn default_post_update_godot_transforms_2d(
    config: Res<GodotDefaultTransformSyncConfig>,
    mut entities: Query<
        (&Transform2D, &mut GodotNodeHandle),
        (
            Or<(Added<Transform2D>, Changed<Transform2D>)>,
            With<Node2DMarker>,
        ),
    >,
) {
    if config.sync_mode == crate::plugins::core::TransformSyncMode::Disabled {
        return;
    }

    for (transform, mut reference) in entities.iter_mut() {
        if let Some(mut obj) = reference.try_get::<Node2D>() {
            let mut obj_transform = GodotTransform2D::IDENTITY.translated(obj.get_position());
            obj_transform = obj_transform.rotated(obj.get_rotation());
            obj_transform = obj_transform.scaled(obj.get_scale());

            if obj_transform != *transform.as_godot() {
                obj.set_transform(*transform.as_godot());
            }
        }
    }
}

pub fn default_pre_update_godot_transforms_2d(
    config: Res<GodotDefaultTransformSyncConfig>,
    mut entities: Query<(&mut Transform2D, &mut GodotNodeHandle), With<Node2DMarker>>,
) {
    if config.sync_mode != crate::plugins::core::TransformSyncMode::TwoWay {
        return;
    }

    for (mut transform, mut reference) in entities.iter_mut() {
        if transform.is_changed() {
            continue;
        }

        if let Some(obj) = reference.try_get::<Node2D>() {
            let mut obj_transform = GodotTransform2D::IDENTITY.translated(obj.get_position());
            obj_transform = obj_transform.rotated(obj.get_rotation());
            obj_transform = obj_transform.scaled(obj.get_scale());

            if *transform.as_godot() != obj_transform {
                *transform.as_godot_mut() = obj_transform;
            }
        }
    }
}

/// Base plugin that only sets up transform sync configuration for custom systems.
/// Use this when you want to provide your own custom transform sync systems.
pub struct GodotCustomTransformSyncPlugin {
    pub sync_mode: crate::plugins::core::TransformSyncMode,
}

impl Default for GodotCustomTransformSyncPlugin {
    fn default() -> Self {
        Self {
            sync_mode: crate::plugins::core::TransformSyncMode::OneWay,
        }
    }
}

impl Plugin for GodotCustomTransformSyncPlugin {
    fn build(&self, app: &mut App) {
        // Only register the transform configuration resource for custom systems
        app.insert_resource(crate::plugins::core::GodotCustomTransformSyncConfig {
            sync_mode: self.sync_mode,
        });
    }
}

/// Plugin that provides default transform synchronization for all Node2D and Node3D entities.
/// This is equivalent to the old GodotTransformSyncPlugin behavior.
///
/// For custom transform sync queries, use `GodotCustomTransformSyncPlugin` instead and
/// define your own systems with the `transform_sync_systems!` macro.
pub struct GodotDefaultTransformSyncPlugin {
    pub sync_mode: crate::plugins::core::TransformSyncMode,
}

impl Default for GodotDefaultTransformSyncPlugin {
    fn default() -> Self {
        Self {
            sync_mode: crate::plugins::core::TransformSyncMode::OneWay,
        }
    }
}

impl Plugin for GodotDefaultTransformSyncPlugin {
    fn build(&self, app: &mut App) {
        // Register the default transform sync configuration resource
        app.insert_resource(GodotDefaultTransformSyncConfig {
            sync_mode: self.sync_mode,
        });

        // Add the default systems (sync all Node2D and Node3D entities)
        app.add_systems(
            Last,
            (
                default_post_update_godot_transforms_2d,
                default_post_update_godot_transforms_3d,
            ),
        )
        .add_systems(
            PreUpdate,
            (
                default_pre_update_godot_transforms_2d,
                default_pre_update_godot_transforms_3d,
            ),
        );
    }
}

/// Legacy alias for backward compatibility.
///
/// **Deprecated**: Use `GodotDefaultTransformSyncPlugin` for default behavior,
/// or `GodotCustomTransformSyncPlugin` + custom systems for advanced use cases.
#[deprecated(
    since = "0.7.0",
    note = "Use `GodotDefaultTransformSyncPlugin` for default behavior, or `GodotCustomTransformSyncPlugin` + custom systems for advanced use cases"
)]
pub type GodotTransformSyncPlugin = GodotDefaultTransformSyncPlugin;
