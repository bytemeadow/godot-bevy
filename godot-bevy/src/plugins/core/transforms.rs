use bevy::app::{App, Last, Plugin, PreUpdate};
use bevy::ecs::query::{Added, Changed, Or};
use bevy::ecs::system::Query;
use bevy::math::Vec3;
use bevy::prelude::Transform as BevyTransform;
use bevy::{ecs::component::Component, math::Quat};
use godot::builtin::{Basis, Quaternion, Vector3};
use godot::classes::{Node2D, Node3D};
use godot::prelude::Transform3D as GodotTransform3D;

use crate::bridge::GodotNodeHandle;

use super::SceneTreeRef;

#[derive(Debug, Component, Default, Copy, Clone)]
pub struct Transform3D {
    bevy: bevy::prelude::Transform,
    godot: godot::prelude::Transform3D,
}

impl From<BevyTransform> for Transform3D {
    fn from(transform: BevyTransform) -> Self {
        Self {
            bevy: transform,
            godot: Self::bevy_to_godot_transform(transform),
        }
    }
}

impl From<GodotTransform3D> for Transform3D {
    fn from(transform: GodotTransform3D) -> Self {
        Self {
            bevy: Self::godot_to_bevy_transform(transform),
            godot: transform,
        }
    }
}

impl Transform3D {
    pub fn bevy_to_godot_transform(transform: BevyTransform) -> godot::prelude::Transform3D {
        let [x, y, z, w] = transform.rotation.to_array();
        let quat = Quaternion::new(x, y, z, w);

        let [sx, sy, sz] = transform.scale.to_array();
        let scale = Vector3::new(sx, sy, sz);

        let basis = Basis::from_quaternion(quat).scaled(scale);

        let [tx, ty, tz] = transform.translation.to_array();
        let origin = Vector3::new(tx, ty, tz);

        godot::prelude::Transform3D { basis, origin }
    }

    pub fn godot_to_bevy_transform(transform: godot::prelude::Transform3D) -> BevyTransform {
        let quat = transform.basis.get_quaternion();
        let quat = Quat::from_xyzw(quat.x, quat.y, quat.z, quat.w);

        let scale = transform.basis.get_scale();
        let scale = Vec3::new(scale.x, scale.y, scale.z);

        let origin = Vec3::new(transform.origin.x, transform.origin.y, transform.origin.z);

        bevy::prelude::Transform {
            rotation: quat,
            translation: origin,
            scale,
        }
    }

    pub fn get_godot(&self) -> godot::prelude::Transform3D {
        self.godot
    }

    pub fn get_bevy(&self) -> BevyTransform {
        self.bevy
    }
}

#[derive(Debug, Component, Clone, Copy)]
pub struct Transform2D(pub godot::prelude::Transform2D);

impl std::ops::Deref for Transform2D {
    type Target = godot::prelude::Transform2D;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for Transform2D {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub struct GodotTransformsPlugin;

impl Plugin for GodotTransformsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Last, post_update_godot_transforms_3d)
            .add_systems(PreUpdate, pre_update_godot_transforms_3d)
            .add_systems(Last, post_update_godot_transforms_2d)
            .add_systems(PreUpdate, pre_update_godot_transforms_2d);
    }
}

fn post_update_godot_transforms_3d(
    _scene_tree: SceneTreeRef,
    mut entities: Query<
        (&Transform3D, &mut GodotNodeHandle),
        Or<(Added<Transform3D>, Changed<Transform3D>)>,
    >,
) {
    for (transform, mut reference) in entities.iter_mut() {
        let mut obj = reference.get::<Node3D>();

        if obj.get_transform() != transform.get_godot() {
            obj.set_transform(transform.get_godot());
        }
    }
}

fn pre_update_godot_transforms_3d(
    _scene_tree: SceneTreeRef,
    mut entities: Query<(&mut Transform3D, &mut GodotNodeHandle)>,
) {
    for (transform, mut reference) in entities.iter_mut() {
        let mut obj = reference.get::<Node3D>();

        if obj.get_transform() != transform.get_godot() {
            obj.set_transform(transform.get_godot());
        }
    }
}

fn post_update_godot_transforms_2d(
    _scene_tree: SceneTreeRef,
    mut entities: Query<
        (&Transform2D, &mut GodotNodeHandle),
        Or<(Added<Transform2D>, Changed<Transform2D>)>,
    >,
) {
    for (transform, mut reference) in entities.iter_mut() {
        let mut obj = reference.get::<Node2D>();

        if obj.get_transform() != transform.0 {
            obj.set_transform(transform.0);
        }
    }
}

fn pre_update_godot_transforms_2d(
    _scene_tree: SceneTreeRef,
    mut entities: Query<(&mut Transform2D, &mut GodotNodeHandle)>,
) {
    for (transform, mut reference) in entities.iter_mut() {
        let mut obj = reference.get::<Node2D>();

        if obj.get_transform() != transform.0 {
            obj.set_transform(transform.0);
        }
    }
}
