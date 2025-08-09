use crate::interop::GodotNodeHandle;
use crate::interop::node_markers::{Node2DMarker, Node3DMarker};
use crate::plugins::transforms::{IntoBevyTransform, IntoGodotTransform, IntoGodotTransform2D};
use crate::prelude::main_thread_system;
use bevy::ecs::change_detection::{DetectChanges, Ref};
use bevy::ecs::query::{AnyOf, Changed};
use bevy::ecs::system::{Query, SystemChangeTick};
use bevy::prelude::Transform as BevyTransform;
use godot::classes::{Engine, Node2D, Node3D, Object, SceneTree};
use godot::prelude::{Gd, ToGodot};

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
    entities: Query<
        (
            Ref<BevyTransform>,
            &mut GodotNodeHandle,
            &TransformSyncMetadata,
            AnyOf<(&Node2DMarker, &Node3DMarker)>,
        ),
        Changed<BevyTransform>,
    >,
) {
    // Try to get the BevyAppSingleton autoload for bulk optimization
    let engine = Engine::singleton();
    if let Some(scene_tree) = engine
        .get_main_loop()
        .and_then(|main_loop| main_loop.try_cast::<SceneTree>().ok())
        && let Some(root) = scene_tree.get_root()
        && let Some(bevy_app) = root.get_node_or_null("BevyAppSingleton")
    {
        // Check if this BevyApp has the raw array methods (prefer these over bulk Dictionary methods)
        if bevy_app.has_method("bulk_update_transforms_3d") {
            // Use bulk optimization path
            let _bulk_span = tracing::info_span!("using_bulk_optimization").entered();
            post_update_godot_transforms_bulk(change_tick, entities, bevy_app.upcast::<Object>());
            return;
        }
    }

    // Fallback to individual FFI calls
    post_update_godot_transforms_individual(change_tick, entities);
}

fn post_update_godot_transforms_bulk(
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
    mut batch_singleton: Gd<Object>,
) {
    let _span = tracing::info_span!("bulk_data_preparation_optimized").entered();

    // Pre-allocate vectors with estimated capacity to avoid reallocations
    let entity_count = entities.iter().count();
    let mut instance_ids_3d = Vec::with_capacity(entity_count);
    let mut positions_3d = Vec::with_capacity(entity_count);
    let mut rotations_3d = Vec::with_capacity(entity_count);
    let mut scales_3d = Vec::with_capacity(entity_count);

    let mut instance_ids_2d = Vec::with_capacity(entity_count);
    let mut positions_2d = Vec::with_capacity(entity_count);
    let mut rotations_2d = Vec::with_capacity(entity_count);
    let mut scales_2d = Vec::with_capacity(entity_count);

    // Collect raw transform data (no FFI allocations)
    let _collect_span = tracing::info_span!("collect_raw_arrays").entered();
    for (transform_ref, reference, metadata, (node2d, node3d)) in entities.iter_mut() {
        // Check if we have sync information for this entity
        if let Some(sync_tick) = metadata.last_sync_tick
            && !transform_ref
                .last_changed()
                .is_newer_than(sync_tick, change_tick.this_run())
        {
            // This change was from our Godot sync, skip it
            continue;
        }

        let instance_id = reference.instance_id();

        if node2d.is_some() {
            // Direct field access - avoid transform conversion overhead
            instance_ids_2d.push(instance_id.to_i64());
            positions_2d.push(godot::prelude::Vector2::new(
                transform_ref.translation.x,
                transform_ref.translation.y,
            ));
            // For 2D, rotation is just Z component
            let (_, _, z) = transform_ref.rotation.to_euler(bevy::math::EulerRot::XYZ);
            rotations_2d.push(z);
            scales_2d.push(godot::prelude::Vector2::new(
                transform_ref.scale.x,
                transform_ref.scale.y,
            ));
        } else if node3d.is_some() {
            // Use Bevy transform components directly (avoid complex conversions)
            instance_ids_3d.push(instance_id.to_i64());
            positions_3d.push(godot::prelude::Vector3::new(
                transform_ref.translation.x,
                transform_ref.translation.y,
                transform_ref.translation.z,
            ));

            // Convert Bevy rotation (quaternion) to Euler angles
            let (x, y, z) = transform_ref.rotation.to_euler(bevy::math::EulerRot::XYZ);
            rotations_3d.push(godot::prelude::Vector3::new(x, y, z));

            scales_3d.push(godot::prelude::Vector3::new(
                transform_ref.scale.x,
                transform_ref.scale.y,
                transform_ref.scale.z,
            ));
        }
    }
    drop(_collect_span);

    let has_3d_updates = !instance_ids_3d.is_empty();
    let has_2d_updates = !instance_ids_2d.is_empty();

    // End data preparation phase
    drop(_span);

    // Make raw array FFI calls if we have updates
    let total_updates = instance_ids_3d.len() + instance_ids_2d.len();
    if total_updates > 0 {
        let _ffi_calls_span =
            tracing::info_span!("raw_array_ffi_calls", total_entities = total_updates).entered();

        if has_3d_updates {
            let _span =
                tracing::info_span!("raw_ffi_call_3d", entities = instance_ids_3d.len()).entered();

            // Convert to packed arrays
            let instance_ids_packed =
                godot::prelude::PackedInt64Array::from(instance_ids_3d.as_slice());
            let positions_packed =
                godot::prelude::PackedVector3Array::from(positions_3d.as_slice());
            let rotations_packed =
                godot::prelude::PackedVector3Array::from(rotations_3d.as_slice());
            let scales_packed = godot::prelude::PackedVector3Array::from(scales_3d.as_slice());

            batch_singleton.call(
                "bulk_update_transforms_3d",
                &[
                    instance_ids_packed.to_variant(),
                    positions_packed.to_variant(),
                    rotations_packed.to_variant(),
                    scales_packed.to_variant(),
                ],
            );
        }
        if has_2d_updates {
            let _span =
                tracing::info_span!("raw_ffi_call_2d", entities = instance_ids_2d.len()).entered();

            // Convert to packed arrays
            let instance_ids_packed =
                godot::prelude::PackedInt64Array::from(instance_ids_2d.as_slice());
            let positions_packed =
                godot::prelude::PackedVector2Array::from(positions_2d.as_slice());
            let rotations_packed =
                godot::prelude::PackedFloat32Array::from(rotations_2d.as_slice());
            let scales_packed = godot::prelude::PackedVector2Array::from(scales_2d.as_slice());

            batch_singleton.call(
                "bulk_update_transforms_2d",
                &[
                    instance_ids_packed.to_variant(),
                    positions_packed.to_variant(),
                    rotations_packed.to_variant(),
                    scales_packed.to_variant(),
                ],
            );
        }
    }
}

fn post_update_godot_transforms_individual(
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
    // Original individual FFI approach
    for (transform_ref, mut reference, metadata, (node2d, node3d)) in entities.iter_mut() {
        // Check if we have sync information for this entity
        if let Some(sync_tick) = metadata.last_sync_tick
            && !transform_ref
                .last_changed()
                .is_newer_than(sync_tick, change_tick.this_run())
        {
            // This change was from our Godot sync, skip it
            continue;
        }

        if node2d.is_some() {
            let _span = tracing::info_span!("individual_ffi_call_2d").entered();
            let mut obj = reference.get::<Node2D>();
            obj.set_transform(transform_ref.to_godot_transform_2d());
        } else if node3d.is_some() {
            let _span = tracing::info_span!("individual_ffi_call_3d").entered();
            let mut obj = reference.get::<Node3D>();
            obj.set_transform(transform_ref.to_godot_transform());
        }
    }
}
