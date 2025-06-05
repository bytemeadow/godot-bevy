//! Shared components for the platformer game
//!
//! This module defines reusable components that can be used across different
//! entity types (players, enemies, etc.) to avoid duplication when using
//! the BevyComponent macro.

use bevy::prelude::*;

/// Component representing movement speed in pixels per second
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Speed(pub f32);

impl Default for Speed {
    fn default() -> Self {
        Self(100.0)
    }
}

/// Component representing jump velocity (negative for upward movement in Godot)
#[derive(Component, Debug, Clone, PartialEq)]
pub struct JumpVelocity(pub f32);

impl Default for JumpVelocity {
    fn default() -> Self {
        Self(-400.0)
    }
}

/// Component marking an entity as the player
#[derive(Component, Debug, Clone, Default)]
pub struct Player;

/// Component marking an entity as an enemy
#[derive(Component, Debug, Clone, Default)]
pub struct Enemy;
