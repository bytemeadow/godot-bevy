use bevy_math::{Quat, Vec3, vec3};
use bevy_transform::components::Transform as BevyTransform;
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
        let translation = vec3(self.origin.x, self.origin.y, 0.0);

        // Scale = column lengths of the 2D rotation+scale matrix
        let scale_x = (self.a.x * self.a.x + self.a.y * self.a.y).sqrt();
        let scale_y = (self.b.x * self.b.x + self.b.y * self.b.y).sqrt();
        let scale = Vec3::new(scale_x, scale_y, 1.0);

        // Build quaternion from the normalized matrix column using half-angle identities:
        //   cos(θ) = a.x / scale_x,  sin(θ) = a.y / scale_x
        //   q.w = cos(θ/2) = sqrt((1 + cos(θ)) / 2)
        //   q.z = sin(θ/2) = sin(θ) / (2 · q.w)
        let rotation = if scale_x > 1e-6 {
            let cos_theta = self.a.x / scale_x;
            let sin_theta = self.a.y / scale_x;
            let cos_half = ((1.0 + cos_theta) * 0.5).sqrt();
            let sin_half = if cos_half.abs() > 1e-6 {
                sin_theta / (2.0 * cos_half)
            } else {
                // θ ≈ ±π, cos(θ/2) ≈ 0, sin(θ/2) ≈ ±1
                if sin_theta >= 0.0 { 1.0 } else { -1.0 }
            };
            Quat::from_xyzw(0.0, 0.0, sin_half, cos_half)
        } else {
            Quat::IDENTITY
        };

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
        // Derive cos(θ) and sin(θ) from the quaternion using the double-angle identity:
        //   cos(θ) = w² - z²
        //   sin(θ) = 2·w·z
        let (cos_rot, sin_rot) = if self.rotation.x.abs() < 1e-6 && self.rotation.y.abs() < 1e-6 {
            let w = self.rotation.w;
            let z = self.rotation.z;
            (w * w - z * z, 2.0 * w * z)
        } else {
            let (_, _, angle) = self.rotation.to_euler(bevy_math::EulerRot::XYZ);
            // sin_cos() returns (sin, cos); this tuple is (cos, sin)
            let (sin, cos) = angle.sin_cos();
            (cos, sin)
        };

        // Apply scale to rotation matrix
        let a = Vector2::new(cos_rot * self.scale.x, sin_rot * self.scale.x);
        let b = Vector2::new(-sin_rot * self.scale.y, cos_rot * self.scale.y);
        let origin = Vector2::new(self.translation.x, self.translation.y);

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

/// q and -q are the same rotation, so sign-normalize via the dot before
/// the per-component compare.
pub(crate) fn quats_differ(a: Quat, b: Quat, epsilon: f32) -> bool {
    let b = if a.dot(b) < 0.0 { -b } else { b };
    (a.x - b.x).abs() > epsilon
        || (a.y - b.y).abs() > epsilon
        || (a.z - b.z).abs() > epsilon
        || (a.w - b.w).abs() > epsilon
}

#[cfg(test)]
include!("conversions_tests.rs");
