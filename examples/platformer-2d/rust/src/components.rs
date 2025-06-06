//! Shared components for the platformer game
//!
//! This module defines reusable components that can be used across different
//! entity types (players, enemies, etc.) to avoid duplication when using
//! the BevyComponent macro.

use bevy::prelude::*;

use crate::level_manager::LevelId;

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

/// Component representing gravity in pixels per second squared
#[derive(Component, Debug, Clone, PartialEq)]
pub struct Gravity(pub f32);

impl Default for Gravity {
    fn default() -> Self {
        Self(980.0)
    }
}

/// Component marking an entity as the player
#[derive(Component, Debug, Clone, Default)]
pub struct Player;

/// Component marking an entity as an enemy
#[derive(Component, Debug, Clone, Default)]
pub struct Enemy;

/// Component marking an entity as a gem
#[derive(Component, Debug, Clone, Default)]
pub struct Gem;

/// Component marking an entity as a door
#[derive(Component, Debug, Clone)]
pub struct Door(pub LevelId);
