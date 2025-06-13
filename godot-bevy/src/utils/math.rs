//! Shared mathematical utilities used across multiple domains.
//!
//! This module contains only truly cross-cutting mathematical functions
//! that are used in multiple parts of the codebase.

use std::f32::consts::PI;

/// Clamp a value to a specified range
pub fn clamp_to_range(value: f32, min: f32, max: f32) -> f32 {
    value.clamp(min, max)
}

/// Normalize an angle to the range [0, 2π)
pub fn normalize_angle(angle: f32) -> f32 {
    let two_pi = 2.0 * PI;
    ((angle % two_pi) + two_pi) % two_pi
}

/// Linear interpolation between two values
pub fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start + (end - start) * t
}

/// Move a value towards a target by at most max_delta
pub fn move_toward(current: f32, target: f32, max_delta: f32) -> f32 {
    let diff = target - current;
    if diff.abs() <= max_delta {
        target
    } else {
        current + diff.signum() * max_delta
    }
}

/// Check if a float value is reasonable (finite and not NaN)
pub fn is_reasonable_float(value: f32) -> bool {
    value.is_finite()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::f32::consts::PI;

    #[test]
    fn test_clamp_to_range() {
        assert_eq!(clamp_to_range(-5.0, 0.0, 10.0), 0.0);
        assert_eq!(clamp_to_range(15.0, 0.0, 10.0), 10.0);
        assert_eq!(clamp_to_range(5.0, 0.0, 10.0), 5.0);
    }

    #[test]
    fn test_normalize_angle() {
        assert!((normalize_angle(3.0 * PI) - PI).abs() < 1e-6);
        assert!((normalize_angle(-PI / 2.0) - (3.0 * PI / 2.0)).abs() < 1e-6);
        assert!((normalize_angle(PI / 4.0) - PI / 4.0).abs() < 1e-6);
    }

    #[test]
    fn test_lerp() {
        assert_eq!(lerp(0.0, 10.0, 0.0), 0.0);
        assert_eq!(lerp(0.0, 10.0, 1.0), 10.0);
        assert_eq!(lerp(0.0, 10.0, 0.5), 5.0);
        assert_eq!(lerp(5.0, 15.0, 0.5), 10.0);
    }

    #[test]
    fn test_move_toward() {
        assert_eq!(move_toward(0.0, 10.0, 5.0), 5.0);
        assert_eq!(move_toward(8.0, 10.0, 5.0), 10.0); // Should reach target
        assert_eq!(move_toward(10.0, 0.0, 3.0), 7.0); // Moving backward
        assert_eq!(move_toward(5.0, 5.0, 1.0), 5.0); // Already at target
    }

    #[test]
    fn test_is_reasonable_float() {
        assert!(is_reasonable_float(5.0));
        assert!(is_reasonable_float(-5.0));
        assert!(is_reasonable_float(0.0));
        assert!(!is_reasonable_float(f32::NAN));
        assert!(!is_reasonable_float(f32::INFINITY));
        assert!(!is_reasonable_float(f32::NEG_INFINITY));
    }
}
