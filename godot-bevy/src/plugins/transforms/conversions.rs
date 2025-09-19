use bevy::math::{Quat, Vec3, vec3};
use bevy::prelude::Transform as BevyTransform;
use godot::builtin::{Basis, Quaternion, Transform2D as GodotTransform2D, Vector3};
use godot::builtin::{Transform3D as GodotTransform3D, Vector2};

pub trait IntoBevyTransform {
    fn to_bevy_transform(self) -> BevyTransform;
}

impl IntoBevyTransform for GodotTransform3D {
    #[inline]
    fn to_bevy_transform(self) -> BevyTransform {
        let translation = self.origin.to_vec3();

        // Extract scale first
        let scale = self.basis.get_scale().to_vec3();

        // Get rotation from the basis
        // Note: get_quaternion() internally calls orthonormalized() to handle scaled bases
        let rotation = self.basis.get_quaternion().to_quat();

        BevyTransform {
            translation,
            rotation,
            scale,
        }
    }
}

impl IntoBevyTransform for GodotTransform2D {
    #[inline]
    fn to_bevy_transform(self) -> BevyTransform {
        // Extract 2D position
        let translation = vec3(self.origin.x, self.origin.y, 0.0);

        // Extract 2D rotation (z-axis rotation from the 2D transform matrix)
        // Optimization: We only need the angle for quaternion construction
        let rotation_angle = self.a.y.atan2(self.a.x);
        let rotation = Quat::from_rotation_z(rotation_angle);

        // Extract 2D scale from the transform matrix
        // Optimization: Could use length_squared and defer sqrt, but we need the actual scale values
        let scale_x = (self.a.x * self.a.x + self.a.y * self.a.y).sqrt();
        let scale_y = (self.b.x * self.b.x + self.b.y * self.b.y).sqrt();
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
    #[inline]
    fn to_godot_transform(self) -> GodotTransform3D {
        let quat = self.rotation.to_quaternion();

        // Create rotation basis from quaternion
        let rotation_basis = Basis::from_quaternion(quat);

        // Scale each basis vector (column) by the corresponding scale component
        // This is different from basis.scaled() which does a left multiplication
        let basis = Basis::from_cols(
            rotation_basis.col_a() * self.scale.x,
            rotation_basis.col_b() * self.scale.y,
            rotation_basis.col_c() * self.scale.z,
        );

        let origin = self.translation.to_vector3();

        GodotTransform3D { basis, origin }
    }
}

impl IntoGodotTransform2D for BevyTransform {
    #[inline]
    fn to_godot_transform_2d(self) -> GodotTransform2D {
        // For 2D transforms, we expect a quaternion representing pure Z-axis rotation
        // A pure Z rotation has the form: (0, 0, sin(θ/2), cos(θ/2))
        // We can check if x and y components are near zero
        let rotation_z = if self.rotation.x.abs() < 1e-6 && self.rotation.y.abs() < 1e-6 {
            // Pure Z rotation - use optimized extraction
            // angle = 2 * atan2(z, w)
            2.0 * self.rotation.z.atan2(self.rotation.w)
        } else {
            // Complex rotation - fall back to full Euler conversion
            let (_, _, z) = self.rotation.to_euler(bevy::math::EulerRot::XYZ);
            z
        };

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

#[cfg(test)]
mod tests {
    use std::f32;

    use super::*;

    const EPSILON: f32 = 1e-5;

    fn assert_vec3_near(a: Vec3, b: Vec3, epsilon: f32) {
        assert!(
            (a.x - b.x).abs() < epsilon,
            "x component mismatch: {} vs {}",
            a.x,
            b.x
        );
        assert!(
            (a.y - b.y).abs() < epsilon,
            "y component mismatch: {} vs {}",
            a.y,
            b.y
        );
        assert!(
            (a.z - b.z).abs() < epsilon,
            "z component mismatch: {} vs {}",
            a.z,
            b.z
        );
    }

    fn assert_quat_near(a: Quat, b: Quat, epsilon: f32) {
        // Quaternions q and -q represent the same rotation
        let dot = a.dot(b);
        let b_adjusted = if dot < 0.0 { -b } else { b };

        assert!(
            (a.x - b_adjusted.x).abs() < epsilon,
            "x component mismatch: {} vs {}",
            a.x,
            b_adjusted.x
        );
        assert!(
            (a.y - b_adjusted.y).abs() < epsilon,
            "y component mismatch: {} vs {}",
            a.y,
            b_adjusted.y
        );
        assert!(
            (a.z - b_adjusted.z).abs() < epsilon,
            "z component mismatch: {} vs {}",
            a.z,
            b_adjusted.z
        );
        assert!(
            (a.w - b_adjusted.w).abs() < epsilon,
            "w component mismatch: {} vs {}",
            a.w,
            b_adjusted.w
        );
    }

    #[test]
    fn test_vector3_conversions() {
        // Test Vec3 to Vector3
        let bevy_vec = Vec3::new(1.0, 2.0, 3.0);
        let godot_vec = bevy_vec.to_vector3();
        assert_eq!(godot_vec.x, bevy_vec.x);
        assert_eq!(godot_vec.y, bevy_vec.y);
        assert_eq!(godot_vec.z, bevy_vec.z);

        // Test Vector3 to Vec3
        let godot_vec = Vector3::new(4.0, 5.0, 6.0);
        let bevy_vec = godot_vec.to_vec3();
        assert_eq!(bevy_vec.x, godot_vec.x);
        assert_eq!(bevy_vec.y, godot_vec.y);
        assert_eq!(bevy_vec.z, godot_vec.z);

        // Round trip
        let original = Vec3::new(1.5, -2.7, f32::consts::PI);
        let round_trip = original.to_vector3().to_vec3();
        assert_vec3_near(original, round_trip, EPSILON);
    }

    #[test]
    fn test_quaternion_conversions() {
        // Test Quat to Quaternion
        let bevy_quat = Quat::from_rotation_y(std::f32::consts::PI / 4.0);
        let godot_quat = bevy_quat.to_quaternion();
        assert!((godot_quat.x - bevy_quat.x).abs() < EPSILON);
        assert!((godot_quat.y - bevy_quat.y).abs() < EPSILON);
        assert!((godot_quat.z - bevy_quat.z).abs() < EPSILON);
        assert!((godot_quat.w - bevy_quat.w).abs() < EPSILON);

        // Test Quaternion to Quat
        let godot_quat = Quaternion::new(0.0, 0.707, 0.0, 0.707);
        let bevy_quat = godot_quat.to_quat();
        assert!((bevy_quat.x - godot_quat.x).abs() < EPSILON);
        assert!((bevy_quat.y - godot_quat.y).abs() < EPSILON);
        assert!((bevy_quat.z - godot_quat.z).abs() < EPSILON);
        assert!((bevy_quat.w - godot_quat.w).abs() < EPSILON);

        // Round trip
        let original = Quat::from_euler(bevy::math::EulerRot::XYZ, 0.1, 0.2, 0.3);
        let round_trip = original.to_quaternion().to_quat();
        assert_quat_near(original, round_trip, EPSILON);
    }

    #[test]
    fn test_transform_3d_identity() {
        // Test identity transform
        let bevy_transform = BevyTransform::IDENTITY;
        let godot_transform = bevy_transform.to_godot_transform();
        let back_to_bevy = godot_transform.to_bevy_transform();

        assert_vec3_near(back_to_bevy.translation, Vec3::ZERO, EPSILON);
        assert_quat_near(back_to_bevy.rotation, Quat::IDENTITY, EPSILON);
        assert_vec3_near(back_to_bevy.scale, Vec3::ONE, EPSILON);
    }

    #[test]
    fn test_transform_3d_translation_only() {
        let bevy_transform = BevyTransform::from_translation(Vec3::new(10.0, 20.0, 30.0));
        let godot_transform = bevy_transform.to_godot_transform();
        let back_to_bevy = godot_transform.to_bevy_transform();

        assert_vec3_near(
            back_to_bevy.translation,
            bevy_transform.translation,
            EPSILON,
        );
        assert_quat_near(back_to_bevy.rotation, Quat::IDENTITY, EPSILON);
        assert_vec3_near(back_to_bevy.scale, Vec3::ONE, EPSILON);
    }

    #[test]
    fn test_transform_3d_rotation_only() {
        let bevy_transform =
            BevyTransform::from_rotation(Quat::from_rotation_y(std::f32::consts::PI / 3.0));
        let godot_transform = bevy_transform.to_godot_transform();
        let back_to_bevy = godot_transform.to_bevy_transform();

        assert_vec3_near(back_to_bevy.translation, Vec3::ZERO, EPSILON);
        assert_quat_near(back_to_bevy.rotation, bevy_transform.rotation, EPSILON);
        assert_vec3_near(back_to_bevy.scale, Vec3::ONE, EPSILON);
    }

    #[test]
    fn test_transform_3d_scale_only() {
        let bevy_transform = BevyTransform::from_scale(Vec3::new(2.0, 0.5, 3.0));
        let godot_transform = bevy_transform.to_godot_transform();
        let back_to_bevy = godot_transform.to_bevy_transform();

        assert_vec3_near(back_to_bevy.translation, Vec3::ZERO, EPSILON);
        assert_quat_near(back_to_bevy.rotation, Quat::IDENTITY, EPSILON);
        assert_vec3_near(back_to_bevy.scale, bevy_transform.scale, EPSILON);
    }

    #[test]
    fn test_transform_3d_complex() {
        let bevy_transform = BevyTransform {
            translation: Vec3::new(5.0, -10.0, 15.0),
            rotation: Quat::from_euler(bevy::math::EulerRot::XYZ, 0.1, 0.2, 0.3),
            scale: Vec3::new(1.5, 2.0, 0.75),
        };
        let godot_transform = bevy_transform.to_godot_transform();
        let back_to_bevy = godot_transform.to_bevy_transform();

        assert_vec3_near(
            back_to_bevy.translation,
            bevy_transform.translation,
            EPSILON,
        );
        assert_quat_near(back_to_bevy.rotation, bevy_transform.rotation, EPSILON);
        assert_vec3_near(back_to_bevy.scale, bevy_transform.scale, EPSILON);
    }

    #[test]
    fn test_transform_2d_identity() {
        let bevy_transform = BevyTransform::IDENTITY;
        let godot_transform = bevy_transform.to_godot_transform_2d();
        let back_to_bevy = godot_transform.to_bevy_transform();

        assert_vec3_near(back_to_bevy.translation, Vec3::ZERO, EPSILON);
        // For 2D, we only care about Z rotation
        assert!((back_to_bevy.scale.x - 1.0).abs() < EPSILON);
        assert!((back_to_bevy.scale.y - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_transform_2d_translation_only() {
        let bevy_transform = BevyTransform::from_translation(Vec3::new(10.0, 20.0, 0.0));
        let godot_transform = bevy_transform.to_godot_transform_2d();
        let back_to_bevy = godot_transform.to_bevy_transform();

        assert!((back_to_bevy.translation.x - bevy_transform.translation.x).abs() < EPSILON);
        assert!((back_to_bevy.translation.y - bevy_transform.translation.y).abs() < EPSILON);
        assert!((back_to_bevy.scale.x - 1.0).abs() < EPSILON);
        assert!((back_to_bevy.scale.y - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_transform_2d_rotation_only() {
        let angle = std::f32::consts::PI / 4.0;
        let bevy_transform = BevyTransform::from_rotation(Quat::from_rotation_z(angle));
        let godot_transform = bevy_transform.to_godot_transform_2d();
        let back_to_bevy = godot_transform.to_bevy_transform();

        assert_vec3_near(back_to_bevy.translation, Vec3::ZERO, EPSILON);

        // Check that the Z rotation is preserved
        let (_, _, z_rot) = back_to_bevy.rotation.to_euler(bevy::math::EulerRot::XYZ);
        assert!(
            (z_rot - angle).abs() < EPSILON,
            "Z rotation mismatch: {} vs {}",
            z_rot,
            angle
        );

        assert!((back_to_bevy.scale.x - 1.0).abs() < EPSILON);
        assert!((back_to_bevy.scale.y - 1.0).abs() < EPSILON);
    }

    #[test]
    fn test_transform_2d_scale_only() {
        let bevy_transform = BevyTransform::from_scale(Vec3::new(2.0, 0.5, 1.0));
        let godot_transform = bevy_transform.to_godot_transform_2d();
        let back_to_bevy = godot_transform.to_bevy_transform();

        assert_vec3_near(back_to_bevy.translation, Vec3::ZERO, EPSILON);
        assert!((back_to_bevy.scale.x - bevy_transform.scale.x).abs() < EPSILON);
        assert!((back_to_bevy.scale.y - bevy_transform.scale.y).abs() < EPSILON);
    }

    #[test]
    fn test_transform_2d_complex() {
        let bevy_transform = BevyTransform {
            translation: Vec3::new(5.0, -10.0, 0.0),
            rotation: Quat::from_rotation_z(0.785), // 45 degrees
            scale: Vec3::new(1.5, 2.0, 1.0),
        };
        let godot_transform = bevy_transform.to_godot_transform_2d();
        let back_to_bevy = godot_transform.to_bevy_transform();

        assert!((back_to_bevy.translation.x - bevy_transform.translation.x).abs() < EPSILON);
        assert!((back_to_bevy.translation.y - bevy_transform.translation.y).abs() < EPSILON);

        // Check Z rotation is preserved
        let (_, _, original_z) = bevy_transform.rotation.to_euler(bevy::math::EulerRot::XYZ);
        let (_, _, back_z) = back_to_bevy.rotation.to_euler(bevy::math::EulerRot::XYZ);
        assert!(
            (back_z - original_z).abs() < EPSILON,
            "Z rotation mismatch: {} vs {}",
            back_z,
            original_z
        );

        assert!((back_to_bevy.scale.x - bevy_transform.scale.x).abs() < EPSILON);
        assert!((back_to_bevy.scale.y - bevy_transform.scale.y).abs() < EPSILON);
    }

    #[test]
    fn test_vector2_to_vec3() {
        let vec2 = Vector2::new(1.0, 2.0);
        let vec3 = vec2.to_vec3();
        assert_eq!(vec3.x, 1.0);
        assert_eq!(vec3.y, 2.0);
        assert_eq!(vec3.z, 0.0);
    }
}
