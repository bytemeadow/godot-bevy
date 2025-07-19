# Custom Transform Sync

For performance-critical applications, you can create custom transform sync systems that only synchronize specific entities. This uses compile-time queries for maximum performance and automatically handles both 2D and 3D nodes.

## When to Use Custom Sync

Use custom transform sync when:
- You have many entities but only some need synchronization
- Performance is critical and you want to minimize overhead
- You need fine-grained control over which entities sync
- Different entity types need different sync directions

## Basic Usage

### 1. Disable Auto Sync

#### Option A: When Adding the Plugin Manually

Use the `.without_auto_sync()` method to disable automatic transform syncing while keeping the Transform and TransformSyncMetadata components:

```rust
use godot_bevy::prelude::*;

#[bevy_app]
fn build_app(app: &mut App) {
    // Disable auto sync but keep transform components
    app.add_plugins(
        GodotTransformSyncPlugin::default()
            .without_auto_sync()
    );
}
```

#### Option B: When Using GodotDefaultPlugins

If you're using `GodotDefaultPlugins`, you need to disable the included `GodotTransformSyncPlugin` and add your own configured version:

```rust
use godot_bevy::prelude::*;

#[bevy_app]
fn build_app(app: &mut App) {
    // Remove the default transform sync plugin and add a custom one
    app.add_plugins(
        GodotDefaultPlugins
            .build()
            .disable::<GodotTransformSyncPlugin>()
    );

    // Add your custom-configured transform sync plugin
    app.add_plugins(
        GodotTransformSyncPlugin::default()
            .without_auto_sync()
    );
}
```

### 2. Define Custom Systems

Use the `add_transform_sync_systems!` macro to define which entities should sync:

```rust
use godot_bevy::add_transform_sync_systems;
use godot_bevy::interop::node_markers::*;
use bevy::ecs::query::{Or, With};

#[bevy_app]
fn build_app(app: &mut App) {
    // Disable auto sync
    app.add_plugins(
        GodotTransformSyncPlugin::default()
            .without_auto_sync()
    );

    // Sync all physics bodies (both 2D and 3D automatically)
    add_transform_sync_systems! {
        app,
        PhysicsEntities = Or<(
            With<RigidBody2DMarker>,
            With<CharacterBody2DMarker>,
            With<StaticBody2DMarker>,
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
add_transform_sync_systems! {
    app,
    // Only ECS → Godot (one-way sync)
    UIElements = bevy_to_godot: With<UIElement>,

    // Only Godot → ECS (useful for reading physics results)
    PhysicsResults = godot_to_bevy: With<PhysicsActor>,

    // Full bidirectional sync
    Player = With<Player>,
}
```

This provides significant performance benefits:
- **`bevy_to_godot` only**: Skips reading Godot transforms, ideal for UI elements and ECS-driven entities
- **`godot_to_bevy` only**: Skips writing to Godot, useful for reading physics results
- **Both directions** (no prefix): Full synchronization when needed

### Real Example: Boids Performance Optimization

From the boids performance test example:

```rust
use godot_bevy::{add_transform_sync_systems, prelude::*};

#[derive(Component)]
struct Boid {
    velocity: Vec2,
    // ... other fields
}

#[bevy_app]
fn build_app(app: &mut App) {
    // Disable auto sync since we want custom sync for performance
    app.add_plugins(
        GodotTransformSyncPlugin::default()
            .without_auto_sync()
    );

    // Add custom transform sync systems for Boid entities only
    // Only sync Bevy -> Godot since boids are driven by ECS movement systems
    add_transform_sync_systems! {
        app,
        Boid = bevy_to_godot: With<Boid>
    }

    // ... movement systems, etc.
}
```

### Multiple Sync Systems in One Call

You can define multiple sync systems with different directions in a single macro call:

```rust
add_transform_sync_systems! {
    app,
    // All physics bodies (bidirectional) - both 2D and 3D
    PhysicsBodies = Or<(
        With<RigidBody2DMarker>,
        With<CharacterBody2DMarker>,
        With<StaticBody2DMarker>,
        With<RigidBody3DMarker>,
        With<CharacterBody3DMarker>,
        With<StaticBody3DMarker>,
    )>,

    // UI elements (ECS-driven only) - both 2D and 3D
    UIElements = bevy_to_godot: Or<(
        With<ButtonMarker>,
        With<LabelMarker>,
        With<Sprite3DMarker>,
    )>,

    // Physics result readers (Godot-driven only) - both 2D and 3D
    PhysicsReaders = godot_to_bevy: With<PhysicsListener>,
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

#[derive(Component)]
struct ReadOnlyTransform;

// Opt-in sync systems
add_transform_sync_systems! {
    app,
    // Only entities explicitly marked for sync
    OptInEntities = With<NeedsTransformSync>,

    // High priority entities (bidirectional)
    HighPriorityEntities = With<HighPrioritySync>,

    // Read-only from Godot
    ReadOnlyEntities = godot_to_bevy: With<ReadOnlyTransform>,
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

## Key Features

### Built-in Change Detection

The custom sync systems automatically use `TransformSyncMetadata` to prevent infinite loops:

```rust
// The generated systems automatically include change detection
// No need to manually handle sync loops - it's built in!
add_transform_sync_systems! {
    app,
    Player = With<Player>,  // Safe bidirectional sync
}
```

### Compile-time Optimization

Each sync system targets only specific entities, avoiding unnecessary iteration:

```rust
// This creates separate optimized systems for each query
add_transform_sync_systems! {
    app,
    FastEntities = With<Player>,           // Only checks Player entities
    SlowEntities = With<DebugMarker>,      // Only checks DebugMarker entities
    PhysicsEntities = With<RigidBody2DMarker>, // Only checks physics entities
}
```

### Automatic System Registration

The macro automatically registers systems in the appropriate schedules:
- `bevy_to_godot` systems run in the `Last` schedule
- `godot_to_bevy` systems run in the `PreUpdate` schedule
- Bidirectional sync (no prefix) runs in both schedules

### 2D and 3D Support

The macro automatically handles both 2D and 3D nodes in the same system:
- Uses `AnyOf<(&Node2DMarker, &Node3DMarker)>` to query both types
- Runtime type detection chooses the appropriate transform conversion
- Single system per query instead of separate 2D/3D systems

## Common Use Cases

### UI Elements (ECS → Godot only)

UI elements are typically driven by ECS systems and don't need to be read back:

```rust
#[derive(Component)]
struct HealthBar;

#[derive(Component)]
struct MenuItem;

add_transform_sync_systems! {
    app,
    UIElements = bevy_to_godot: Or<(
        With<HealthBar>,
        With<MenuItem>,
        With<LabelMarker>,
    )>
}
```

### Physics Results (Godot → ECS only)

When using Godot physics, you often only need to read the results:

```rust
#[derive(Component)]
struct PhysicsActor;

add_transform_sync_systems! {
    app,
    PhysicsActors = godot_to_bevy: Or<(
        With<RigidBody2DMarker>,
        With<CharacterBody2DMarker>,
        With<RigidBody3DMarker>,
        With<CharacterBody3DMarker>,
        With<PhysicsActor>,
    )>
}
```

### Interactive Elements (Bidirectional)

Player characters and interactive objects often need both directions:

```rust
#[derive(Component)]
struct Player;

#[derive(Component)]
struct NPC;

add_transform_sync_systems! {
    app,
    Interactive = Or<(With<Player>, With<NPC>)>,
}
```

## Best Practices

### 1. Start Simple

Begin with a single, broad filter and optimize as needed:

```rust
#[derive(Component)]
struct GameEntity;

add_transform_sync_systems! {
    app,
    GameEntities = With<GameEntity>
}
```

### 2. Use Descriptive Names

Choose clear names for your sync systems:

```rust
add_transform_sync_systems! {
    app,
    MovingEntities = Or<(With<Player>, With<Enemy>)>,
    StaticUI = bevy_to_godot: With<StaticUIElement>,
}
```

### 3. Avoid Over-Optimization

Don't create too many specialized systems unless profiling shows it's necessary:

```rust
// Good: Logical groups
add_transform_sync_systems! {
    app,
    GameEntities = Or<(With<Player>, With<Enemy>, With<Pickup>)>,
    UiElements = bevy_to_godot: Or<(With<ButtonMarker>, With<LabelMarker>)>,
}

// Avoid: Too many micro-optimizations
add_transform_sync_systems! {
    app,
    Players = With<Player>,
    Enemies = With<Enemy>,
    Pickups = With<Pickup>,
    Buttons = bevy_to_godot: With<ButtonMarker>,
    Labels = bevy_to_godot: With<LabelMarker>,
    // ... too granular
}
```

### 4. Profile Performance

Use Bevy's diagnostic tools to measure the impact of your custom sync systems:

```rust
use bevy::diagnostic::{FrameTimeDiagnosticsPlugin, LogDiagnosticsPlugin};

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins((
        FrameTimeDiagnosticsPlugin,
        LogDiagnosticsPlugin::default(),
    ));

    // Your custom sync systems
    add_transform_sync_systems! {
        app,
        OptimizedEntities = With<Player>,
    }
}
```

## Syntax Reference

The macro supports three sync directions:

```rust
add_transform_sync_systems! {
    app,
    // Bidirectional sync (default)
    EntityName = With<Component>,

    // One-way: ECS → Godot only
    EntityName = bevy_to_godot: With<Component>,

    // One-way: Godot → ECS only
    EntityName = godot_to_bevy: With<Component>,
}
```

You can mix multiple directions in a single macro call, and use any Bevy query filter:

```rust
add_transform_sync_systems! {
    app,
    PhysicsBodies = Or<(With<CharacterBody2DMarker>, With<RigidBody2DMarker>)>,
    UIElements = bevy_to_godot: (With<UIElement>, Without<Disabled>),
    PlayerInputs = godot_to_bevy: With<PlayerInput>,
}
```