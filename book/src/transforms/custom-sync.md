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

### Directional Sync Control

You can specify which direction of synchronization you need for optimal performance:

```rust
add_transform_sync_systems! {
    app,
    // Only ECS → Godot (one-way sync)
    UIElements = 2d: bevy_to_godot: With<UIElement>,
    
    // Only Godot → ECS (useful for reading physics results)
    PhysicsResults = 3d: godot_to_bevy: With<PhysicsActor>,
    
    // Full bidirectional sync
    Player = 2d: bevy_to_godot: With<Player>, godot_to_bevy: With<Player>,
}
```

This provides significant performance benefits:
- **`bevy_to_godot` only**: Skips reading Godot transforms, ideal for UI elements
- **`godot_to_bevy` only**: Skips writing to Godot, useful for reading physics results  
- **Both directions**: Full synchronization when needed

### Multiple Sync Systems

You can define multiple sync systems for different entity types:

```rust
add_transform_sync_systems! {
    app,
    // 3D physics bodies
    PhysicsBody3D = 3d: Or<(
        With<RigidBody3DMarker>,
        With<CharacterBody3DMarker>,
        With<StaticBody3DMarker>,
    )>,
    
    // 2D physics bodies
    PhysicsBody2D = 2d: Or<(
        With<RigidBody2DMarker>,
        With<CharacterBody2DMarker>,
        With<StaticBody2DMarker>,
    )>,
    
    // Visual elements (ECS-driven only)
    VisualOnly = 3d: bevy_to_godot: Or<(
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

### Directional Optimization

The most efficient approach is to specify exactly which direction of sync you need:

```rust
add_transform_sync_systems! {
    app,
    // UI elements only need ECS → Godot
    UIElements = 2d: bevy_to_godot: With<UIElement>,
    
    // Physics bodies only need Godot → ECS for reading results
    PhysicsResults = 3d: godot_to_bevy: With<PhysicsActor>,
    
    // Player needs both directions
    Player = 2d: bevy_to_godot: With<Player>, godot_to_bevy: With<Player>,
}
```

### Manual System Control

For maximum control, you can generate systems manually and add only what you need:

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

## Common Use Cases

### UI Elements (ECS → Godot only)

UI elements are typically driven by ECS systems and don't need to be read back:

```rust
add_transform_sync_systems! {
    app,
    UIElements = 2d: bevy_to_godot: Or<(
        With<HealthBar>,
        With<MenuItem>,
        With<DialogBox>,
    )>
}
```

### Physics Results (Godot → ECS only)

When using Godot physics, you often only need to read the results:

```rust
add_transform_sync_systems! {
    app,
    PhysicsActors = 3d: godot_to_bevy: Or<(
        With<RigidBody3DMarker>,
        With<CharacterBody3DMarker>,
    )>
}
```

### Interactive Elements (Bidirectional)

Player characters and interactive objects often need both directions:

```rust
add_transform_sync_systems! {
    app,
    Interactive = 2d: bevy_to_godot: With<Player>, godot_to_bevy: With<Player>,
    NPCs = 2d: bevy_to_godot: With<NPC>, godot_to_bevy: With<NPC>,
}
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
4. Check that you've specified the correct direction (`bevy_to_godot` or `godot_to_bevy`)

### "Performance is worse than default"

1. You might have too many sync systems - consolidate them
2. Check that your queries are specific enough
3. Consider using directional sync instead of bidirectional
4. Use `bevy_to_godot` only for UI elements and `godot_to_bevy` only for physics results

### "Systems not found"

The macro generates systems with lowercase names. `PhysicsBody3D` becomes `physicsbody3d`:

```rust
// Generated name
app.add_systems(Last, post_update_godot_transforms_3d_physicsbody3d);
```

### "Directional sync not working"

1. Verify you're using the correct syntax: `2d: bevy_to_godot: With<Component>`
2. Check that the generated systems are being added to the correct schedules
3. Ensure you're not overriding the sync direction in your config

## Performance Comparison

| Approach | Entities Synced | Direction | Query Overhead | Memory Usage |
|----------|----------------|-----------|----------------|--------------|
| Default Plugin | ALL Node2D/Node3D | Bidirectional | Minimal | High |
| Custom: All Physics | Only physics bodies | Bidirectional | Minimal | Medium |
| Custom: Opt-in Marker | Only marked entities | Bidirectional | Minimal | Low |
| Custom: ECS → Godot only | Only marked entities | One-way | Minimal | Very Low |
| Custom: Godot → ECS only | Only marked entities | One-way | Minimal | Very Low |

**Directional Performance Benefits:**
- **`bevy_to_godot` only**: ~50% fewer systems, no PreUpdate overhead
- **`godot_to_bevy` only**: ~50% fewer systems, no Last schedule overhead  
- **Both directions**: Full functionality with targeted entities

Choose the approach that best fits your performance requirements and entity distribution.