# Transform System Overview

The transform system is one of the most important aspects of godot-bevy, handling position, rotation, and scale synchronization between Bevy ECS and Godot nodes.

## Three Approaches to Movement

godot-bevy supports three distinct approaches for handling transforms and movement:

### 1. ECS Transform Components

Use `Transform2D`/`Transform3D` components with automatic syncing between ECS and Godot. This is the default approach. You update transforms in ECS, and we take care of syncing the transforms to the Godot side at the end of each frame.

```rust
use godot_bevy::prelude::*;

fn move_entity(mut query: Query<&mut Transform2D>) {
    for mut transform in query.iter_mut() {
        transform.as_bevy_mut().translation.x += 1.0;
    }
}
```

### 2. Direct Godot Physics

Use `GodotNodeHandle` to directly control Godot physics nodes. Perfect for physics-heavy games. This usually means you're calling Godot's move methods to have it handle physics for you.

```rust
fn move_character(mut query: Query<&mut GodotNodeHandle>) {
    for mut handle in query.iter_mut() {
        let mut body = handle.get::<CharacterBody2D>();
        body.set_velocity(Vector2::new(100.0, 0.0));
        body.move_and_slide();
    }
}
```

### 3. Hybrid Approach

Allows for modifying transforms both from Godot's side and from ECS side. Useful during migration from a GDScript project to godot-bevy or when you're using Godot's physics methods but still want transforms to be updated for reading on the ECS side.

## Default Behavior

By default, godot-bevy operates in **one-way sync mode**:

- ✅ **Writing enabled**: Changes to ECS transform components update Godot nodes
- ❌ **Reading disabled**: Changes to Godot nodes don't update ECS components

This is optimal for pure ECS applications where all movement logic lives in Bevy systems.

## When to Use Each Approach

### Use ECS Transforms When:
- Building a pure ECS game
- Movement logic is simple (no complex physics)
- You want clean separation between logic and presentation
- Performance of transform sync is acceptable

### Use Direct Godot Physics When:
- Building platformers or physics-heavy games
- You need Godot's collision detection features
- Using CharacterBody2D/3D or RigidBody2D/3D
- You want zero transform sync overhead

### Use Hybrid Approach When:
- Migrating an existing Godot project to ECS
- Some systems need ECS transforms, others need physics
- Gradually transitioning from GDScript to Rust

## Key Concepts

### Transform Components

godot-bevy provides two transform components that maintain both Bevy and Godot representations:

- **`Transform2D`** - For 2D games
- **`Transform3D`** - For 3D games

These components automatically keep Bevy and Godot transforms in sync based on your configuration.

### Sync Modes

The transform system supports three synchronization modes:

1. **Disabled** - No syncing, no transform components created
2. **OneWay** - ECS → Godot only (default)
3. **TwoWay** - ECS ↔ Godot bidirectional sync

### Performance Considerations

Each approach has different performance characteristics:

- **ECS Transforms**: Small overhead from syncing
- **Direct Physics**: Zero sync overhead
- **Hybrid**: Depends on usage pattern

## Next Steps

- Learn about [Sync Modes](./sync-modes.md) in detail
