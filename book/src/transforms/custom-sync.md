# Custom Transform Sync

For performance-critical applications, you can create custom transform sync systems that only synchronize specific entities. This uses compile-time queries for maximum performance.

## When to Use Custom Sync

Use custom transform sync when:
- You have many entities but only some need synchronization
- Performance is critical and you want to minimize overhead
- You need fine-grained control over which entities sync
- You're building a game with physics bodies mixed with UI elements

## Basic Usage

### 1. Add the Config Plugin

First, add the config-only plugin instead of the default transform sync plugin:

```rust
use godot_bevy::plugins::transforms::GodotCustomTransformSyncPlugin;

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotCustomTransformSyncPlugin::default());
}
```

### 2. Define Custom Systems

Use the `add_transform_sync_systems!` macro to define which entities should sync:

```rust
use godot_bevy::plugins::transforms::add_transform_sync_systems;
use godot_bevy::interop::node_markers::*;
use bevy::ecs::query::{Or, With};

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotCustomTransformSyncPlugin::default());
    
    // Only sync physics bodies
    add_transform_sync_systems! {
        app,
        PhysicsOnly = Or<(
            With<RigidBody3DMarker>,
            With<CharacterBody3DMarker>,
            With<StaticBody3DMarker>,
        )>
    }
}
```

## Advanced Usage

### Multiple Sync Systems

You can define multiple sync systems for different entity types:

```rust
add_transform_sync_systems! {
    app,
    // 3D physics bodies
    PhysicsBody3D: 3d = Or<(
        With<RigidBody3DMarker>,
        With<CharacterBody3DMarker>,
        With<StaticBody3DMarker>,
    )>,
    
    // 2D physics bodies
    PhysicsBody2D: 2d = Or<(
        With<RigidBody2DMarker>,
        With<CharacterBody2DMarker>,
        With<StaticBody2DMarker>,
    )>,
    
    // Visual elements
    VisualOnly: 3d = Or<(
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

add_transform_sync_systems! {
    app,
    // Only entities explicitly marked for sync
    OptIn = With<NeedsTransformSync>,
    
    // High priority entities
    HighPriority = With<HighPrioritySync>,
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

## Performance Optimization

### One-Way Sync Only

If you don't need bidirectional sync, you can optimize further by manually adding only the systems you need:

```rust
use godot_bevy::plugins::transforms::transform_sync_systems;

// Generate systems but don't add them automatically
transform_sync_systems! {
    PhysicsOnly = Or<(
        With<RigidBody3DMarker>,
        With<CharacterBody3DMarker>,
    )>
}

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotCustomTransformSyncPlugin::default());
    
    // Only add the ECS → Godot system
    app.add_systems(Last, post_update_godot_transforms_3d_physicsonly);
    // Skip the Godot → ECS system for one-way sync
}
```

### Conditional Sync

You can add conditions to your sync systems:

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotCustomTransformSyncPlugin::default());
    
    add_transform_sync_systems! {
        app,
        PhysicsOnly = Or<(With<RigidBody3DMarker>, With<CharacterBody3DMarker>)>
    }
    
    // Add conditions
    app.add_systems(
        Last,
        post_update_godot_transforms_3d_physicsonly.run_if(in_state(GameState::Playing))
    );
}
```

## Configuration

Custom sync systems use `GodotCustomTransformSyncConfig` for configuration:

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    // Configure the custom sync behavior
    app.add_plugins(GodotCustomTransformSyncPlugin {
        sync_mode: TransformSyncMode::TwoWay,
    });
    
    add_transform_sync_systems! {
        app,
        PhysicsOnly = Or<(With<RigidBody3DMarker>, With<CharacterBody3DMarker>)>
    }
}

// Runtime configuration
fn enable_two_way_sync(mut commands: Commands) {
    commands.insert_resource(GodotCustomTransformSyncConfig::two_way());
}
```

## Generated System Names

The macro generates predictable system names based on your identifiers:

```rust
transform_sync_systems! {
    PhysicsBody3D = Or<(With<RigidBody3DMarker>, With<CharacterBody3DMarker>)>
}
```

Generates:
- `pre_update_godot_transforms_3d_physicsbody3d`
- `post_update_godot_transforms_3d_physicsbody3d`

You can reference these systems for ordering or conditions:

```rust
app.add_systems(
    Last,
    (
        my_physics_system,
        post_update_godot_transforms_3d_physicsbody3d,
    ).chain()
);
```

## Best Practices

### 1. Start Simple

Begin with a single, broad filter and optimize as needed:

```rust
add_transform_sync_systems! {
    app,
    GameEntities = Or<(With<Player>, With<Enemy>, With<Pickup>)>
}
```

### 2. Use Descriptive Names

Choose clear names for your sync systems:

```rust
add_transform_sync_systems! {
    app,
    MovingEntities = Or<(With<Player>, With<Enemy>)>,
    StaticProps = With<StaticProp>,
}
```

### 3. Document Your Filters

Add comments explaining your sync strategy:

```rust
add_transform_sync_systems! {
    app,
    // Entities that move and need frequent sync
    DynamicEntities = Or<(With<Player>, With<Enemy>)>,
    
    // UI elements that occasionally change
    UiElements = Or<(With<HealthBar>, With<MenuButton>)>,
}
```

### 4. Avoid Over-Optimization

Don't create too many specialized systems unless profiling shows it's necessary:

```rust
// Good: Two logical groups
add_transform_sync_systems! {
    app,
    GameEntities = Or<(With<Player>, With<Enemy>, With<Pickup>)>,
    UiElements = Or<(With<Button>, With<Label>)>,
}

// Avoid: Too many micro-optimizations
add_transform_sync_systems! {
    app,
    Players = With<Player>,
    Enemies = With<Enemy>,
    Pickups = With<Pickup>,
    Buttons = With<Button>,
    Labels = With<Label>,
    // ... too granular
}
```

## Migration from Default Sync

If you're migrating from the default sync plugin:

### Before

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotDefaultTransformSyncPlugin::default());
}
```

### After

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotCustomTransformSyncPlugin::default());
    
    // Equivalent to default behavior
    add_transform_sync_systems! {
        app,
        AllNodes: 2d = With<Node2DMarker>,
        AllNodes: 3d = With<Node3DMarker>,
    }
}
```

### Performance-Focused Migration

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotCustomTransformSyncPlugin::default());
    
    // Only sync what you actually need
    add_transform_sync_systems! {
        app,
        PhysicsOnly = Or<(
            With<RigidBody3DMarker>,
            With<CharacterBody3DMarker>,
        )>
    }
}
```

## Troubleshooting

### "My entities aren't syncing"

1. Check that you're using `GodotCustomTransformSyncPlugin`, not `GodotDefaultTransformSyncPlugin`
2. Verify your query matches the entities you expect
3. Ensure the entities have the required components AND the transform components

### "Performance is worse than default"

1. You might have too many sync systems - consolidate them
2. Check that your queries are specific enough
3. Consider using one-way sync instead of bidirectional

### "Systems not found"

The macro generates systems with lowercase names. `PhysicsBody3D` becomes `physicsbody3d`:

```rust
// Generated name
app.add_systems(Last, post_update_godot_transforms_3d_physicsbody3d);
```

## Performance Comparison

| Approach | Entities Synced | Query Overhead | Memory Usage |
|----------|----------------|----------------|--------------|
| Default Plugin | ALL Node2D/Node3D | Minimal | High |
| Custom: All Physics | Only physics bodies | Minimal | Medium |
| Custom: Opt-in Marker | Only marked entities | Minimal | Low |

Choose the approach that best fits your performance requirements and entity distribution.