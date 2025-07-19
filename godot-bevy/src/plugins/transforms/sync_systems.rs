use crate::interop::GodotNodeHandle;
use crate::interop::node_markers::{Node2DMarker, Node3DMarker};
use crate::plugins::transforms::change_filter::GodotSyncedEntities;
use crate::plugins::transforms::{IntoBevyTransform, IntoGodotTransform, IntoGodotTransform2D};
use crate::prelude::main_thread_system;
use bevy::ecs::change_detection::{DetectChanges, Ref};
use bevy::ecs::query::{Added, Changed, Or, With};
use bevy::ecs::system::{Query, Res, ResMut, SystemChangeTick};
use bevy::prelude::{Entity, Transform as BevyTransform};
use godot::classes::{Node2D, Node3D};

/// Clear the synced entities map at the start of each frame
pub fn clear_godot_synced_entities(mut synced_entities: ResMut<GodotSyncedEntities>) {
    synced_entities.synced_entities.clear();
}

#[main_thread_system]
pub fn post_update_godot_transforms_3d(
    change_tick: SystemChangeTick,
    synced_entities: Res<GodotSyncedEntities>,
    mut entities: Query<
        (Entity, Ref<BevyTransform>, &mut GodotNodeHandle),
        (
            Or<(Added<BevyTransform>, Changed<BevyTransform>)>,
            With<Node3DMarker>,
        ),
    >,
) {
    for (entity, transform_ref, mut reference) in entities.iter_mut() {
        // Check if we have sync information for this entity
        if let Some(sync_tick) = synced_entities.synced_entities.get(&entity) {
            if !transform_ref
                .last_changed()
                .is_newer_than(*sync_tick, change_tick.this_run())
            {
                // This change was from our Godot sync, skip it
                continue;
            }
        }

        let mut obj = reference.get::<Node3D>();
        obj.set_transform(transform_ref.to_godot_transform());
    }
}

#[main_thread_system]
pub fn pre_update_godot_transforms_3d(
    mut synced_entities: ResMut<GodotSyncedEntities>,
    mut entities: Query<(Entity, &mut BevyTransform, &mut GodotNodeHandle), With<Node3DMarker>>,
) {
    for (entity, mut bevy_transform, mut reference) in entities.iter_mut() {
        let godot_transform = reference.get::<Node3D>().get_transform();
        let new_bevy_transform = godot_transform.to_bevy_transform();

        // Only write if actually different - avoids triggering change detection
        if *bevy_transform != new_bevy_transform {
            *bevy_transform = new_bevy_transform;

            // Store the sync tick for this entity
            let change_tick = bevy_transform.last_changed();
            synced_entities.synced_entities.insert(entity, change_tick);
        }
    }
}

#[main_thread_system]
pub fn post_update_godot_transforms_2d(
    change_tick: SystemChangeTick,
    synced_entities: Res<GodotSyncedEntities>,
    mut entities: Query<
        (Entity, Ref<BevyTransform>, &mut GodotNodeHandle),
        (
            Or<(Added<BevyTransform>, Changed<BevyTransform>)>,
            With<Node2DMarker>,
        ),
    >,
) {
    for (entity, transform_ref, mut reference) in entities.iter_mut() {
        // Check if we have sync information for this entity
        if let Some(sync_tick) = synced_entities.synced_entities.get(&entity) {
            if !transform_ref
                .last_changed()
                .is_newer_than(*sync_tick, change_tick.this_run())
            {
                // This change was from our Godot sync, skip it
                continue;
            }
        }

        let mut obj = reference.get::<Node2D>();
        obj.set_transform(transform_ref.to_godot_transform_2d());
    }
}

#[main_thread_system]
pub fn pre_update_godot_transforms_2d(
    mut synced_entities: ResMut<GodotSyncedEntities>,
    mut entities: Query<(Entity, &mut BevyTransform, &mut GodotNodeHandle), With<Node2DMarker>>,
) {
    for (entity, mut bevy_transform, mut reference) in entities.iter_mut() {
        let godot_transform = reference.get::<Node2D>().get_transform();
        let new_bevy_transform = godot_transform.to_bevy_transform();

        // Only write if actually different - avoids triggering change detection
        if *bevy_transform != new_bevy_transform {
            *bevy_transform = new_bevy_transform;

            // Store the sync tick for this entity
            let change_tick = bevy_transform.last_changed();
            synced_entities.synced_entities.insert(entity, change_tick);
        }
    }
}
