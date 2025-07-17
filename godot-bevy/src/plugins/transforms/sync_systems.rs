use crate::interop::GodotNodeHandle;
use crate::interop::node_markers::{Node2DMarker, Node3DMarker};
use crate::plugins::transforms::{IntoBevyTransform, IntoGodotTransform, IntoGodotTransform2D};
use crate::prelude::main_thread_system;
use bevy::ecs::query::{Added, Changed, Or, With};
use bevy::ecs::system::Query;
use bevy::prelude::Transform as BevyTransform;
use godot::classes::{Node2D, Node3D};

#[main_thread_system]
pub fn post_update_godot_transforms_3d(
    mut entities: Query<
        (&BevyTransform, &mut GodotNodeHandle),
        (
            Or<(Added<BevyTransform>, Changed<BevyTransform>)>,
            With<Node3DMarker>,
        ),
    >,
) {
    for (bevy_transform, mut reference) in entities.iter_mut() {
        let mut obj = reference.get::<Node3D>();
        obj.set_transform(bevy_transform.to_godot_transform());
    }
}

pub fn pre_update_godot_transforms_3d(
    mut entities: Query<(&mut BevyTransform, &mut GodotNodeHandle), With<Node3DMarker>>,
) {
    for (mut bevy_transform, mut reference) in entities.iter_mut() {
        let godot_transform = reference.get::<Node3D>().get_transform();
        *bevy_transform = godot_transform.to_bevy_transform();
    }
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
    for (bevy_transform, mut reference) in entities.iter_mut() {
        let mut obj = reference.get::<Node2D>();
        obj.set_transform(bevy_transform.to_godot_transform_2d());
    }
}

pub fn pre_update_godot_transforms_2d(
    mut entities: Query<(&mut BevyTransform, &mut GodotNodeHandle), With<Node2DMarker>>,
) {
    for (mut bevy_transform, mut reference) in entities.iter_mut() {
        let obj = reference.get::<Node2D>();
        *bevy_transform = obj.get_transform().to_bevy_transform();
    }
}
