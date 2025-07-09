# Transform Sync Modes

godot-bevy provides three transform synchronization modes to fit different use cases. Understanding these modes is crucial for optimal performance and correct behavior.

## Available Modes

### `TransformSyncMode::Disabled`

No transform syncing occurs and no transform components are created.

**Characteristics:**
- ✅ Zero performance overhead
- ✅ Best for when your ECS systems rarely read/write Godot Node transforms, or you wish to explicitly control when
synchronization occurs
- ❌ Godot Transform changes aren't automatically reflected in ECS
- ❌ ECS Transform changes aren't automatically reflected in Godot

**Use when:**
- Building platformers with CharacterBody2D
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
- Physics is controlled in ECS, i.e., you've disabled all Godot Physics engines and use something like Avian physics

### `TransformSyncMode::TwoWay`

Full bidirectional synchronization between ECS and Godot.

**Characteristics:**
- ✅ Changes in either system are reflected
- ✅ Works with Godot animations
- ✅ Supports hybrid architectures
- ❌ Highest performance cost

**Use when:**
- Migrating from GDScript to ECS
- Using Godot's AnimationPlayer
- Mixing ECS and GDScript logic

## Configuration

Configure the sync mode in your `#[bevy_app]` function:

### Disabled Mode

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.insert_resource(GodotTransformConfig::disabled());
    
    // Use direct physics instead
    app.add_systems(Update, physics_movement);
}
```

### One-Way Mode (Default)

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    // One-way is the default, no configuration needed
    // Or explicitly:
    app.insert_resource(GodotTransformConfig::one_way());
    
    app.add_systems(Update, ecs_movement);
}
```

### Two-Way Mode

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.insert_resource(GodotTransformConfig::two_way());
    
    app.add_systems(Update, hybrid_movement);
}
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
        (&Transform, &mut GodotNodeHandle),
        Or<(Added<Transform>, Changed<Transform>)>
    >
) {
    // Only processes entities with new or changed transforms
}
```

## Common Patterns

### Switching Modes at Runtime

While not common, you can change modes during runtime:

```rust
fn switch_to_physics_mode(
    mut commands: Commands,
) {
    commands.insert_resource(GodotTransformConfig::disabled());
}
```

Note: Existing transform components remain but stop syncing.

### Checking Current Mode

```rust
fn check_sync_mode(
    config: Res<GodotTransformConfig>,
) {
    match config.sync_mode {
        TransformSyncMode::Disabled => {
            println!("Using direct physics");
        }
        TransformSyncMode::OneWay => {
            println!("ECS drives transforms");
        }
        TransformSyncMode::TwoWay => {
            println!("Bidirectional sync active");
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
