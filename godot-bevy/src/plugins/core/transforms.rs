use std::marker::PhantomData;

use bevy::app::{App, Last, Plugin, PreUpdate};
use bevy::ecs::change_detection::DetectChanges;
use bevy::ecs::query::{Added, Changed, Or};
use bevy::ecs::system::Query;
use bevy::math::Vec3;
use bevy::prelude::Res;
use bevy::prelude::Transform as BevyTransform;
use bevy::{ecs::component::Component, math::Quat};
use godot::builtin::Transform2D as GodotTransform2D;
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

impl Transform3D {
    pub fn as_bevy(&self) -> &bevy::prelude::Transform {
        &self.bevy
    }

    pub fn as_bevy_mut(&mut self) -> TransformMutGuard<'_, BevyTransform> {
        self.into()
    }

    pub fn as_godot(&self) -> &godot::prelude::Transform3D {
        &self.godot
    }

    pub fn as_godot_mut(&mut self) -> TransformMutGuard<'_, GodotTransform3D> {
        self.into()
    }

    fn update_godot(&mut self) {
        self.godot = self.bevy.to_godot_transform();
    }

    fn update_bevy(&mut self) {
        self.bevy = self.godot.to_bevy_transform();
    }
}

impl From<BevyTransform> for Transform3D {
    fn from(bevy: BevyTransform) -> Self {
        Self {
            bevy,
            godot: bevy.to_godot_transform(),
        }
    }
}

impl From<GodotTransform3D> for Transform3D {
    fn from(godot: GodotTransform3D) -> Self {
        Self {
            bevy: godot.to_bevy_transform(),
            godot,
        }
    }
}

#[derive(Copy, Clone)]
enum TransformRequested {
    Bevy,
    Godot,
}

pub struct TransformMutGuard<'a, T>(&'a mut Transform3D, TransformRequested, PhantomData<T>);

impl<'a> std::ops::Deref for TransformMutGuard<'a, GodotTransform3D> {
    type Target = GodotTransform3D;
    fn deref(&self) -> &Self::Target {
        &self.0.godot
    }
}

impl<'a> std::ops::DerefMut for TransformMutGuard<'a, GodotTransform3D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.godot
    }
}

impl<'a> std::ops::Deref for TransformMutGuard<'a, BevyTransform> {
    type Target = BevyTransform;
    fn deref(&self) -> &Self::Target {
        &self.0.bevy
    }
}

impl<'a> std::ops::DerefMut for TransformMutGuard<'a, BevyTransform> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.bevy
    }
}

impl<'a> From<&'a mut Transform3D> for TransformMutGuard<'a, GodotTransform3D> {
    fn from(transform: &'a mut Transform3D) -> Self {
        TransformMutGuard(transform, TransformRequested::Godot, PhantomData)
    }
}

impl<'a> From<&'a mut Transform3D> for TransformMutGuard<'a, BevyTransform> {
    fn from(transform: &'a mut Transform3D) -> Self {
        TransformMutGuard(transform, TransformRequested::Bevy, PhantomData)
    }
}

impl<'a, T> Drop for TransformMutGuard<'a, T> {
    fn drop(&mut self) {
        match self.1 {
            TransformRequested::Bevy => self.0.update_godot(),
            TransformRequested::Godot => self.0.update_bevy(),
        }
    }
}

pub trait IntoBevyTransform {
    fn to_bevy_transform(self) -> bevy::prelude::Transform;
}

impl IntoBevyTransform for godot::prelude::Transform3D {
    fn to_bevy_transform(self) -> bevy::prelude::Transform {
        let quat = self.basis.get_quaternion();
        let quat = Quat::from_xyzw(quat.x, quat.y, quat.z, quat.w);

        let scale = self.basis.get_scale();
        let scale = Vec3::new(scale.x, scale.y, scale.z);

        let origin = Vec3::new(self.origin.x, self.origin.y, self.origin.z);

        bevy::prelude::Transform {
            rotation: quat,
            translation: origin,
            scale,
        }
    }
}

impl IntoBevyTransform for godot::prelude::Transform2D {
    fn to_bevy_transform(self) -> bevy::prelude::Transform {
        // Extract 2D position
        let translation = Vec3::new(self.origin.x, self.origin.y, 0.0);

        // Extract 2D rotation (z-axis rotation from the 2D transform matrix)
        let rotation_angle = self.a.y.atan2(self.a.x);
        let rotation = Quat::from_rotation_z(rotation_angle);

        // Extract 2D scale from the transform matrix
        let scale_x = self.a.length();
        let scale_y = self.b.length();
        let scale = Vec3::new(scale_x, scale_y, 1.0);

        bevy::prelude::Transform {
            translation,
            rotation,
            scale,
        }
    }
}

pub trait IntoGodotTransform {
    fn to_godot_transform(self) -> godot::prelude::Transform3D;
}

pub trait IntoGodotTransform2D {
    fn to_godot_transform_2d(self) -> godot::prelude::Transform2D;
}

impl IntoGodotTransform for bevy::prelude::Transform {
    fn to_godot_transform(self) -> godot::prelude::Transform3D {
        let [x, y, z, w] = self.rotation.to_array();
        let quat = Quaternion::new(x, y, z, w);

        let [sx, sy, sz] = self.scale.to_array();
        let scale = Vector3::new(sx, sy, sz);

        let basis = Basis::from_quaternion(quat).scaled(scale);

        let [tx, ty, tz] = self.translation.to_array();
        let origin = Vector3::new(tx, ty, tz);

        godot::prelude::Transform3D { basis, origin }
    }
}

impl IntoGodotTransform2D for bevy::prelude::Transform {
    fn to_godot_transform_2d(self) -> godot::prelude::Transform2D {
        // Extract the Z rotation component from the quaternion
        let (_, _, rotation_z) = self.rotation.to_euler(bevy::math::EulerRot::XYZ);

        // Create 2D rotation matrix
        let cos_rot = rotation_z.cos();
        let sin_rot = rotation_z.sin();

        // Apply scale to rotation matrix
        let a = godot::builtin::Vector2::new(cos_rot * self.scale.x, sin_rot * self.scale.x);
        let b = godot::builtin::Vector2::new(-sin_rot * self.scale.y, cos_rot * self.scale.y);
        let origin = godot::builtin::Vector2::new(self.translation.x, self.translation.y);

        godot::prelude::Transform2D { a, b, origin }
    }
}

#[derive(Debug, Component, Clone, Copy)]
pub struct Transform2D {
    bevy: bevy::prelude::Transform,
    godot: godot::prelude::Transform2D,
}

impl Transform2D {
    pub fn as_bevy(&self) -> &bevy::prelude::Transform {
        &self.bevy
    }

    pub fn as_bevy_mut(&mut self) -> Transform2DMutGuard<'_, BevyTransform> {
        self.into()
    }

    pub fn as_godot(&self) -> &godot::prelude::Transform2D {
        &self.godot
    }

    pub fn as_godot_mut(&mut self) -> Transform2DMutGuard<'_, GodotTransform2D> {
        self.into()
    }

    fn update_godot(&mut self) {
        self.godot = self.bevy.to_godot_transform_2d();
    }

    fn update_bevy(&mut self) {
        self.bevy = self.godot.to_bevy_transform();
    }
}

impl Default for Transform2D {
    fn default() -> Self {
        Self {
            bevy: bevy::prelude::Transform::IDENTITY,
            godot: godot::prelude::Transform2D::IDENTITY,
        }
    }
}

impl From<BevyTransform> for Transform2D {
    fn from(bevy: BevyTransform) -> Self {
        Self {
            bevy,
            godot: bevy.to_godot_transform_2d(),
        }
    }
}

impl From<GodotTransform2D> for Transform2D {
    fn from(godot: GodotTransform2D) -> Self {
        Self {
            bevy: godot.to_bevy_transform(),
            godot,
        }
    }
}

pub struct Transform2DMutGuard<'a, T>(&'a mut Transform2D, TransformRequested, PhantomData<T>);

impl<'a> std::ops::Deref for Transform2DMutGuard<'a, GodotTransform2D> {
    type Target = GodotTransform2D;
    fn deref(&self) -> &Self::Target {
        &self.0.godot
    }
}

impl<'a> std::ops::DerefMut for Transform2DMutGuard<'a, GodotTransform2D> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.godot
    }
}

impl<'a> std::ops::Deref for Transform2DMutGuard<'a, BevyTransform> {
    type Target = BevyTransform;
    fn deref(&self) -> &Self::Target {
        &self.0.bevy
    }
}

impl<'a> std::ops::DerefMut for Transform2DMutGuard<'a, BevyTransform> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0.bevy
    }
}

impl<'a> From<&'a mut Transform2D> for Transform2DMutGuard<'a, GodotTransform2D> {
    fn from(transform: &'a mut Transform2D) -> Self {
        Transform2DMutGuard(transform, TransformRequested::Godot, PhantomData)
    }
}

impl<'a> From<&'a mut Transform2D> for Transform2DMutGuard<'a, BevyTransform> {
    fn from(transform: &'a mut Transform2D) -> Self {
        Transform2DMutGuard(transform, TransformRequested::Bevy, PhantomData)
    }
}

impl<'a, T> Drop for Transform2DMutGuard<'a, T> {
    fn drop(&mut self) {
        match self.1 {
            TransformRequested::Bevy => self.0.update_godot(),
            TransformRequested::Godot => self.0.update_bevy(),
        }
    }
}

pub struct GodotTransformsPlugin;

impl Plugin for GodotTransformsPlugin {
    fn build(&self, app: &mut App) {
        // Always add writing systems
        app.add_systems(Last, post_update_godot_transforms_3d)
            .add_systems(Last, post_update_godot_transforms_2d);

        // Always add reading systems, but they'll check the config at runtime
        app.add_systems(PreUpdate, pre_update_godot_transforms_3d)
            .add_systems(PreUpdate, pre_update_godot_transforms_2d);
    }
}

fn post_update_godot_transforms_3d(
    config: Res<super::GodotTransformConfig>,
    _scene_tree: SceneTreeRef,
    mut entities: Query<
        (&Transform3D, &mut GodotNodeHandle),
        Or<(Added<Transform3D>, Changed<Transform3D>)>,
    >,
) {
    // Early return if transform syncing is disabled
    if config.sync_mode == super::TransformSyncMode::Disabled {
        return;
    }

    for (transform, mut reference) in entities.iter_mut() {
        let mut obj = reference.get::<Node3D>();

        if obj.get_transform() != *transform.as_godot() {
            obj.set_transform(*transform.as_godot());
        }
    }
}

fn pre_update_godot_transforms_3d(
    config: Res<super::GodotTransformConfig>,
    _scene_tree: SceneTreeRef,
    mut entities: Query<(&mut Transform3D, &mut GodotNodeHandle)>,
) {
    // Early return if not using two-way sync
    if config.sync_mode != super::TransformSyncMode::TwoWay {
        return;
    }

    for (mut transform, mut reference) in entities.iter_mut() {
        // Skip entities that were changed recently (e.g., by PhysicsUpdate systems)
        if transform.is_changed() {
            continue;
        }

        let godot_transform = reference.get::<Node3D>().get_transform();
        if *transform.as_godot() != godot_transform {
            *transform.as_godot_mut() = godot_transform;
        }
    }
}

fn post_update_godot_transforms_2d(
    config: Res<super::GodotTransformConfig>,
    _scene_tree: SceneTreeRef,
    mut entities: Query<
        (&Transform2D, &mut GodotNodeHandle),
        Or<(Added<Transform2D>, Changed<Transform2D>)>,
    >,
) {
    // Early return if transform syncing is disabled
    if config.sync_mode == super::TransformSyncMode::Disabled {
        return;
    }

    for (transform, mut reference) in entities.iter_mut() {
        let mut obj = reference.get::<Node2D>();

        let mut obj_transform = GodotTransform2D::IDENTITY.translated(obj.get_position());
        obj_transform = obj_transform.rotated(obj.get_rotation());
        obj_transform = obj_transform.scaled(obj.get_scale());

        if obj_transform != *transform.as_godot() {
            obj.set_transform(*transform.as_godot());
        }
    }
}

fn pre_update_godot_transforms_2d(
    config: Res<super::GodotTransformConfig>,
    _scene_tree: SceneTreeRef,
    mut entities: Query<(&mut Transform2D, &mut GodotNodeHandle)>,
) {
    // Early return if not using two-way sync
    if config.sync_mode != super::TransformSyncMode::TwoWay {
        return;
    }

    for (mut transform, mut reference) in entities.iter_mut() {
        // Skip entities that were changed recently (e.g., by PhysicsUpdate systems)
        if transform.is_changed() {
            continue;
        }

        let obj = reference.get::<Node2D>();

        let mut obj_transform = GodotTransform2D::IDENTITY.translated(obj.get_position());
        obj_transform = obj_transform.rotated(obj.get_rotation());
        obj_transform = obj_transform.scaled(obj.get_scale());

        if obj_transform != *transform.as_godot() {
            *transform.as_godot_mut() = obj_transform;
        }
    }
}
