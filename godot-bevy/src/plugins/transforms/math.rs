/// Mathematical utilities for transform conversions.
///
/// These functions provide testable implementations of core mathematical
/// operations used in transform conversion traits.
use bevy::prelude::{Quat, Transform};

/// Extract rotation angle from 2D transform matrix components
pub fn extract_rotation_from_2d_matrix(a_x: f32, a_y: f32) -> f32 {
    a_y.atan2(a_x)
}

/// Extract scale from 2D transform matrix components
pub fn extract_scale_from_2d_matrix(a_x: f32, a_y: f32, b_x: f32, b_y: f32) -> (f32, f32) {
    let scale_x = (a_x * a_x + a_y * a_y).sqrt();
    let scale_y = (b_x * b_x + b_y * b_y).sqrt();
    (scale_x, scale_y)
}

/// Create 2D rotation matrix components from angle and scale
pub fn create_2d_rotation_matrix(
    rotation_z: f32,
    scale_x: f32,
    scale_y: f32,
) -> ((f32, f32), (f32, f32)) {
    let cos_rot = rotation_z.cos();
    let sin_rot = rotation_z.sin();

    let a = (cos_rot * scale_x, sin_rot * scale_x);
    let b = (-sin_rot * scale_y, cos_rot * scale_y);

    (a, b)
}

/// Validate that transform components are reasonable for conversion
pub fn validate_transform_for_conversion(transform: &Transform) -> bool {
    // Check translation is finite
    if !transform.translation.is_finite() {
        return false;
    }

    // Check rotation quaternion is normalized and finite
    if !transform.rotation.is_finite() || !transform.rotation.is_normalized() {
        return false;
    }

    // Check scale is finite and positive
    if !transform.scale.is_finite() || transform.scale.min_element() <= 0.0 {
        return false;
    }

    true
}

/// Extract Z-axis rotation from quaternion (for 2D conversion)
pub fn extract_z_rotation_from_quat(quat: Quat) -> f32 {
    let (_, _, rotation_z) = quat.to_euler(bevy::math::EulerRot::XYZ);
    rotation_z
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy::prelude::Vec3;
    use std::f32::consts::PI;

    #[test]
    fn test_extract_rotation_from_2d_matrix() {
        // Test identity matrix (no rotation)
        assert!((extract_rotation_from_2d_matrix(1.0, 0.0) - 0.0).abs() < 1e-6);

        // Test 90-degree rotation
        assert!((extract_rotation_from_2d_matrix(0.0, 1.0) - PI / 2.0).abs() < 1e-6);
    }

    #[test]
    fn test_extract_scale_from_2d_matrix() {
        // Test identity matrix with scale (2, 3)
        let (scale_x, scale_y) = extract_scale_from_2d_matrix(2.0, 0.0, 0.0, 3.0);
        assert!((scale_x - 2.0).abs() < 1e-6);
        assert!((scale_y - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_create_2d_rotation_matrix() {
        // Test identity rotation with scale
        let ((a_x, a_y), (b_x, b_y)) = create_2d_rotation_matrix(0.0, 2.0, 3.0);
        assert!((a_x - 2.0).abs() < 1e-6);
        assert!(a_y.abs() < 1e-6);
        assert!(b_x.abs() < 1e-6);
        assert!((b_y - 3.0).abs() < 1e-6);
    }

    #[test]
    fn test_validate_transform_for_conversion() {
        // Valid transform
        let valid_transform = Transform {
            translation: Vec3::new(1.0, 2.0, 3.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::new(1.0, 1.0, 1.0),
        };
        assert!(validate_transform_for_conversion(&valid_transform));

        // Invalid translation (NaN)
        let invalid_transform = Transform {
            translation: Vec3::new(f32::NAN, 2.0, 3.0),
            rotation: Quat::IDENTITY,
            scale: Vec3::new(1.0, 1.0, 1.0),
        };
        assert!(!validate_transform_for_conversion(&invalid_transform));
    }

    #[test]
    fn test_extract_z_rotation_from_quat() {
        // Test identity quaternion
        assert!(extract_z_rotation_from_quat(Quat::IDENTITY).abs() < 1e-6);

        // Test Z rotation
        let z_rot_quat = Quat::from_rotation_z(PI / 4.0);
        assert!((extract_z_rotation_from_quat(z_rot_quat) - PI / 4.0).abs() < 1e-6);
    }
}
