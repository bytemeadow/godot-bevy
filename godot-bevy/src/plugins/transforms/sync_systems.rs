use bevy::ecs::change_detection::DetectChanges;
use bevy::ecs::query::{Added, Changed, Or, With};
use bevy::ecs::system::Query;
use bevy::prelude::Res;
use godot::builtin::Transform2D as GodotTransform2D;
use godot::classes::{Node2D, Node3D};

use crate::interop::GodotNodeHandle;
use crate::interop::node_markers::{Node2DMarker, Node3DMarker};
use crate::prelude::main_thread_system;

use super::components::{Transform2D, Transform3D};

#[main_thread_system]
pub fn post_update_godot_transforms_3d(
    config: Res<crate::plugins::core::GodotTransformConfig>,
    mut entities: Query<
        (&Transform3D, &mut GodotNodeHandle),
        (
            Or<(Added<Transform3D>, Changed<Transform3D>)>,
            With<Node3DMarker>,
        ),
    >,
) {
    // Early return if transform syncing is disabled
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

#[main_thread_system]
pub fn pre_update_godot_transforms_3d(
    config: Res<crate::plugins::core::GodotTransformConfig>,
    mut entities: Query<(&mut Transform3D, &mut GodotNodeHandle), With<Node3DMarker>>,
) {
    // Early return if not using two-way sync
    if config.sync_mode != crate::plugins::core::TransformSyncMode::TwoWay {
        return;
    }

    for (mut transform, mut reference) in entities.iter_mut() {
        // Skip entities that were changed recently (e.g., by PhysicsUpdate systems)
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
pub fn post_update_godot_transforms_2d(
    config: Res<crate::plugins::core::GodotTransformConfig>,
    mut entities: Query<
        (&Transform2D, &mut GodotNodeHandle),
        (
            Or<(Added<Transform2D>, Changed<Transform2D>)>,
            With<Node2DMarker>,
        ),
    >,
) {
    // Early return if transform syncing is disabled
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

#[main_thread_system]
pub fn pre_update_godot_transforms_2d(
    config: Res<crate::plugins::core::GodotTransformConfig>,
    mut entities: Query<(&mut Transform2D, &mut GodotNodeHandle), With<Node2DMarker>>,
) {
    // Early return if not using two-way sync
    if config.sync_mode != crate::plugins::core::TransformSyncMode::TwoWay {
        return;
    }

    for (mut transform, mut reference) in entities.iter_mut() {
        // Skip entities that were changed recently (e.g., by PhysicsUpdate systems)
        if transform.is_changed() {
            continue;
        }

        if let Some(obj) = reference.try_get::<Node2D>() {
            let mut obj_transform = GodotTransform2D::IDENTITY.translated(obj.get_position());
            obj_transform = obj_transform.rotated(obj.get_rotation());
            obj_transform = obj_transform.scaled(obj.get_scale());

            if obj_transform != *transform.as_godot() {
                *transform.as_godot_mut() = obj_transform;
            }
        }
    }
}
