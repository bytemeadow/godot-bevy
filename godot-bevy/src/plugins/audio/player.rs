//! Audio player types and Godot integration

use bevy_math::{Vec2, Vec3};

/// Type of Godot AudioStreamPlayer to use
#[derive(Debug, Clone)]
pub enum AudioPlayerType {
    /// Non-positional audio (AudioStreamPlayer)
    NonPositional,
    /// 2D positional audio (AudioStreamPlayer2D)
    Spatial2D { position: Vec2 },
    /// 3D positional audio (AudioStreamPlayer3D)
    Spatial3D { position: Vec3 },
}
