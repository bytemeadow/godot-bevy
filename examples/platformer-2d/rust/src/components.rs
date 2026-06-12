//! Shared components for the platformer game
//!
//! This module defines reusable components that can be used across different
//! entity types (players, enemies, etc.) to avoid duplication when using
//! the `GodotNode` macro.

use crate::level_manager::LevelId;
use bevy::prelude::*;
use godot::classes::ProjectSettings;
use godot::obj::Singleton;
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

/// Component marking an entity as the player. Also defines the `Player2D` Godot
/// node class with exported `speed`, `jump_velocity`, and `gravity` fields, each
/// inserted as a companion component when the node enters the scene tree.
#[derive(Component, GodotNode, Debug, Clone, Default, Reflect)]
#[reflect(Component)]
#[godot_node(base(CharacterBody2D), class_name(Player2D))]
#[godot_components(
    speed(Speed, export_type(f32), default(250.0)),
    jump_velocity(JumpVelocity, export_type(f32), default(-400.0)),
    gravity(Gravity, export_type(f32), default(ProjectSettings::singleton()
        .get_setting("physics/2d/default_gravity")
        .try_to::<f32>()
        .unwrap_or(980.0))),
)]
pub struct Player;

/// Component marking an entity as a gem
#[derive(Component, GodotNode, Default, Debug, Clone)]
#[godot_node(base(Area2D), class_name(Gem2D))]
pub struct Gem;

/// Component marking an entity as a door
#[derive(Component, GodotNode, Default, Debug, Clone)]
#[godot_node(base(Area2D), class_name(Door2D))]
pub struct Door {
    #[godot_export(default(LevelId::Level1))]
    pub level_id: LevelId,
}
