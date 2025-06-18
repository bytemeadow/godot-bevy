use bevy::ecs::component::Component;
use crate::node_registry::NodeAccess;
use godot::classes::*;

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

// NodeAccess implementations for all built-in markers
// This enables typed node access through the NodeRegistry

impl NodeAccess for NodeMarker {
    type GodotType = Node;
}

impl NodeAccess for Node2DMarker {
    type GodotType = Node2D;
}

impl NodeAccess for Node3DMarker {
    type GodotType = Node3D;
}

impl NodeAccess for ControlMarker {
    type GodotType = Control;
}

impl NodeAccess for CanvasItemMarker {
    type GodotType = CanvasItem;
}

impl NodeAccess for Sprite2DMarker {
    type GodotType = Sprite2D;
}

impl NodeAccess for Sprite3DMarker {
    type GodotType = Sprite3D;
}

impl NodeAccess for MeshInstance2DMarker {
    type GodotType = MeshInstance2D;
}

impl NodeAccess for MeshInstance3DMarker {
    type GodotType = MeshInstance3D;
}

impl NodeAccess for AnimatedSprite2DMarker {
    type GodotType = AnimatedSprite2D;
}

impl NodeAccess for AnimatedSprite3DMarker {
    type GodotType = AnimatedSprite3D;
}

impl NodeAccess for RigidBody2DMarker {
    type GodotType = RigidBody2D;
}

impl NodeAccess for RigidBody3DMarker {
    type GodotType = RigidBody3D;
}

impl NodeAccess for CharacterBody2DMarker {
    type GodotType = CharacterBody2D;
}

impl NodeAccess for CharacterBody3DMarker {
    type GodotType = CharacterBody3D;
}

impl NodeAccess for StaticBody2DMarker {
    type GodotType = StaticBody2D;
}

impl NodeAccess for StaticBody3DMarker {
    type GodotType = StaticBody3D;
}

impl NodeAccess for Area2DMarker {
    type GodotType = Area2D;
}

impl NodeAccess for Area3DMarker {
    type GodotType = Area3D;
}

impl NodeAccess for CollisionShape2DMarker {
    type GodotType = CollisionShape2D;
}

impl NodeAccess for CollisionShape3DMarker {
    type GodotType = CollisionShape3D;
}

impl NodeAccess for CollisionPolygon2DMarker {
    type GodotType = CollisionPolygon2D;
}

impl NodeAccess for CollisionPolygon3DMarker {
    type GodotType = CollisionPolygon3D;
}

impl NodeAccess for AudioStreamPlayerMarker {
    type GodotType = AudioStreamPlayer;
}

impl NodeAccess for AudioStreamPlayer2DMarker {
    type GodotType = AudioStreamPlayer2D;
}

impl NodeAccess for AudioStreamPlayer3DMarker {
    type GodotType = AudioStreamPlayer3D;
}

impl NodeAccess for LabelMarker {
    type GodotType = Label;
}

impl NodeAccess for ButtonMarker {
    type GodotType = Button;
}

impl NodeAccess for LineEditMarker {
    type GodotType = LineEdit;
}

impl NodeAccess for TextEditMarker {
    type GodotType = TextEdit;
}

impl NodeAccess for PanelMarker {
    type GodotType = Panel;
}

impl NodeAccess for Camera2DMarker {
    type GodotType = Camera2D;
}

impl NodeAccess for Camera3DMarker {
    type GodotType = Camera3D;
}

impl NodeAccess for DirectionalLight3DMarker {
    type GodotType = DirectionalLight3D;
}

impl NodeAccess for SpotLight3DMarker {
    type GodotType = SpotLight3D;
}

impl NodeAccess for AnimationPlayerMarker {
    type GodotType = AnimationPlayer;
}

impl NodeAccess for AnimationTreeMarker {
    type GodotType = AnimationTree;
}

impl NodeAccess for TimerMarker {
    type GodotType = Timer;
}

impl NodeAccess for Path2DMarker {
    type GodotType = Path2D;
}

impl NodeAccess for Path3DMarker {
    type GodotType = Path3D;
}

impl NodeAccess for PathFollow2DMarker {
    type GodotType = PathFollow2D;
}

impl NodeAccess for PathFollow3DMarker {
    type GodotType = PathFollow3D;
}
