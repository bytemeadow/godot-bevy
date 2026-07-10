#[cfg(debug_assertions)]
use crate::interop::BulkOperationsCache;
use crate::interop::node_markers::{Node2DMarker, Node3DMarker};
use crate::interop::{GodotAccess, GodotNodeHandle};
use crate::plugins::transforms::{IntoBevyTransform, IntoGodotTransform, IntoGodotTransform2D};
use bevy_ecs::change_detection::{Mut, Ref};
use bevy_ecs::entity::Entity;
use bevy_ecs::query::{AnyOf, Changed, QueryFilter};
#[cfg(debug_assertions)]
use bevy_ecs::system::NonSendMut;
use bevy_ecs::system::Query;
use bevy_math::Quat;
use bevy_transform::components::Transform as BevyTransform;
#[cfg(debug_assertions)]
use godot::builtin::{PackedInt64Array, VarDictionary};
#[cfg(debug_assertions)]
use godot::classes::Object;
use godot::classes::{Engine, Node, Node2D, Node3D, SceneTree};
use godot::obj::Singleton;
#[cfg(debug_assertions)]
use godot::prelude::{Gd, ToGodot};

use super::change_filter::TransformSyncMetadata;
use super::conversions::quats_differ;

// Match the Godot<->Bevy conversion round-trip tolerance (conversions.rs): the
// individual path derives scale from basis column-length sqrt, which can drift up
// to ~1e-5, so a tighter epsilon would spuriously re-pull scale every frame.
const SCALE_EPSILON: f32 = 1e-5;
const ROTATION_EPSILON: f32 = 1e-5;

fn rotation_differs(a: Quat, b: Quat) -> bool {
    quats_differ(a, b, ROTATION_EPSILON)
}

// merge godot into bevy per-axis: translation & scale per scalar component (godot
// may author some, bevy others), rotation whole. only axes godot actually moved
// are pulled, with the shadow tracking what we've exchanged. returns whether
// anything moved -- caller trips Changed (deref_mut) only then.
pub(crate) fn merge_godot_into_bevy(
    bevy: &mut Mut<BevyTransform>,
    godot: &BevyTransform,
    shadow: &mut BevyTransform,
) -> bool {
    let mut merged = **bevy; // edit a copy so a no-op read never trips Changed
    let mut changed = false;

    // translation exact -- godot round-trips translation f32-exact in both paths
    for i in 0..3 {
        if godot.translation[i] != shadow.translation[i] {
            merged.translation[i] = godot.translation[i];
            shadow.translation[i] = godot.translation[i];
            changed = true;
        }
    }
    // scale tolerates the lossy column-length sqrt in the individual path
    for i in 0..3 {
        if (godot.scale[i] - shadow.scale[i]).abs() > SCALE_EPSILON {
            merged.scale[i] = godot.scale[i];
            shadow.scale[i] = godot.scale[i];
            changed = true;
        }
    }
    if rotation_differs(godot.rotation, shadow.rotation) {
        merged.rotation = godot.rotation;
        shadow.rotation = godot.rotation;
        changed = true;
    }

    if changed {
        **bevy = merged;
    }
    changed
}

// value gate: did Bevy author anything the shadow hasn't seen? same epsilons as
// the read so a value just pulled from Godot reads back clean -- no echo, no FTI
// reset.
pub(crate) fn write_needed(bevy: &BevyTransform, shadow: &BevyTransform) -> bool {
    bevy.translation != shadow.translation
        || (bevy.scale - shadow.scale).abs().max_element() > SCALE_EPSILON
        || rotation_differs(bevy.rotation, shadow.rotation)
}

#[cfg(debug_assertions)]
#[tracing::instrument]
pub fn pre_update_godot_transforms<F: QueryFilter>(
    entities: Query<
        (
            Entity,
            &mut BevyTransform,
            &GodotNodeHandle,
            &mut TransformSyncMetadata,
            AnyOf<(&Node2DMarker, &Node3DMarker)>,
        ),
        F,
    >,
    mut godot: GodotAccess,
    bulk_ops_cache: NonSendMut<BulkOperationsCache>,
) {
    if let Some(bulk_ops) = bulk_ops_cache.get() {
        let _bulk_span = tracing::info_span!("using_bulk_read_optimization").entered();
        pre_update_godot_transforms_bulk(entities, bulk_ops);
        return;
    }
    pre_update_godot_transforms_individual(entities, &mut godot);
}

#[cfg(not(debug_assertions))]
#[tracing::instrument]
pub fn pre_update_godot_transforms<F: QueryFilter>(
    entities: Query<
        (
            Entity,
            &mut BevyTransform,
            &GodotNodeHandle,
            &mut TransformSyncMetadata,
            AnyOf<(&Node2DMarker, &Node3DMarker)>,
        ),
        F,
    >,
    mut godot: GodotAccess,
) {
    pre_update_godot_transforms_individual(entities, &mut godot);
}

#[cfg(debug_assertions)]
fn pre_update_godot_transforms_bulk<F: QueryFilter>(
    mut entities: Query<
        (
            Entity,
            &mut BevyTransform,
            &GodotNodeHandle,
            &mut TransformSyncMetadata,
            AnyOf<(&Node2DMarker, &Node3DMarker)>,
        ),
        F,
    >,
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

        // a dead id aborts the GDScript read -> NIL; skip 3D this step
        if let Ok(result) = batch_singleton
            .call("bulk_get_transforms_3d", &[ids_packed.to_variant()])
            .try_to::<VarDictionary>()
            && let (Some(positions), Some(rotations), Some(scales)) = (
                result
                    .get("positions")
                    .map(|v| v.to::<godot::builtin::PackedVector3Array>()),
                result
                    .get("rotations")
                    .map(|v| v.to::<godot::builtin::PackedVector4Array>()),
                result
                    .get("scales")
                    .map(|v| v.to::<godot::builtin::PackedVector3Array>()),
            )
        {
            for (i, (entity, _)) in entities_3d.iter().enumerate() {
                if let Ok((_, mut bevy_transform, _, mut metadata, _)) = entities.get_mut(*entity)
                    && let (Some(pos), Some(rot), Some(scale)) =
                        (positions.get(i), rotations.get(i), scales.get(i))
                {
                    let godot_transform = BevyTransform {
                        translation: bevy_math::Vec3::new(pos.x, pos.y, pos.z),
                        rotation: Quat::from_xyzw(rot.x, rot.y, rot.z, rot.w),
                        scale: bevy_math::Vec3::new(scale.x, scale.y, scale.z),
                    };

                    merge_godot_into_bevy(
                        &mut bevy_transform,
                        &godot_transform,
                        &mut metadata.shadow,
                    );
                }
            }
        }
    }

    // Process 2D entities
    if !entities_2d.is_empty() {
        let _span = tracing::info_span!("bulk_read_2d", count = entities_2d.len()).entered();

        let instance_ids: Vec<i64> = entities_2d.iter().map(|(_, id)| *id).collect();
        let ids_packed = PackedInt64Array::from(instance_ids.as_slice());

        // a dead id aborts the GDScript read -> NIL; skip 2D this step
        if let Ok(result) = batch_singleton
            .call("bulk_get_transforms_2d", &[ids_packed.to_variant()])
            .try_to::<VarDictionary>()
            && let (Some(positions), Some(rotations), Some(scales)) = (
                result
                    .get("positions")
                    .map(|v| v.to::<godot::builtin::PackedVector2Array>()),
                result
                    .get("rotations")
                    .map(|v| v.to::<godot::builtin::PackedFloat32Array>()),
                result
                    .get("scales")
                    .map(|v| v.to::<godot::builtin::PackedVector2Array>()),
            )
        {
            for (i, (entity, _)) in entities_2d.iter().enumerate() {
                if let Ok((_, mut bevy_transform, _, mut metadata, _)) = entities.get_mut(*entity)
                    && let (Some(pos), Some(rot), Some(scale)) =
                        (positions.get(i), rotations.get(i), scales.get(i))
                {
                    let godot_transform = BevyTransform {
                        translation: bevy_math::Vec3::new(pos.x, pos.y, 0.0),
                        rotation: Quat::from_rotation_z(rot),
                        scale: bevy_math::Vec3::new(scale.x, scale.y, 1.0),
                    };

                    merge_godot_into_bevy(
                        &mut bevy_transform,
                        &godot_transform,
                        &mut metadata.shadow,
                    );
                }
            }
        }
    }
}

fn pre_update_godot_transforms_individual<F: QueryFilter>(
    mut entities: Query<
        (
            Entity,
            &mut BevyTransform,
            &GodotNodeHandle,
            &mut TransformSyncMetadata,
            AnyOf<(&Node2DMarker, &Node3DMarker)>,
        ),
        F,
    >,
    godot: &mut GodotAccess,
) {
    for (_, mut bevy_transform, reference, mut metadata, (node2d, node3d)) in entities.iter_mut() {
        let godot_transform = if node2d.is_some() {
            let Some(node) = godot.try_get::<Node2D>(*reference) else {
                tracing::trace!(target: "godot_transforms",
                    "skipped transform read for freed node {:?}", reference.instance_id());
                continue;
            };
            node.get_transform().to_bevy_transform()
        } else if node3d.is_some() {
            let Some(node) = godot.try_get::<Node3D>(*reference) else {
                tracing::trace!(target: "godot_transforms",
                    "skipped transform read for freed node {:?}", reference.instance_id());
                continue;
            };
            node.get_transform().to_bevy_transform()
        } else {
            panic!("Expected AnyOf to match either a Node2D or a Node3D, is there a bug in bevy?");
        };

        merge_godot_into_bevy(&mut bevy_transform, &godot_transform, &mut metadata.shadow);
    }
}

#[cfg(debug_assertions)]
#[tracing::instrument]
pub fn post_update_godot_transforms<F: QueryFilter>(
    entities: Query<
        (
            Ref<BevyTransform>,
            &GodotNodeHandle,
            &mut TransformSyncMetadata,
            AnyOf<(&Node2DMarker, &Node3DMarker)>,
        ),
        (Changed<BevyTransform>, F),
    >,
    mut godot: GodotAccess,
    bulk_ops_cache: NonSendMut<BulkOperationsCache>,
) {
    if let Some(bulk_ops) = bulk_ops_cache.get() {
        let _bulk_span = tracing::info_span!("using_bulk_optimization").entered();
        post_update_godot_transforms_bulk(entities, bulk_ops, &mut godot);
        return;
    }
    post_update_godot_transforms_individual(entities, &mut godot);
}

#[cfg(not(debug_assertions))]
#[tracing::instrument]
pub fn post_update_godot_transforms<F: QueryFilter>(
    entities: Query<
        (
            Ref<BevyTransform>,
            &GodotNodeHandle,
            &mut TransformSyncMetadata,
            AnyOf<(&Node2DMarker, &Node3DMarker)>,
        ),
        (Changed<BevyTransform>, F),
    >,
    mut godot: GodotAccess,
) {
    post_update_godot_transforms_individual(entities, &mut godot);
}

#[cfg(debug_assertions)]
fn post_update_godot_transforms_bulk<F: QueryFilter>(
    mut entities: Query<
        (
            Ref<BevyTransform>,
            &GodotNodeHandle,
            &mut TransformSyncMetadata,
            AnyOf<(&Node2DMarker, &Node3DMarker)>,
        ),
        (Changed<BevyTransform>, F),
    >,
    mut batch_singleton: Gd<Object>,
    godot: &mut GodotAccess,
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

    // Read once per system run to avoid per-entity FFI.
    let fti_enabled = physics_interpolation_enabled();

    let mut first_write_handles: Vec<GodotNodeHandle> = Vec::new();

    // Collect raw transform data (no FFI allocations)
    let _collect_span = tracing::info_span!("collect_raw_arrays").entered();
    for (transform_ref, reference, mut metadata, (node2d, node3d)) in entities.iter_mut() {
        // value-skip first: a pure-Godot value never trips an FTI reset
        if !write_needed(&transform_ref, &metadata.shadow) {
            continue;
        }

        let is_first_write = !metadata.written_once;
        if is_first_write {
            metadata.written_once = true;
            if fti_enabled {
                first_write_handles.push(*reference);
            }
        }
        metadata.shadow = *transform_ref;

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

    // Reset physics interpolation after the bulk write, so it applies to the just-set transform.
    for handle in first_write_handles {
        if let Some(mut node) = godot.try_get::<Node>(handle) {
            node.reset_physics_interpolation();
        }
    }
}

fn post_update_godot_transforms_individual<F: QueryFilter>(
    mut entities: Query<
        (
            Ref<BevyTransform>,
            &GodotNodeHandle,
            &mut TransformSyncMetadata,
            AnyOf<(&Node2DMarker, &Node3DMarker)>,
        ),
        (Changed<BevyTransform>, F),
    >,
    godot: &mut GodotAccess,
) {
    // Read once per system run to avoid per-entity FFI.
    let fti_enabled = physics_interpolation_enabled();

    for (transform_ref, reference, mut metadata, (node2d, node3d)) in entities.iter_mut() {
        // value-skip first: a pure-Godot value never trips an FTI reset
        if !write_needed(&transform_ref, &metadata.shadow) {
            continue;
        }

        let is_first_write = !metadata.written_once;

        if node2d.is_some() {
            let _span = tracing::info_span!("individual_ffi_call_2d").entered();
            let Some(mut obj) = godot.try_get::<Node2D>(*reference) else {
                tracing::trace!(target: "godot_transforms",
                    "skipped transform write for freed node {:?}", reference.instance_id());
                continue;
            };
            obj.set_transform(transform_ref.to_godot_transform_2d());
        } else if node3d.is_some() {
            let _span = tracing::info_span!("individual_ffi_call_3d").entered();
            let Some(mut obj) = godot.try_get::<Node3D>(*reference) else {
                tracing::trace!(target: "godot_transforms",
                    "skipped transform write for freed node {:?}", reference.instance_id());
                continue;
            };
            obj.set_transform(transform_ref.to_godot_transform());
        }

        metadata.shadow = *transform_ref;
        if is_first_write {
            metadata.written_once = true;
            if fti_enabled && let Some(mut n) = godot.try_get::<Node>(*reference) {
                n.reset_physics_interpolation();
            }
        }
    }
}

/// Whether Godot's project-wide physics interpolation is enabled.
fn physics_interpolation_enabled() -> bool {
    Engine::singleton()
        .get_main_loop()
        .and_then(|ml| ml.try_cast::<SceneTree>().ok())
        .map(|tree| tree.is_physics_interpolation_enabled())
        .unwrap_or(false)
}
