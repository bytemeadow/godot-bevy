use crate::interop::GodotNodeHandle;
use crate::interop::node_markers::{Node2DMarker, Node3DMarker};
use crate::plugins::transforms::change_filter::LastGodotSyncTick;
use crate::plugins::transforms::{IntoBevyTransform, IntoGodotTransform, IntoGodotTransform2D};
use crate::prelude::main_thread_system;
use bevy::ecs::change_detection::{DetectChanges, Ref};
use bevy::ecs::query::{Added, Changed, Or, With};
use bevy::ecs::system::{Commands, Query, SystemChangeTick};
use bevy::prelude::{Entity, Transform as BevyTransform};
use godot::classes::{Node2D, Node3D};

#[main_thread_system]
pub fn post_update_godot_transforms_3d(
    mut entities: Query<
        (
            &BevyTransform,
            &mut GodotNodeHandle,
            Option<&LastGodotSyncTick>,
            Ref<BevyTransform>,
        ),
        (
            Or<(Added<BevyTransform>, Changed<BevyTransform>)>,
            With<Node3DMarker>,
        ),
    >,
) {
    for (bevy_transform, mut reference, sync_tick, transform_ref) in entities.iter_mut() {
        // Check if this change happened after our last Godot sync
        if let Some(sync_tick) = sync_tick {
            if transform_ref.last_changed().get() <= sync_tick.tick {
                // This change was from our Godot sync, skip it
                continue;
            }
        }

        let mut obj = reference.get::<Node3D>();
        obj.set_transform(bevy_transform.to_godot_transform());
    }
}

#[main_thread_system]
pub fn pre_update_godot_transforms_3d(
    mut commands: Commands,
    change_tick: SystemChangeTick,
    mut entities: Query<(Entity, &mut BevyTransform, &mut GodotNodeHandle), With<Node3DMarker>>,
) {
    for (entity, mut bevy_transform, mut reference) in entities.iter_mut() {
        let godot_transform = reference.get::<Node3D>().get_transform();
        let new_bevy_transform = godot_transform.to_bevy_transform();

        // Only write if actually different - avoids triggering change detection
        if *bevy_transform != new_bevy_transform {
            *bevy_transform = new_bevy_transform;

            // Store the current tick - any changes after this tick are from Bevy systems
            commands.entity(entity).insert(LastGodotSyncTick {
                tick: change_tick.this_run().get(),
            });
        }
    }
}

#[main_thread_system]
pub fn post_update_godot_transforms_2d(
    mut entities: Query<
        (
            &BevyTransform,
            &mut GodotNodeHandle,
            Option<&LastGodotSyncTick>,
            Ref<BevyTransform>,
        ),
        (
            Or<(Added<BevyTransform>, Changed<BevyTransform>)>,
            With<Node2DMarker>,
        ),
    >,
) {
    for (bevy_transform, mut reference, sync_tick, transform_ref) in entities.iter_mut() {
        // Check if this change happened after our last Godot sync
        if let Some(sync_tick) = sync_tick {
            if transform_ref.last_changed().get() <= sync_tick.tick {
                // This change was from our Godot sync, skip it
                continue;
            }
        }

        let mut obj = reference.get::<Node2D>();
        obj.set_transform(bevy_transform.to_godot_transform_2d());
    }
}

#[main_thread_system]
pub fn pre_update_godot_transforms_2d(
    mut commands: Commands,
    change_tick: SystemChangeTick,
    mut entities: Query<(Entity, &mut BevyTransform, &mut GodotNodeHandle), With<Node2DMarker>>,
) {
    for (entity, mut bevy_transform, mut reference) in entities.iter_mut() {
        let godot_transform = reference.get::<Node2D>().get_transform();
        let new_bevy_transform = godot_transform.to_bevy_transform();

        // Only write if actually different - avoids triggering change detection
        if *bevy_transform != new_bevy_transform {
            *bevy_transform = new_bevy_transform;

            // Store the current tick - any changes after this tick are from Bevy systems
            commands.entity(entity).insert(LastGodotSyncTick {
                tick: change_tick.this_run().get(),
            });
        }
    }
}
