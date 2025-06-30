use bevy::ecs::component::Component;

/// Marker components for common Godot node types.
/// These enable type-safe ECS queries like: Query<&GodotNodeHandle, With<Sprite2DMarker>>

// Base node types
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Node2DMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Node3DMarker;

// Control nodes
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ControlMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CanvasItemMarker;

// Visual nodes
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sprite2DMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Sprite3DMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshInstance2DMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct MeshInstance3DMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimatedSprite2DMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimatedSprite3DMarker;

// Physics nodes
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RigidBody2DMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct RigidBody3DMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterBody2DMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CharacterBody3DMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticBody2DMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct StaticBody3DMarker;

// Area nodes
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Area2DMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Area3DMarker;

// Collision nodes
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionShape2DMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionShape3DMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionPolygon2DMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct CollisionPolygon3DMarker;

// Audio nodes
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioStreamPlayerMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioStreamPlayer2DMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AudioStreamPlayer3DMarker;

// UI nodes
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LabelMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct ButtonMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct LineEditMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TextEditMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PanelMarker;

// Camera nodes
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Camera2DMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Camera3DMarker;

// Light nodes
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct DirectionalLight3DMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct SpotLight3DMarker;

// Animation nodes
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationPlayerMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct AnimationTreeMarker;

// Timer nodes
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct TimerMarker;

// Path nodes
#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Path2DMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct Path3DMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathFollow2DMarker;

#[derive(Component, Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathFollow3DMarker;
