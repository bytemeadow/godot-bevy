//! Shared components for the platformer game
//!
//! This module defines reusable components that can be used across different
//! entity types (players, enemies, etc.) to avoid duplication when using
//! the BevyComponent macro.

use crate::level_manager::LevelId;
use bevy::prelude::*;
use godot_bevy::prelude::GodotNode;

/// Component representing movement speed in pixels per second
#[derive(Component, Debug, Clone, PartialEq, Reflect)]
#[reflect(Component)]
pub struct Speed(pub f32);

impl Default for Speed {
    fn default() -> Self {
        Self(100.0)
    }
}

/// Component representing jump velocity (negative for upward movement in Godot)
#[derive(Component, Debug, Clone, PartialEq, Reflect)]
#[reflect(Component)]
pub struct JumpVelocity(pub f32);

impl Default for JumpVelocity {
    fn default() -> Self {
        Self(-400.0)
    }
}

/// Component representing gravity in pixels per second squared
#[derive(Component, Debug, Clone, PartialEq, Reflect)]
#[reflect(Component)]
pub struct Gravity(pub f32);

impl Default for Gravity {
    fn default() -> Self {
        Self(980.0)
    }
}

/// Player marker + the `Player2D` Godot node with exported speed/jump/gravity,
/// each inserted as a companion when the node enters the tree.
#[derive(Component, GodotNode, Default, Debug, Clone, Reflect)]
#[reflect(Component)]
#[gdbevy(base = CharacterBody2D, class_name = Player2D)]
#[gdbevy(
    require(speed: Speed, as = f32, default = 250.0),
    require(jump_velocity: JumpVelocity, as = f32, default = -400.0),
    require(gravity: Gravity, as = f32, default = 980.0),
)]
pub struct Player;

/// Component marking an entity as a gem
#[derive(Component, GodotNode, Default, Debug, Clone)]
#[gdbevy(base = Area2D, class_name = Gem2D)]
pub struct Gem;

/// Component marking an entity as a door
#[derive(Component, GodotNode, Default, Debug, Clone)]
#[gdbevy(base = Area2D, class_name = Door2D)]
pub struct Door {
    #[gdbevy(export, default = LevelId::Level1)]
    pub level_id: LevelId,
}
