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
include!("math_tests.rs");
