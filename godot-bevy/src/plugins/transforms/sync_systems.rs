use crate::interop::GodotNodeHandle;
use crate::interop::node_markers::{Node2DMarker, Node3DMarker};
use crate::plugins::transforms::{IntoBevyTransform, IntoGodotTransform, IntoGodotTransform2D};
use crate::prelude::main_thread_system;
use bevy::ecs::change_detection::{DetectChanges, Ref};
use bevy::ecs::query::{AnyOf, Changed};
use bevy::ecs::system::{Query, SystemChangeTick};
use bevy::prelude::Transform as BevyTransform;
use godot::classes::{Node2D, Node3D};

use super::change_filter::TransformSyncMetadata;

#[main_thread_system]
#[tracing::instrument]
pub fn pre_update_godot_transforms(
    mut entities: Query<(
        &mut BevyTransform,
        &mut GodotNodeHandle,
        &mut TransformSyncMetadata,
        AnyOf<(&Node2DMarker, &Node3DMarker)>,
    )>,
) {
    for (mut bevy_transform, mut reference, mut metadata, (node2d, node3d)) in entities.iter_mut() {
        let new_bevy_transform = if node2d.is_some() {
            reference
                .get::<Node2D>()
                .get_transform()
                .to_bevy_transform()
        } else if node3d.is_some() {
            reference
                .get::<Node3D>()
                .get_transform()
                .to_bevy_transform()
        } else {
            panic!("Expected AnyOf to match either a Node2D or a Node3D, is there a bug in bevy?");
        };

        // Only write if actually different - avoids triggering change detection
        if *bevy_transform != new_bevy_transform {
            *bevy_transform = new_bevy_transform;

            // Store the last changed tick for this entity, this helps us in the post_ operations
            // to disambiguate our change (syncing from Godot to Bevy above) versus changes that
            // *user* systems do this frame. It's only the latter that we may need to copy back to
            // Godot
            metadata.last_sync_tick = Some(bevy_transform.last_changed());
        }
    }
}

#[main_thread_system]
#[tracing::instrument]
pub fn post_update_godot_transforms(
    change_tick: SystemChangeTick,
    mut entities: Query<
        (
            Ref<BevyTransform>,
            &mut GodotNodeHandle,
            &TransformSyncMetadata,
            AnyOf<(&Node2DMarker, &Node3DMarker)>,
        ),
        Changed<BevyTransform>,
    >,
) {
    for (transform_ref, mut reference, metadata, (node2d, node3d)) in entities.iter_mut() {
        // Check if we have sync information for this entity
        if let Some(sync_tick) = metadata.last_sync_tick {
            if !transform_ref
                .last_changed()
                .is_newer_than(sync_tick, change_tick.this_run())
            {
                // This change was from our Godot sync, skip it
                continue;
            }
        }

        if node2d.is_some() {
            let mut obj = reference.get::<Node2D>();
            obj.set_transform(transform_ref.to_godot_transform_2d());
        } else if node3d.is_some() {
            let mut obj = reference.get::<Node3D>();
            obj.set_transform(transform_ref.to_godot_transform());
        }
    }
}
