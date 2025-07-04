use std::marker::PhantomData;

use bevy::ecs::component::Component;
use bevy::prelude::Transform as BevyTransform;
use godot::builtin::Transform2D as GodotTransform2D;
use godot::prelude::Transform3D as GodotTransform3D;

use super::conversions::{IntoBevyTransform, IntoGodotTransform, IntoGodotTransform2D};

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

// 2D Transform Components

#[derive(Debug, Component, Default, Copy, Clone)]
pub struct Transform2D {
    bevy: bevy::prelude::Transform,
    godot: godot::builtin::Transform2D,
}

impl Transform2D {
    pub fn as_bevy(&self) -> &bevy::prelude::Transform {
        &self.bevy
    }

    pub fn as_bevy_mut(&mut self) -> Transform2DMutGuard<'_, BevyTransform> {
        self.into()
    }

    pub fn as_godot(&self) -> &godot::builtin::Transform2D {
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
