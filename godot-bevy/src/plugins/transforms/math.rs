/// Mathematical utilities for transform conversions.
///
/// These functions provide testable implementations of core mathematical
/// operations used in transform conversion traits.
use bevy_math::Quat;
use bevy_transform::components::Transform;

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
    let (_, _, rotation_z) = quat.to_euler(bevy_math::EulerRot::XYZ);
    rotation_z
}

#[cfg(test)]
include!("math_tests.rs");
