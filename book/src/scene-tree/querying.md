# Querying with Node Type Markers and NodeRegistry

When godot-bevy discovers nodes in your Godot scene tree, it automatically creates ECS entities with `GodotNodeHandle` components to represent them. To enable efficient, type-safe querying, the library also adds **marker components** that indicate what type of Godot node each entity represents.

## NodeRegistry Access Pattern

You can use the **NodeRegistry** system to access Godot nodes through marker components:

```rust
use godot_bevy::prelude::*;

// Query entities with markers, access nodes via registry
fn update_sprites(
    sprites: Query<Entity, (With<Sprite2DMarker>, With<GodotNodeHandle>)>,
    registry: NodeRegistryAccess,
) {
    for entity in sprites.iter() {
        // Direct access - panics if entity not found or wrong type
        let mut sprite = registry.access::<Sprite2D>(entity);
        // Work with the sprite...
    }
}
```

> **Important**: Always include `With<GodotNodeHandle>` when using `NodeRegistryAccess` to ensure entities are ready for node access otherwise sometimes the registry will fail to produce a node handle since it hasn't been created yet. See our [timing doc](./timing.md#dynamic-scene-spawning-timing) for more details.

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
fn system1(
    nodes: Query<Entity, (With<NodeMarker>, With<GodotNodeHandle>)>,
    registry: NodeRegistryAccess,
) { /* ... */ }

// Query all 2D nodes
fn system2(
    nodes_2d: Query<Entity, (With<Node2DMarker>, With<GodotNodeHandle>)>,
    registry: NodeRegistryAccess,
) { /* ... */ }

// Query only CharacterBody2D nodes
fn system3(
    characters: Query<Entity, (With<CharacterBody2DMarker>, With<GodotNodeHandle>)>,
    registry: NodeRegistryAccess,
) { /* ... */ }
```

## Advanced Query Patterns

### Combining Markers

```rust
// Entities that have BOTH a Sprite2D AND a RigidBody2D
fn physics_sprites(
    query: Query<Entity, (With<Sprite2DMarker>, With<RigidBody2DMarker>, With<GodotNodeHandle>)>,
    registry: NodeRegistryAccess,
) {
    for entity in query.iter() {
        let sprite = registry.access::<Sprite2D>(entity);
        let body = registry.access::<RigidBody2D>(entity);
        // Work with both components...
    }
}
```

### Excluding Node Types

```rust
// All sprites EXCEPT character bodies (e.g., environmental sprites)
fn environment_sprites(
    query: Query<Entity, (With<Sprite2DMarker>, Without<CharacterBody2DMarker>, With<GodotNodeHandle>)>,
    registry: NodeRegistryAccess,
) {
    for entity in query.iter() {
        // These are sprites but not character bodies
        let sprite = registry.access::<Sprite2D>(entity);
        // Work with environmental sprites...
    }
}
```

### Multiple Node Types in One System

```rust
// Handle different audio player types efficiently
fn update_audio_system(
    players_1d: Query<Entity, (With<AudioStreamPlayerMarker>, With<GodotNodeHandle>)>,
    players_2d: Query<Entity, (With<AudioStreamPlayer2DMarker>, With<GodotNodeHandle>)>,
    players_3d: Query<Entity, (With<AudioStreamPlayer3DMarker>, With<GodotNodeHandle>)>,
    registry: NodeRegistryAccess,
) {
    // Process each type separately
    for entity in players_1d.iter() {
        let player = registry.access::<AudioStreamPlayer>(entity);
        // Handle 1D audio...
    }

    for entity in players_2d.iter() {
        let player = registry.access::<AudioStreamPlayer2D>(entity);
        // Handle 2D spatial audio...
    }

    for entity in players_3d.iter() {
        let player = registry.access::<AudioStreamPlayer3D>(entity);
        // Handle 3D spatial audio...
    }
}
```

### Safe vs Panicking Access

The NodeRegistry provides two access methods:

```rust
fn flexible_access(
    sprites: Query<Entity, (With<Sprite2DMarker>, With<GodotNodeHandle>)>,
    registry: NodeRegistryAccess,
) {
    for entity in sprites.iter() {
        // Option-based access (safe, returns None if node doesn't exist or wrong type)
        if let Some(sprite) = registry.try_access::<Sprite2D>(entity) {
            // Handle sprite...
        }

        // Panicking access (use when you're certain the entity exists in registry and is correct type)
        let sprite = registry.access::<Sprite2D>(entity);
        // Will panic if entity not found in registry or node can't be cast to Sprite2D
    }
}
```

## Performance Benefits

The NodeRegistry pattern with marker components provides performance benefits:

1. **Reduced Iteration**: Only process entities you care about
2. **No Runtime Type Checking**: Skip `try_get()` calls when using `registry.access()`
3. **Better ECS Optimization**: Bevy can optimize queries with markers
4. **Cache Efficiency**: Process similar entities together

## Automatic Application

You don't need to add marker components manually. The library automatically:

1. Detects the Godot node type during scene tree traversal
2. Adds the appropriate marker component(s) to the entity
3. Includes all parent type markers in the inheritance hierarchy
4. Ensures every entity gets the base `NodeMarker`
5. Registers entities in the NodeRegistry for fast access

This happens transparently when nodes are discovered in your scene tree, making the markers immediately available for your systems to use.

## Best Practices

- **Always include `With<GodotNodeHandle>`** when using `NodeRegistryAccess` to ensure entities are ready
- Use specific markers when you know the exact node type: `With<Sprite2DMarker>`
- Use hierarchy markers for broader categories: `With<Node2DMarker>` for all 2D nodes
- Combine markers to find entities with multiple components
- Use `registry.access()` when you're confident the node exists (panics on failure)
- Use `registry.try_access()` when the node might not exist (returns `Option`)

## Alternative Access Patterns

You can also access Godot nodes directly through `GodotNodeHandle` queries:

```rust
fn update_sprites_direct(
    mut sprites: Query<&mut GodotNodeHandle, With<Sprite2DMarker>>,
) {
    for mut handle in sprites.iter_mut() {
        let sprite = handle.get::<Sprite2D>();
        // Work with sprite...
    }
}
```

Or combine multiple node types using the NodeRegistry approach:

```rust
fn update_multiple_types(
    sprites: Query<Entity, (With<Sprite2DMarker>, With<GodotNodeHandle>)>,
    buttons: Query<Entity, (With<ButtonMarker>, With<GodotNodeHandle>)>,
    registry: NodeRegistryAccess,
) {
    for entity in sprites.iter() {
        let sprite = registry.access::<Sprite2D>(entity);
        // Work with sprite...
    }
    for entity in buttons.iter() {
        let button = registry.access::<Button>(entity);
        // Work with button...
    }
}
```

For migration information from pre-0.7.0 versions to 0.7.0, see the [Migration Guide](../migration/v0.6-to-v0.7.md).
