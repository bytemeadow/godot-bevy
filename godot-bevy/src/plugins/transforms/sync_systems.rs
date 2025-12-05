use crate::interop::GodotNodeHandle;
use crate::interop::node_markers::{Node2DMarker, Node3DMarker};
use crate::plugins::transforms::{IntoBevyTransform, IntoGodotTransform, IntoGodotTransform2D};
use crate::prelude::main_thread_system;
use bevy_ecs::change_detection::{DetectChanges, Ref};
use bevy_ecs::entity::Entity;
use bevy_ecs::query::{AnyOf, Changed};
use bevy_ecs::system::{Query, SystemChangeTick};
use bevy_math::Quat;
use bevy_transform::components::Transform as BevyTransform;
use godot::builtin::{Dictionary, PackedInt64Array};
use godot::classes::{Engine, Node, Node2D, Node3D, Object, SceneTree};
use godot::obj::Singleton;
use godot::prelude::{Gd, ToGodot};

use super::change_filter::TransformSyncMetadata;

/// Helper to find the OptimizedBulkOperations node
fn get_bulk_operations_node() -> Option<Gd<Object>> {
    let engine = Engine::singleton();
    let scene_tree = engine
        .get_main_loop()
        .and_then(|main_loop| main_loop.try_cast::<SceneTree>().ok())?;
    let root = scene_tree.get_root()?;

    // Try to find OptimizedBulkOperations as a child of BevyAppSingleton
    root.get_node_or_null("BevyAppSingleton/OptimizedBulkOperations")
        .or_else(|| root.get_node_or_null("/root/BevyAppSingleton/OptimizedBulkOperations"))
        .map(|n: Gd<Node>| n.upcast::<Object>())
}

#[main_thread_system]
#[tracing::instrument]
pub fn pre_update_godot_transforms(
    entities: Query<(
        Entity,
        &mut BevyTransform,
        &mut GodotNodeHandle,
        &mut TransformSyncMetadata,
        AnyOf<(&Node2DMarker, &Node3DMarker)>,
    )>,
) {
    // In debug builds, use bulk optimization (GDScript path) which is faster when Rust FFI
    // overhead is high. In release builds, use individual FFI calls which are faster due to
    // optimized Rust FFI and avoiding GDScript interpreter overhead.
    #[cfg(debug_assertions)]
    {
        if let Some(bulk_ops) = get_bulk_operations_node() {
            let _bulk_span = tracing::info_span!("using_bulk_read_optimization").entered();
            pre_update_godot_transforms_bulk(entities, bulk_ops);
            return;
        }
        pre_update_godot_transforms_individual(entities);
    }

    #[cfg(not(debug_assertions))]
    {
        pre_update_godot_transforms_individual(entities);
    }
}

fn pre_update_godot_transforms_bulk(
    mut entities: Query<(
        Entity,
        &mut BevyTransform,
        &mut GodotNodeHandle,
        &mut TransformSyncMetadata,
        AnyOf<(&Node2DMarker, &Node3DMarker)>,
    )>,
    mut batch_singleton: Gd<Object>,
) {
    let _span = tracing::info_span!("bulk_read_preparation").entered();

    // Collect entity info for 3D and 2D nodes separately
    let mut entities_3d: Vec<(Entity, i64)> = Vec::new();
    let mut entities_2d: Vec<(Entity, i64)> = Vec::new();

    for (entity, _, reference, _, (node2d, node3d)) in entities.iter() {
        let instance_id = reference.instance_id().to_i64();
        if node2d.is_some() {
            entities_2d.push((entity, instance_id));
        } else if node3d.is_some() {
            entities_3d.push((entity, instance_id));
        }
    }

    drop(_span);

    // Process 3D entities
    if !entities_3d.is_empty() {
        let _span = tracing::info_span!("bulk_read_3d", count = entities_3d.len()).entered();

        let instance_ids: Vec<i64> = entities_3d.iter().map(|(_, id)| *id).collect();
        let ids_packed = PackedInt64Array::from(instance_ids.as_slice());

        let result = batch_singleton
            .call("bulk_get_transforms_3d", &[ids_packed.to_variant()])
            .to::<Dictionary>();

        if let (Some(positions), Some(rotations), Some(scales)) = (
            result
                .get("positions")
                .map(|v| v.to::<godot::builtin::PackedVector3Array>()),
            result
                .get("rotations")
                .map(|v| v.to::<godot::builtin::PackedVector4Array>()),
            result
                .get("scales")
                .map(|v| v.to::<godot::builtin::PackedVector3Array>()),
        ) {
            for (i, (entity, _)) in entities_3d.iter().enumerate() {
                if let Ok((_, mut bevy_transform, _, mut metadata, _)) = entities.get_mut(*entity)
                    && let (Some(pos), Some(rot), Some(scale)) =
                        (positions.get(i), rotations.get(i), scales.get(i))
                {
                    let new_bevy_transform = BevyTransform {
                        translation: bevy_math::Vec3::new(pos.x, pos.y, pos.z),
                        rotation: Quat::from_xyzw(rot.x, rot.y, rot.z, rot.w),
                        scale: bevy_math::Vec3::new(scale.x, scale.y, scale.z),
                    };

                    if *bevy_transform != new_bevy_transform {
                        *bevy_transform = new_bevy_transform;
                        metadata.last_sync_tick = Some(bevy_transform.last_changed());
                    }
                }
            }
        }
    }

    // Process 2D entities
    if !entities_2d.is_empty() {
        let _span = tracing::info_span!("bulk_read_2d", count = entities_2d.len()).entered();

        let instance_ids: Vec<i64> = entities_2d.iter().map(|(_, id)| *id).collect();
        let ids_packed = PackedInt64Array::from(instance_ids.as_slice());

        let result = batch_singleton
            .call("bulk_get_transforms_2d", &[ids_packed.to_variant()])
            .to::<Dictionary>();

        if let (Some(positions), Some(rotations), Some(scales)) = (
            result
                .get("positions")
                .map(|v| v.to::<godot::builtin::PackedVector2Array>()),
            result
                .get("rotations")
                .map(|v| v.to::<godot::builtin::PackedFloat32Array>()),
            result
                .get("scales")
                .map(|v| v.to::<godot::builtin::PackedVector2Array>()),
        ) {
            for (i, (entity, _)) in entities_2d.iter().enumerate() {
                if let Ok((_, mut bevy_transform, _, mut metadata, _)) = entities.get_mut(*entity)
                    && let (Some(pos), Some(rot), Some(scale)) =
                        (positions.get(i), rotations.get(i), scales.get(i))
                {
                    let new_bevy_transform = BevyTransform {
                        translation: bevy_math::Vec3::new(pos.x, pos.y, 0.0),
                        rotation: Quat::from_rotation_z(rot),
                        scale: bevy_math::Vec3::new(scale.x, scale.y, 1.0),
                    };

                    if *bevy_transform != new_bevy_transform {
                        *bevy_transform = new_bevy_transform;
                        metadata.last_sync_tick = Some(bevy_transform.last_changed());
                    }
                }
            }
        }
    }
}

fn pre_update_godot_transforms_individual(
    mut entities: Query<(
        Entity,
        &mut BevyTransform,
        &mut GodotNodeHandle,
        &mut TransformSyncMetadata,
        AnyOf<(&Node2DMarker, &Node3DMarker)>,
    )>,
) {
    for (_, mut bevy_transform, mut reference, mut metadata, (node2d, node3d)) in
        entities.iter_mut()
    {
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
    // In debug builds, use bulk optimization (GDScript path) which is faster when Rust FFI
    // overhead is high. In release builds, use individual FFI calls which are faster due to
    // optimized Rust FFI and avoiding GDScript interpreter overhead.
    #[cfg(debug_assertions)]
    {
        if let Some(bulk_ops) = get_bulk_operations_node() {
            let _bulk_span = tracing::info_span!("using_bulk_optimization").entered();
            post_update_godot_transforms_bulk(change_tick, entities, bulk_ops);
            return;
        }
        post_update_godot_transforms_individual(change_tick, entities);
    }

    #[cfg(not(debug_assertions))]
    {
        post_update_godot_transforms_individual(change_tick, entities);
    }
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
            let (_, _, z) = transform_ref.rotation.to_euler(bevy_math::EulerRot::XYZ);
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

            rotations_3d.push(godot::prelude::Vector4 {
                x: transform_ref.rotation.x,
                y: transform_ref.rotation.y,
                z: transform_ref.rotation.z,
                w: transform_ref.rotation.w,
            });

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
                godot::prelude::PackedVector4Array::from(rotations_3d.as_slice());
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
