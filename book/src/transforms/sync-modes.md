# Transform Sync Modes

godot-bevy provides three transform synchronization modes to fit different use cases. Understanding these modes is crucial for optimal performance and correct behavior.

## Available Modes

### `TransformSyncMode::Disabled`

No transform syncing occurs and no transform components are created.

**Characteristics:**
- ✅ Zero performance overhead
- ✅ No memory usage for transform components  
- ✅ Best for physics-heavy games
- ❌ Cannot use Transform2D/Transform3D components

**Use when:**
- Building platformers with CharacterBody2D
- Using RigidBody physics exclusively
- You need maximum performance

### `TransformSyncMode::OneWay` (Default)

Synchronizes transforms from ECS to Godot only.

**Characteristics:**
- ✅ ECS components control Godot node positions
- ✅ Good performance (minimal overhead)
- ✅ Clean ECS architecture
- ❌ Godot changes don't reflect in ECS

**Use when:**
- Building pure ECS games
- All movement logic is in Bevy systems
- You don't need to read Godot transforms

### `TransformSyncMode::TwoWay`

Full bidirectional synchronization between ECS and Godot.

**Characteristics:**
- ✅ Changes in either system are reflected
- ✅ Works with Godot animations
- ✅ Supports hybrid architectures
- ❌ Higher performance overhead

**Use when:**
- Migrating from GDScript to ECS
- Using Godot's AnimationPlayer
- Mixing ECS and GDScript logic

## Configuration

Configure the sync mode based on which approach you're using:

### Default Transform Sync Plugin

Configure the plugin directly:

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    // Disabled mode - no transform syncing
    app.add_plugins(GodotDefaultTransformSyncPlugin {
        sync_mode: TransformSyncMode::Disabled,
    });
    
    // One-way mode (default)
    app.add_plugins(GodotDefaultTransformSyncPlugin::default());
    
    // Two-way mode
    app.add_plugins(GodotDefaultTransformSyncPlugin {
        sync_mode: TransformSyncMode::TwoWay,
    });
}
```

### Custom Transform Sync Systems

Configure via the config plugin:

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    // Disabled mode
    app.add_plugins(GodotCustomTransformSyncPlugin {
        sync_mode: TransformSyncMode::Disabled,
    });
    
    // One-way mode (default)
    app.add_plugins(GodotCustomTransformSyncPlugin::default());
    
    // Two-way mode
    app.add_plugins(GodotCustomTransformSyncPlugin {
        sync_mode: TransformSyncMode::TwoWay,
    });
    
    // Add your custom systems
    add_transform_sync_systems! {
        app,
        PhysicsOnly = Or<(With<RigidBody3DMarker>, With<CharacterBody3DMarker>)>
    }
}
```

### Runtime Configuration

You can also modify configuration at runtime:

```rust
// For default systems
app.insert_resource(GodotDefaultTransformSyncConfig::two_way());

// For custom systems
app.insert_resource(GodotCustomTransformSyncConfig::two_way());
```

## Performance Impact

### Disabled Mode Performance
```
Transform Components: Not created
Sync Systems: Not running
Memory Usage: None
CPU Usage: None
```

### One-Way Mode Performance
```
Transform Components: Created
Write Systems: Running (Last schedule)
Read Systems: Not running
Memory Usage: ~48 bytes per entity
CPU Usage: O(changed entities)
```

### Two-Way Mode Performance
```
Transform Components: Created
Write Systems: Running (Last schedule)
Read Systems: Running (PreUpdate schedule)
Memory Usage: ~48 bytes per entity
CPU Usage: O(all entities with transforms)
```

## Implementation Details

### System Execution Order

**Write Systems (ECS → Godot)**
- Schedule: `Last`
- Only processes changed transforms
- Runs for both OneWay and TwoWay modes

**Read Systems (Godot → ECS)**
- Schedule: `PreUpdate`
- Checks all transforms for external changes
- Only runs in TwoWay mode

### Change Detection

The system uses Bevy's change detection to optimize writes:

```rust
fn post_update_transforms(
    mut query: Query<
        (&Transform2D, &mut GodotNodeHandle),
        Or<(Added<Transform2D>, Changed<Transform2D>)>
    >
) {
    // Only processes entities with new or changed transforms
}
```

## Common Patterns

### Switching Modes at Runtime

While not common, you can change modes during runtime:

```rust
// For default systems
fn switch_default_to_physics_mode(
    mut commands: Commands,
) {
    commands.insert_resource(GodotDefaultTransformSyncConfig::disabled());
}

// For custom systems  
fn switch_custom_to_physics_mode(
    mut commands: Commands,
) {
    commands.insert_resource(GodotCustomTransformSyncConfig::disabled());
}
```

Note: Existing transform components remain but stop syncing.

### Checking Current Mode

```rust
// For default systems
fn check_default_sync_mode(
    config: Res<GodotDefaultTransformSyncConfig>,
) {
    match config.sync_mode {
        TransformSyncMode::Disabled => {
            println!("Default sync: Using direct physics");
        }
        TransformSyncMode::OneWay => {
            println!("Default sync: ECS drives transforms");
        }
        TransformSyncMode::TwoWay => {
            println!("Default sync: Bidirectional sync active");
        }
    }
}

// For custom systems
fn check_custom_sync_mode(
    config: Res<GodotCustomTransformSyncConfig>,
) {
    match config.sync_mode {
        TransformSyncMode::Disabled => {
            println!("Custom sync: Using direct physics");
        }
        TransformSyncMode::OneWay => {
            println!("Custom sync: ECS drives transforms");
        }
        TransformSyncMode::TwoWay => {
            println!("Custom sync: Bidirectional sync active");
        }
    }
}
```

## Best Practices

1. **Choose mode early** - Switching modes mid-project can be complex
2. **Default to OneWay** - Unless you specifically need other modes
3. **Benchmark your game** - Measure actual performance impact
4. **Document your choice** - Help team members understand the architecture

## Troubleshooting

### "Transform changes not visible"
- Check you're not in Disabled mode
- Ensure transform components exist on entities
- Verify systems are running in correct schedules

### "Performance degradation with many entities"
- Consider switching from TwoWay to OneWay
- Use Disabled mode for physics entities
- Profile to identify bottlenecks

### "Godot animations not affecting ECS"
- Enable TwoWay mode for animated entities
- Ensure transforms aren't being overwritten by ECS systems
- Check system execution order