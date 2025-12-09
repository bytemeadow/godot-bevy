//! Options for quaternion display and editing.

use super::InspectorOptionsType;

/// Options for how quaternions are displayed and edited.
#[derive(Clone, Default)]
#[non_exhaustive]
pub struct QuatOptions {
    /// How to display the quaternion.
    pub display: QuatDisplay,
}

/// How to display a quaternion value.
#[derive(Copy, Clone, Default, Debug, PartialEq, Eq)]
pub enum QuatDisplay {
    /// Display as raw x, y, z, w components.
    Raw,
    /// Display as Euler angles (pitch, yaw, roll in radians).
    #[default]
    Euler,
    /// Display as yaw, pitch, roll (in degrees, more intuitive).
    YawPitchRoll,
    /// Display as axis + angle.
    AxisAngle,
}

impl InspectorOptionsType for bevy_math::Quat {
    type DeriveOptions = QuatOptions;
    type Options = QuatOptions;

    fn options_from_derive(options: Self::DeriveOptions) -> Self::Options {
        options
    }
}
