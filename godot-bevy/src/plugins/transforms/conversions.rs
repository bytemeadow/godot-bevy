use bevy::math::{Quat, Vec3};
use bevy::prelude::Transform as BevyTransform;
use godot::builtin::Transform3D as GodotTransform3D;
use godot::builtin::{Basis, Quaternion, Transform2D as GodotTransform2D, Vector3};

pub trait IntoBevyTransform {
    fn to_bevy_transform(self) -> BevyTransform;
}

impl IntoBevyTransform for GodotTransform3D {
    fn to_bevy_transform(self) -> BevyTransform {
        let quat = self.basis.get_quaternion();
        let quat = Quat::from_xyzw(quat.x, quat.y, quat.z, quat.w);

        let scale = self.basis.get_scale();
        let scale = Vec3::new(scale.x, scale.y, scale.z);

        let origin = Vec3::new(self.origin.x, self.origin.y, self.origin.z);

        BevyTransform {
            rotation: quat,
            translation: origin,
            scale,
        }
    }
}

impl IntoBevyTransform for GodotTransform2D {
    fn to_bevy_transform(self) -> BevyTransform {
        // Extract 2D position
        let translation = Vec3::new(self.origin.x, self.origin.y, 0.0);

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
        let [x, y, z, w] = self.rotation.to_array();
        let quat = Quaternion::new(x, y, z, w);

        let [sx, sy, sz] = self.scale.to_array();
        let scale = Vector3::new(sx, sy, sz);

        let basis = Basis::from_quaternion(quat).scaled(scale);

        let [tx, ty, tz] = self.translation.to_array();
        let origin = Vector3::new(tx, ty, tz);

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
