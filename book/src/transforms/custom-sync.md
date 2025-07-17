# Custom Transform Sync

For performance-critical applications, you can create custom transform sync systems that only synchronize specific entities. This uses compile-time queries for maximum performance.

## When to Use Custom Sync

Use custom transform sync when:
- You have many entities but only some need synchronization
- Performance is critical and you want to minimize overhead
- You need fine-grained control over which entities sync

## Basic Usage

### 1. Disable Default Plugin (if using GodotDefaultPlugins)

If you're using `GodotDefaultPlugins`, you'll want to disable the default transform sync plugin:

```rust
use godot_bevy::plugins::{GodotDefaultPlugins, transforms::GodotTransformSyncPlugin};

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotDefaultPlugins.build().disable::<GodotTransformSyncPlugin>());
}
```

### 2. Define Custom Systems

Use the `add_transform_sync_systems_2d!` or `add_transform_sync_systems_3d!` macros to define which entities should sync:

```rust
use godot_bevy::plugins::transforms::{add_transform_sync_systems_2d, add_transform_sync_systems_3d};
use godot_bevy::plugins::{GodotDefaultPlugins, transforms::GodotTransformSyncPlugin};
use godot_bevy::interop::node_markers::*;
use bevy::ecs::query::{Or, With};

#[bevy_app]
fn build_app(app: &mut App) {
    // Disable default transform sync plugin
    app.add_plugins(GodotDefaultPlugins.build().disable::<GodotTransformSyncPlugin>());

    // Only sync 2D physics bodies
    add_transform_sync_systems_2d! {
        app,
        PhysicsOnly2D = Or<(
            With<RigidBody2DMarker>,
            With<CharacterBody2DMarker>,
            With<StaticBody2DMarker>,
        )>
    }

    // Only sync 3D physics bodies
    add_transform_sync_systems_3d! {
        app,
        PhysicsOnly3D = Or<(
            With<RigidBody3DMarker>,
            With<CharacterBody3DMarker>,
            With<StaticBody3DMarker>,
        )>
    }
}
```

## Advanced Usage

### Directional Sync Control

You can specify which direction of synchronization you need for optimal performance:

```rust
// 2D systems
add_transform_sync_systems_2d! {
    app,
    // Only ECS → Godot (one-way sync)
    UIElements = bevy_to_godot: With<UIElement>,

    // Only Godot → ECS (useful for reading physics results)
    PhysicsResults = godot_to_bevy: With<PhysicsActor>,

    // Full bidirectional sync
    Player = With<Player>,
}

// 3D systems
add_transform_sync_systems_3d! {
    app,
    // Only ECS → Godot (one-way sync)
    VisualEffects = bevy_to_godot: With<VisualEffect>,

    // Only Godot → ECS (useful for reading physics results)
    PhysicsResults3D = godot_to_bevy: With<PhysicsActor3D>,

    // Full bidirectional sync
    Player3D = With<Player3D>,
}
```

This provides significant performance benefits:
- **`bevy_to_godot` only**: Skips reading Godot transforms, ideal for UI elements
- **`godot_to_bevy` only**: Skips writing to Godot, useful for reading physics results
- **Both directions** (no prefix): Full synchronization when needed

### Multiple Sync Systems

You can define multiple sync systems for different entity types:

```rust
// 2D systems
add_transform_sync_systems_2d! {
    app,
    // 2D physics bodies
    PhysicsBody2D = Or<(
        With<RigidBody2DMarker>,
        With<CharacterBody2DMarker>,
        With<StaticBody2DMarker>,
    )>,

    // 2D UI elements (ECS-driven only)
    UIElements = bevy_to_godot: Or<(
        With<Button>,
        With<Label>,
    )>,
}

// 3D systems
add_transform_sync_systems_3d! {
    app,
    // 3D physics bodies
    PhysicsBody3D = Or<(
        With<RigidBody3DMarker>,
        With<CharacterBody3DMarker>,
        With<StaticBody3DMarker>,
    )>,

    // Visual elements (ECS-driven only)
    VisualOnly = bevy_to_godot: Or<(
        With<Sprite3DMarker>,
        With<MeshInstance3DMarker>,
    )>
}
```

### Custom Marker Components

For maximum control, create custom marker components:

```rust
use bevy::prelude::*;

#[derive(Component)]
struct NeedsTransformSync;

#[derive(Component)]
struct HighPrioritySync;

// 2D opt-in sync
add_transform_sync_systems_2d! {
    app,
    // Only entities explicitly marked for sync
    OptIn2D = With<NeedsTransformSync>,

    // High priority entities
    HighPriority2D = With<HighPrioritySync>,
}

// 3D opt-in sync
add_transform_sync_systems_3d! {
    app,
    // Only entities explicitly marked for sync
    OptIn3D = With<NeedsTransformSync>,

    // High priority entities
    HighPriority3D = With<HighPrioritySync>,
}

// In your spawning systems
fn spawn_entity(mut commands: Commands) {
    commands.spawn((
        RigidBody3DMarker,
        NeedsTransformSync,  // Only entities with this will sync
        // ... other components
    ));
}
```

### Selective System Registration

The macros automatically register systems in the appropriate schedules:
- `bevy_to_godot` systems run in the `Last` schedule
- `godot_to_bevy` systems run in the `PreUpdate` schedule
- Bidirectional sync (no prefix) runs in both schedules

This happens automatically when you use the macros, providing optimal performance without manual system management.

## Common Use Cases

### UI Elements (ECS → Godot only)

UI elements are typically driven by ECS systems and don't need to be read back:

```rust
add_transform_sync_systems_2d! {
    app,
    UIElements = bevy_to_godot: Or<(
        With<HealthBar>,
        With<MenuItem>,
        With<DialogBox>,
    )>
}
```

### Physics Results (Godot → ECS only)

When using Godot physics, you often only need to read the results:

```rust
add_transform_sync_systems_2d! {
    app,
    PhysicsActors2D = godot_to_bevy: Or<(
        With<RigidBody2DMarker>,
        With<CharacterBody2DMarker>,
    )>
}

add_transform_sync_systems_3d! {
    app,
    PhysicsActors3D = godot_to_bevy: Or<(
        With<RigidBody3DMarker>,
        With<CharacterBody3DMarker>,
    )>
}
```

### Interactive Elements (Bidirectional)

Player characters and interactive objects often need both directions:

```rust
add_transform_sync_systems_2d! {
    app,
    Interactive2D = With<Player>,
    NPCs2D = With<NPC>,
}

add_transform_sync_systems_3d! {
    app,
    Interactive3D = With<Player3D>,
    NPCs3D = With<NPC3D>,
}
```

## Best Practices

### 1. Start Simple

Begin with a single, broad filter and optimize as needed:

```rust
add_transform_sync_systems_2d! {
    app,
    GameEntities2D = Or<(With<Player>, With<Enemy>, With<Pickup>)>
}

add_transform_sync_systems_3d! {
    app,
    GameEntities3D = Or<(With<Player3D>, With<Enemy3D>, With<Pickup3D>)>
}
```

### 2. Use Descriptive Names

Choose clear names for your sync systems:

```rust
add_transform_sync_systems_2d! {
    app,
    MovingEntities2D = Or<(With<Player>, With<Enemy>)>,
    StaticProps2D = With<StaticProp>,
}

add_transform_sync_systems_3d! {
    app,
    MovingEntities3D = Or<(With<Player3D>, With<Enemy3D>)>,
    StaticProps3D = With<StaticProp3D>,
}
```

### 3. Avoid Over-Optimization

Don't create too many specialized systems unless profiling shows it's necessary:

```rust
// Good: Two logical groups
add_transform_sync_systems_2d! {
    app,
    GameEntities = Or<(With<Player>, With<Enemy>, With<Pickup>)>,
    UiElements = bevy_to_godot: Or<(With<Button>, With<Label>)>,
}

// Avoid: Too many micro-optimizations
add_transform_sync_systems_2d! {
    app,
    Players = With<Player>,
    Enemies = With<Enemy>,
    Pickups = With<Pickup>,
    Buttons = bevy_to_godot: With<Button>,
    Labels = bevy_to_godot: With<Label>,
    // ... too granular
}
```
