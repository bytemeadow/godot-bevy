# Querying with Node Type Markers

When godot-bevy discovers nodes in your Godot scene tree, it automatically creates ECS entities with `GodotNodeHandle` components to represent them. To enable efficient, type-safe querying, the library also adds **marker components** that indicate what type of Godot node each entity represents.

## Overview

Every entity that represents a Godot node gets marker components automatically:

```rust
use godot_bevy::prelude::*;

// Query all Sprite2D entities - no runtime type checking needed!
fn update_sprites(mut sprites: Query<&mut GodotNodeHandle, With<Sprite2DMarker>>) {
    for mut handle in sprites.iter_mut() {
        // We know this is a Sprite2D, so .get() is safe
        let sprite = handle.get::<Sprite2D>();
        // Work with the sprite...
    }
}
```

## Available Marker Components

### Base Node Types
- `NodeMarker` - All nodes (every entity gets this)
- `Node2DMarker` - All 2D nodes 
- `Node3DMarker` - All 3D nodes
- `ControlMarker` - UI control nodes
- `CanvasItemMarker` - Canvas items

### Visual Nodes
- `Sprite2DMarker` / `Sprite3DMarker`
- `AnimatedSprite2DMarker` / `AnimatedSprite3DMarker` 
- `MeshInstance2DMarker` / `MeshInstance3DMarker`

### Physics Bodies
- `RigidBody2DMarker` / `RigidBody3DMarker`
- `CharacterBody2DMarker` / `CharacterBody3DMarker`
- `StaticBody2DMarker` / `StaticBody3DMarker`

### Areas and Collision
- `Area2DMarker` / `Area3DMarker`
- `CollisionShape2DMarker` / `CollisionShape3DMarker`
- `CollisionPolygon2DMarker` / `CollisionPolygon3DMarker`

### Audio Players
- `AudioStreamPlayerMarker`
- `AudioStreamPlayer2DMarker` 
- `AudioStreamPlayer3DMarker`

### UI Elements
- `LabelMarker`
- `ButtonMarker`
- `LineEditMarker`
- `TextEditMarker`
- `PanelMarker`

### Cameras and Lighting
- `Camera2DMarker` / `Camera3DMarker`
- `DirectionalLight3DMarker`
- `SpotLight3DMarker`

### Animation and Timing
- `AnimationPlayerMarker`
- `AnimationTreeMarker`
- `TimerMarker`

### Path Nodes
- `Path2DMarker` / `Path3DMarker`
- `PathFollow2DMarker` / `PathFollow3DMarker`

## Hierarchical Markers

Node type markers follow Godot's inheritance hierarchy. For example, a `CharacterBody2D` entity will have:

- `NodeMarker` (all nodes inherit from Node)
- `Node2DMarker` (CharacterBody2D inherits from Node2D)
- `CharacterBody2DMarker` (the specific type)

This lets you query at any level of specificity:

```rust
// Query ALL nodes
fn system1(nodes: Query<&GodotNodeHandle, With<NodeMarker>>) { /* ... */ }

// Query all 2D nodes  
fn system2(nodes_2d: Query<&GodotNodeHandle, With<Node2DMarker>>) { /* ... */ }

// Query only CharacterBody2D nodes
fn system3(characters: Query<&GodotNodeHandle, With<CharacterBody2DMarker>>) { /* ... */ }
```

## Advanced Query Patterns

### Combining Markers

```rust
// Entities that have BOTH a Sprite2D AND a RigidBody2D
fn physics_sprites(
    query: Query<&mut GodotNodeHandle, (With<Sprite2DMarker>, With<RigidBody2DMarker>)>
) {
    for mut handle in query.iter_mut() {
        let sprite = handle.get::<Sprite2D>();
        let body = handle.get::<RigidBody2D>();
        // Work with both components...
    }
}
```

### Excluding Node Types

```rust
// All sprites EXCEPT character bodies (e.g., environmental sprites)
fn environment_sprites(
    query: Query<&mut GodotNodeHandle, (With<Sprite2DMarker>, Without<CharacterBody2DMarker>)>
) {
    for mut handle in query.iter_mut() {
        // These are sprites but not character bodies
        let sprite = handle.get::<Sprite2D>();
        // Work with environmental sprites...
    }
}
```

### Multiple Specific Types

```rust
// Handle different audio player types efficiently
fn update_audio_system(
    players_1d: Query<&mut GodotNodeHandle, With<AudioStreamPlayerMarker>>,
    players_2d: Query<&mut GodotNodeHandle, With<AudioStreamPlayer2DMarker>>,
    players_3d: Query<&mut GodotNodeHandle, With<AudioStreamPlayer3DMarker>>,
) {
    // Process each type separately - no runtime type checking!
    for mut handle in players_1d.iter_mut() {
        let player = handle.get::<AudioStreamPlayer>();
        // Handle 1D audio...
    }
    
    for mut handle in players_2d.iter_mut() {
        let player = handle.get::<AudioStreamPlayer2D>();
        // Handle 2D spatial audio...
    }
    
    for mut handle in players_3d.iter_mut() {
        let player = handle.get::<AudioStreamPlayer3D>();
        // Handle 3D spatial audio...
    }
}
```

## Performance Benefits

Node type markers provide significant performance improvements:

1. **Reduced Iteration**: Only process entities you care about
2. **No Runtime Type Checking**: Skip `try_get()` calls
3. **Better ECS Optimization**: Bevy can optimize queries with markers
4. **Cache Efficiency**: Process similar entities together

## Automatic Application

You don't need to add marker components manually. The library automatically:

1. Detects the Godot node type during scene tree traversal
2. Adds the appropriate marker component(s) to the entity
3. Includes all parent type markers in the inheritance hierarchy
4. Ensures every entity gets the base `NodeMarker`

This happens transparently when nodes are discovered in your scene tree, making the markers immediately available for your systems to use.

## Best Practices

- Use specific markers when you know the exact node type: `With<Sprite2DMarker>`
- Use hierarchy markers for broader categories: `With<Node2DMarker>` for all 2D nodes
- Combine markers to find entities with multiple components
- Prefer `.get()` over `.try_get()` when using markers - it's both faster and safer

For migration information from pre-0.7.0 versions, see the [Migration Guide](../migration/v0.6-to-v0.7.md).