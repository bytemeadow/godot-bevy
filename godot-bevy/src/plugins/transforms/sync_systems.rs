use crate::interop::GodotNodeHandle;
use crate::interop::node_markers::{Node2DMarker, Node3DMarker};
use crate::plugins::transforms::{IntoBevyTransform, IntoGodotTransform, IntoGodotTransform2D};
use crate::prelude::main_thread_system;
use bevy::ecs::change_detection::{DetectChanges, Ref};
use bevy::ecs::query::{AnyOf, Changed};
use bevy::ecs::system::{Query, SystemChangeTick};
use bevy::prelude::Transform as BevyTransform;
use godot::classes::{Engine, Node2D, Node3D, Object, SceneTree};
use godot::global::godot_print;
use godot::prelude::{Array, Dictionary, Gd, ToGodot};

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
    {
        if let Some(root) = scene_tree.get_root() {
            if let Some(bevy_app) = root.get_node_or_null("BevyAppSingleton") {
                // Check if this BevyApp has the bulk transform methods by trying to call one
                if bevy_app.has_method("update_transforms_bulk_3d") {
                    // Use bulk optimization path
                    static mut BULK_LOG_COUNTER: u32 = 0;
                    unsafe {
                        BULK_LOG_COUNTER += 1;
                        if BULK_LOG_COUNTER % 60 == 1 {
                            // Log once per second at 60fps
                            godot_print!(
                                "godot-bevy: Using bulk transform optimization via BevyApp"
                            );
                        }
                    }
                    let _bulk_span = tracing::info_span!("using_bulk_optimization").entered();
                    post_update_godot_transforms_bulk(
                        change_tick,
                        entities,
                        bevy_app.upcast::<Object>(),
                    );
                    return;
                }
            }
        }
    }

    // Fallback to individual FFI calls
    static mut INDIVIDUAL_LOG_COUNTER: u32 = 0;
    unsafe {
        INDIVIDUAL_LOG_COUNTER += 1;
        if INDIVIDUAL_LOG_COUNTER % 60 == 1 {
            // Log once per second at 60fps
            godot_print!("godot-bevy: Using individual transform sync (fallback)");
        }
    }
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
    let _span = tracing::info_span!("bulk_data_preparation").entered();
    let mut updates_3d = Array::new();
    let mut updates_2d = Array::new();

    for (transform_ref, reference, metadata, (node2d, node3d)) in entities.iter_mut() {
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

        let instance_id = reference.instance_id();

        if node2d.is_some() {
            let mut update = Dictionary::new();
            update.set("instance_id", instance_id);
            update.set("transform", transform_ref.to_godot_transform_2d());
            updates_2d.push(&update);
        } else if node3d.is_some() {
            let godot_transform = transform_ref.to_godot_transform();
            let mut update = Dictionary::new();
            update.set("instance_id", instance_id);
            update.set("basis", godot_transform.basis);
            update.set("origin", godot_transform.origin);
            updates_3d.push(&update);
        }
    }

    // End data preparation phase
    drop(_span);

    // Make bulk FFI calls if we have updates
    let total_updates = updates_3d.len() + updates_2d.len();
    if total_updates > 0 {
        static mut BATCH_LOG_COUNTER: u32 = 0;
        unsafe {
            BATCH_LOG_COUNTER += 1;
            if BATCH_LOG_COUNTER % 60 == 1 {
                // Log once per second at 60fps
                godot_print!(
                    "godot-bevy: Bulk sync processing {} entities ({} 3D, {} 2D)",
                    total_updates,
                    updates_3d.len(),
                    updates_2d.len()
                );
            }
        }

        let _ffi_calls_span = tracing::info_span!("bulk_ffi_calls", total_entities = total_updates).entered();
        
        if !updates_3d.is_empty() {
            let _span = tracing::info_span!("bulk_ffi_call_3d", entities = updates_3d.len()).entered();
            godot_print!("About to call bulk 3D update for {} entities", updates_3d.len());
            batch_singleton.call("update_transforms_bulk_3d", &[updates_3d.to_variant()]);
            godot_print!("Finished bulk 3D update");
        }
        if !updates_2d.is_empty() {
            let _span = tracing::info_span!("bulk_ffi_call_2d", entities = updates_2d.len()).entered();
            godot_print!("About to call bulk 2D update for {} entities", updates_2d.len());
            batch_singleton.call("update_transforms_bulk_2d", &[updates_2d.to_variant()]);
            godot_print!("Finished bulk 2D update");
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
