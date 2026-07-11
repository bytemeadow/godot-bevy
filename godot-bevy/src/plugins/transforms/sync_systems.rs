use crate::interop::node_markers::{Node2DMarker, Node3DMarker};
use crate::interop::{GodotAccess, GodotNodeHandle};
use crate::plugins::transforms::{IntoBevyTransform, IntoGodotTransform, IntoGodotTransform2D};
use bevy_ecs::change_detection::{Mut, Ref};
use bevy_ecs::entity::Entity;
use bevy_ecs::query::{AnyOf, Changed, QueryFilter};
use bevy_ecs::system::Query;
use bevy_math::Quat;
use bevy_transform::components::Transform as BevyTransform;
use godot::classes::{Engine, Node, Node2D, Node3D, SceneTree};
use godot::obj::Singleton;

use super::change_filter::TransformSyncMetadata;
use super::conversions::quats_differ;

// Match the Godot<->Bevy conversion round-trip tolerance (conversions.rs): scale
// is derived from basis column-length sqrt, which can drift up to ~1e-5, so a
// tighter epsilon would spuriously re-pull scale every frame.
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

    // translation exact -- godot round-trips translation f32-exact
    for i in 0..3 {
        if godot.translation[i] != shadow.translation[i] {
            merged.translation[i] = godot.translation[i];
            shadow.translation[i] = godot.translation[i];
            changed = true;
        }
    }
    // scale tolerates the lossy column-length sqrt conversion
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

#[tracing::instrument]
pub fn pre_update_godot_transforms<F: QueryFilter>(
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
    mut godot: GodotAccess,
) {
    for (_, mut bevy_transform, reference, mut metadata, (node2d, node3d)) in entities.iter_mut() {
        let godot_transform = if node2d.is_some() {
            let Some(node) = godot.try_get::<Node2D>(*reference) else {
                continue;
            };
            node.get_transform().to_bevy_transform()
        } else if node3d.is_some() {
            let Some(node) = godot.try_get::<Node3D>(*reference) else {
                continue;
            };
            node.get_transform().to_bevy_transform()
        } else {
            panic!("Expected AnyOf to match either a Node2D or a Node3D, is there a bug in bevy?");
        };

        merge_godot_into_bevy(&mut bevy_transform, &godot_transform, &mut metadata.shadow);
    }
}

#[tracing::instrument]
pub fn post_update_godot_transforms<F: QueryFilter>(
    mut entities: Query<
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
    // Read once per system run to avoid per-entity FFI.
    let fti_enabled = physics_interpolation_enabled();

    for (transform_ref, reference, mut metadata, (node2d, node3d)) in entities.iter_mut() {
        // value-skip first: a pure-Godot value never trips an FTI reset
        if !write_needed(&transform_ref, &metadata.shadow) {
            continue;
        }

        let is_first_write = !metadata.written_once;

        if node2d.is_some() {
            let _span = tracing::info_span!("ffi_call_2d").entered();
            let Some(mut obj) = godot.try_get::<Node2D>(*reference) else {
                continue;
            };
            obj.set_transform(transform_ref.to_godot_transform_2d());
        } else if node3d.is_some() {
            let _span = tracing::info_span!("ffi_call_3d").entered();
            let Some(mut obj) = godot.try_get::<Node3D>(*reference) else {
                continue;
            };
            obj.set_transform(transform_ref.to_godot_transform());
        }

        metadata.shadow = *transform_ref;
        if is_first_write {
            metadata.written_once = true;
            if fti_enabled && let Some(mut node) = godot.try_get::<Node>(*reference) {
                node.reset_physics_interpolation();
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
