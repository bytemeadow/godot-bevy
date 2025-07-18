use crate::interop::node_markers::{Node2DMarker, Node3DMarker};
use crate::interop::GodotNodeHandle;
use crate::plugins::transforms::{IntoBevyTransform, IntoGodotTransform, IntoGodotTransform2D};
use crate::prelude::main_thread_system;
use bevy::ecs::change_detection::DetectChanges;
use bevy::ecs::component::{Component, Tick};
use bevy::ecs::entity::Entity;
use bevy::ecs::query::{Added, Changed, Or, With};
use bevy::ecs::system::{Commands, Query};
use bevy::ecs::world::{Mut, Ref};
use bevy::log::info;
use bevy::prelude::Transform as BevyTransform;
use godot::classes::{Node2D, Node3D};

#[derive(Component)]
pub struct PreUpdateMarker(Tick);

#[main_thread_system]
pub fn add_update_marker(
    mut commands: Commands,
    mut entities: Query<(Entity, Ref<BevyTransform>), Added<BevyTransform>>,
) {
    for (e, transform) in entities.iter_mut() {
        commands
            .entity(e)
            .insert_if_new(PreUpdateMarker(transform.last_changed()));
    }
}

// post_update() {
//   // has the bevy transform changed *after* our change marker?  if so, copy bevy to godot
// }
#[main_thread_system]
pub fn post_update_godot_transforms_3d(
    mut entities: Query<
        (Ref<BevyTransform>, &PreUpdateMarker, &mut GodotNodeHandle),
        (Added<BevyTransform>, With<Node3DMarker>),
    >,
) {
    let mut count = 0;
    for (bevy_transform, change_marker, mut godot_node_handle) in entities.iter_mut() {
        if change_marker.0 == bevy_transform.last_changed() {
            info!("skipped updating");
            continue;
        }

        let mut obj = godot_node_handle.get::<Node3D>();
        obj.set_transform(bevy_transform.to_godot_transform());
        count += 1;
    }
    info!("post update count: {count}");
}

// pre_update() {
//   // copy godot to bevy, if they are different
//
//       // oops, now bevy sees the transform has changed, since we changed it.  but this isn't a real user-driven change :-/
//
//       // create our own change marker for right now
// }
#[main_thread_system]
pub fn pre_update_godot_transforms_3d(
    mut entities: Query<
        (
            Mut<BevyTransform>,
            &mut PreUpdateMarker,
            &mut GodotNodeHandle,
        ),
        With<Node3DMarker>,
    >,
) {
    info!("---- frame ----");

    let mut count = 0;
    for (mut bevy_transform, mut change_marker, mut godot_node_handle) in entities.iter_mut() {
        let godot_transform = godot_node_handle
            .get::<Node3D>()
            .get_transform()
            .to_bevy_transform();

        if godot_transform != *bevy_transform {
            *bevy_transform = godot_transform;
            change_marker.0 = bevy_transform.last_changed();
            count += 1;
        }
    }
    info!("pre update count: {count}");
}

#[main_thread_system]
pub fn post_update_godot_transforms_2d(
    mut entities: Query<
        (&BevyTransform, &mut GodotNodeHandle),
        (
            Or<(Added<BevyTransform>, Changed<BevyTransform>)>,
            With<Node2DMarker>,
        ),
    >,
) {
    for (bevy_transform, mut godot_node_handle) in entities.iter_mut() {
        let mut obj = godot_node_handle.get::<Node2D>();
        obj.set_transform(bevy_transform.to_godot_transform_2d());
    }
}

#[main_thread_system]
pub fn pre_update_godot_transforms_2d(
    mut entities: Query<(&mut BevyTransform, &mut GodotNodeHandle), With<Node2DMarker>>,
) {
    for (mut bevy_transform, mut godot_node_handle) in entities.iter_mut() {
        let obj = godot_node_handle.get::<Node2D>();
        *bevy_transform = obj.get_transform().to_bevy_transform();
    }
}
