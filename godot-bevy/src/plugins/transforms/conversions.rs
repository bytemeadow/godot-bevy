use bevy::math::{Quat, Vec3, vec3};
use bevy::prelude::Transform as BevyTransform;
use godot::builtin::{Basis, Quaternion, Transform2D as GodotTransform2D, Vector3};
use godot::builtin::{Transform3D as GodotTransform3D, Vector2};

pub trait IntoBevyTransform {
    fn to_bevy_transform(self) -> BevyTransform;
}

impl IntoBevyTransform for GodotTransform3D {
    fn to_bevy_transform(self) -> BevyTransform {
        let translation = self.origin.to_vec3();
        let rotation = self.basis.get_quaternion().to_quat();
        let scale = self.basis.get_scale().to_vec3();

        BevyTransform {
            translation,
            rotation,
            scale,
        }
    }
}

impl IntoBevyTransform for GodotTransform2D {
    fn to_bevy_transform(self) -> BevyTransform {
        // Extract 2D position
        let translation = self.origin.to_vec3();

        // Extract 2D rotation (z-axis rotation from the 2D transform matrix)
        let rotation_angle = self.a.y.atan2(self.a.x);
        let rotation = Quat::from_rotation_z(rotation_angle);

        // Extract 2D scale from the transform matrix
        let scale_x = self.a.length();
        let scale_y = self.b.length();
        let scale = Vec3::new(scale_x, scale_y, 1.0);

        BevyTransform {
            translation,
            rotation,
            scale,
        }
    }
}

pub trait IntoGodotTransform {
    fn to_godot_transform(self) -> GodotTransform3D;
}

pub trait IntoGodotTransform2D {
    fn to_godot_transform_2d(self) -> GodotTransform2D;
}

impl IntoGodotTransform for BevyTransform {
    fn to_godot_transform(self) -> GodotTransform3D {
        let quat = self.rotation.to_quaternion();

        let scale = self.scale.to_vector3();

        let basis = Basis::from_quaternion(quat).scaled(scale);

        let origin = self.translation.to_vector3();

        GodotTransform3D { basis, origin }
    }
}

impl IntoGodotTransform2D for BevyTransform {
    fn to_godot_transform_2d(self) -> GodotTransform2D {
        // Extract the Z rotation component from the quaternion
        let (_, _, rotation_z) = self.rotation.to_euler(bevy::math::EulerRot::XYZ);

        // Create 2D rotation matrix
        let cos_rot = rotation_z.cos();
        let sin_rot = rotation_z.sin();

        // Apply scale to rotation matrix
        let a = godot::builtin::Vector2::new(cos_rot * self.scale.x, sin_rot * self.scale.x);
        let b = godot::builtin::Vector2::new(-sin_rot * self.scale.y, cos_rot * self.scale.y);
        let origin = godot::builtin::Vector2::new(self.translation.x, self.translation.y);

        GodotTransform2D { a, b, origin }
    }
}

pub trait IntoVector3 {
    fn to_vector3(self) -> Vector3;
}

impl IntoVector3 for Vec3 {
    #[inline]
    fn to_vector3(self) -> Vector3 {
        Vector3::new(self.x, self.y, self.z)
    }
}

pub trait IntoVec3 {
    fn to_vec3(self) -> Vec3;
}

impl IntoVec3 for Vector3 {
    #[inline]
    fn to_vec3(self) -> Vec3 {
        vec3(self.x, self.y, self.z)
    }
}

impl IntoVec3 for Vector2 {
    #[inline]
    fn to_vec3(self) -> Vec3 {
        vec3(self.x, self.y, 0.)
    }
}

pub trait IntoQuat {
    fn to_quat(self) -> Quat;
}

impl IntoQuat for Quaternion {
    #[inline]
    fn to_quat(self) -> Quat {
        Quat::from_xyzw(self.x, self.y, self.z, self.w)
    }
}

pub trait IntoQuaternion {
    fn to_quaternion(self) -> Quaternion;
}

impl IntoQuaternion for Quat {
    #[inline]
    fn to_quaternion(self) -> Quaternion {
        Quaternion::new(self.x, self.y, self.z, self.w)
    }
}
