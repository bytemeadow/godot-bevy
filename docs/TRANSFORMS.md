# Transform Syncing in Godot-Bevy

This document explains how godot-bevy handles transform synchronization between Godot nodes and Bevy ECS components.

## ⚠️ **Important: Default Behavior**

**By default, transform syncing is one-way: ECS → Godot**
- ✅ **Writing enabled**: ECS components update Godot node transforms
- ❌ **Reading disabled**: Godot node transforms don't update ECS components

This is optimal for pure ECS applications where all transform logic happens in Bevy systems.

## Transform Sync Modes

Godot-Bevy supports three distinct sync modes:

### `TransformSyncMode::Disabled`
- **No syncing** - zero overhead
- **No transform components created** - saves memory and CPU
- Use `GodotNodeHandle` for direct Godot physics (`move_and_slide`, etc.)
- **Best for**: Platformers, physics-heavy games

### `TransformSyncMode::OneWay` (Default)
- **ECS → Godot** syncing only
- ECS components drive Godot node positions
- **Best for**: Pure ECS games, simple movement systems

### `TransformSyncMode::TwoWay`
- **ECS ↔ Godot** bidirectional syncing
- External changes to Godot nodes sync back to ECS
- **Best for**: Hybrid apps migrating from GDScript to ECS


## Transform Components

`godot-bevy` provides unified transform components that maintain both Bevy and Godot representations:

### `Transform3D`
```rust
use godot_bevy::prelude::*;

fn setup_3d_entity(mut commands: Commands) {
    commands.spawn(Transform3D::from(Transform::from_translation(Vec3::new(1.0, 2.0, 3.0))));
}

fn move_3d_entity(mut transforms: Query<&mut Transform3D>) {
    for mut transform in transforms.iter_mut() {
        // Modify through Bevy interface
        transform.as_bevy_mut().translation.x += 1.0;
        
        // Or modify through Godot interface
        transform.as_godot_mut().origin.x += 1.0;
    }
}
```

### `Transform2D`
```rust
use godot_bevy::prelude::*;

fn setup_2d_entity(mut commands: Commands) {
    commands.spawn(Transform2D::from(Transform::from_translation(Vec3::new(100.0, 200.0, 0.0))));
}

fn move_2d_entity(mut transforms: Query<&mut Transform2D>) {
    for mut transform in transforms.iter_mut() {
        // Modify through Bevy interface
        transform.as_bevy_mut().translation.x += 10.0;
        
        // Or modify through Godot interface  
        transform.as_godot_mut().origin.x += 10.0;
    }
}
```

## Synchronization Direction

### Writing (ECS → Godot) - Always Enabled
When ECS components change, the corresponding Godot node transforms are automatically updated:

```rust
fn update_player_position(mut query: Query<&mut Transform3D, With<Player>>) {
    for mut transform in query.iter_mut() {
        // This change will automatically sync to the Godot node
        transform.as_bevy_mut().translation.x += 5.0;
    }
}
```

### Reading (Godot → ECS) - Opt-in Only
When Godot nodes are modified outside of ECS (e.g., by GDScript, animations, or physics), those changes can optionally sync back to ECS components.

## When to Enable Transform Reading

### ✅ **Enable Reading For:**
- **Hybrid applications** gradually migrating from GDScript to Bevy ECS
- **Physics-driven transforms** where Godot's physics engine moves RigidBody nodes
- **External tool integration** where transforms are modified outside your Rust code

### ❌ **Keep Reading Disabled For:**
- **Pure ECS applications** where all transform logic is in Bevy systems
- **Performance-critical applications** (reading adds overhead)
- **Simple games** with straightforward transform management

## Configuring Transform Sync

Configure the sync mode in your `#[bevy_app]` function:

### Disabled Mode (For Direct Godot Physics)
```rust
use godot_bevy::prelude::*;

#[bevy_app]
fn build_app(app: &mut App) {
    // Disable transform syncing - use direct Godot physics
    app.insert_resource(GodotTransformConfig::disabled());
    
    app.add_systems(PhysicsUpdate, player_physics_movement);
}
```

### One-Way Mode (Default - ECS Drives Transforms)
```rust
#[bevy_app]
fn build_app(app: &mut App) {
    // One-way sync is the default - no configuration needed
    // Or explicitly set it:
    app.insert_resource(GodotTransformConfig::one_way());
    
    app.add_systems(Update, move_entities_with_ecs);
}
```

### Two-Way Mode (For Hybrid Apps)
```rust
#[bevy_app] 
fn build_app(app: &mut App) {
    // Enable bidirectional sync for hybrid apps
    app.insert_resource(GodotTransformConfig::two_way());
    
    app.add_systems(Update, (
        handle_player_movement,
        sync_camera_to_player,
    ));
}
```

### Manual Configuration
```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.insert_resource(GodotTransformConfig {
        sync_mode: TransformSyncMode::TwoWay,
    });
}
```

## Migration Strategy for Hybrid Apps

When gradually migrating from GDScript to ECS:

### Phase 1: Enable Two-Way Sync
```rust
#[bevy_app]
fn build_app(app: &mut App) {
    // Enable bidirectional sync during migration
    app.insert_resource(GodotTransformConfig::two_way());
    
    app.add_systems(Update, (
        // Start with ECS systems for new features
        handle_new_gameplay_mechanics,
        // Keep existing GDScript for complex logic
    ));
}
```

### Phase 2: Gradual Migration
```rust
fn migrated_player_system(
    mut player_query: Query<&mut Transform3D, With<Player>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    for mut transform in player_query.iter_mut() {
        // Gradually move logic from GDScript to ECS
        if input.pressed(KeyCode::ArrowRight) {
            transform.as_bevy_mut().translation.x += 5.0;
        }
    }
}
```

### Phase 3: Pure ECS
```rust
#[bevy_app]
fn build_app(app: &mut App) {
    // Switch to one-way sync once migration is complete
    // (or simply remove the resource configuration - one-way is default)
    app.insert_resource(GodotTransformConfig::one_way());
    
    app.add_systems(Update, (
        handle_player_movement,
        handle_enemy_ai,
        handle_projectiles,
    ));
}
```

## Performance Considerations

### Sync Mode Performance

**`Disabled` Mode:**
- ✅ **Zero overhead** - no transform systems run
- ✅ **No memory usage** - transform components not created
- ✅ **Best performance** for physics-heavy games

**`OneWay` Mode:**
- ✅ **Minimal overhead** - only writing systems run
- ✅ **Good performance** for ECS-driven games

**`TwoWay` Mode:**
- ❌ **Higher overhead** - both reading and writing systems run
- ❌ **Use only when necessary** for hybrid scenarios

### Optimization Tips
1. **Choose the right mode** for your use case
2. **Use `Disabled` mode** for direct Godot physics
3. **Switch from `TwoWay` to `OneWay`** once migration is complete
4. **Use change detection** in your systems to minimize unnecessary work
5. **Batch transform updates** when possible

```rust
fn optimized_transform_system(
    mut transforms: Query<&mut Transform3D, Changed<Transform3D>>,
) {
    // Only processes entities whose transforms actually changed
    for mut transform in transforms.iter_mut() {
        // Process only changed transforms
    }
}
```

## System Execution Order

Transform synchronization happens at specific points in the frame:

### Writing Systems (ECS → Godot)
- **Schedule**: `Last`
- **When**: After all gameplay systems have run
- **Purpose**: Ensure Godot nodes reflect final ECS component state

### Reading Systems (Godot → ECS)
- **Schedule**: `PreUpdate` 
- **When**: Before gameplay systems run
- **Purpose**: Ensure ECS components reflect external Godot changes
- **Condition**: Only runs when `enable_transform_reading` is true

## Direct Godot Physics Pattern

For physics-heavy games like platformers, you can bypass transform syncing entirely and use Godot's physics directly:

```rust
use godot_bevy::prelude::*;
use godot::classes::{CharacterBody2D, ICharacterBody2D};

#[derive(GodotClass, BevyBundle)]
#[class(base=CharacterBody2D)]
#[bevy_bundle((Speed: speed), (Player))]
pub struct Player2D {
    base: Base<CharacterBody2D>,
    #[export]
    speed: f32,
}

fn player_movement(
    mut player: Query<(&mut GodotNodeHandle, &Speed), With<Player>>,
    input: Res<ButtonInput<KeyCode>>,
) {
    if let Ok((mut handle, speed)) = player.single_mut() {
        let Some(mut character_body) = handle.try_get::<CharacterBody2D>() else { return };
        
        let mut velocity = character_body.get_velocity();
        
        // Direct physics manipulation - no transform syncing needed
        if input.pressed(KeyCode::ArrowLeft) {
            velocity.x = -speed.0;
        } else if input.pressed(KeyCode::ArrowRight) {
            velocity.x = speed.0;
        } else {
            velocity.x = 0.0;
        }
        
        character_body.set_velocity(velocity);
        character_body.move_and_slide(); // Godot handles position updates
    }
}
```

**Benefits of Direct Physics:**
- ✅ **Native collision detection** using Godot's physics engine
- ✅ **Better performance** for physics-heavy games
- ✅ **No transform syncing overhead**
- ✅ **Access to specialized physics features** (slopes, moving platforms, etc.)
- ✅ **Familiar to Godot developers**

**When to use:**
- Platformers and physics-based games
- Complex collision scenarios
- When you need Godot's specialized physics features
- Performance-critical movement systems

## Common Patterns

### Animation-Driven Movement (Two-Way Sync)
```rust
#[bevy_app]
fn build_app(app: &mut App) {
    // Enable two-way sync for animation-driven transforms
    app.insert_resource(GodotTransformConfig::two_way());
    
    app.add_systems(Update, sync_animated_platforms);
}

fn sync_animated_platforms(
    animated_platforms: Query<&Transform3D, (With<AnimatedPlatform>, Changed<Transform3D>)>,
    mut player_query: Query<&mut Transform3D, With<Player>>,
) {
    // React to platform movements driven by Godot animations
    for platform_transform in animated_platforms.iter() {
        // Update player position relative to moving platform
    }
}
```

### Direct Godot Physics (Disabled Sync)
```rust
#[bevy_app]
fn build_app(app: &mut App) {
    // Disable sync for direct physics
    app.insert_resource(GodotTransformConfig::disabled());
    
    app.add_systems(PhysicsUpdate, handle_physics_objects);
}

fn handle_physics_objects(
    mut physics_objects: Query<&mut GodotNodeHandle, With<PhysicsObject>>,
) {
    // Direct Godot physics manipulation - no transform sync needed
    for mut handle in physics_objects.iter_mut() {
        let Some(mut body) = handle.try_get::<RigidBody2D>() else { continue };
        // Apply forces, velocities, etc. directly to Godot physics
        body.apply_central_impulse(Vector2::new(100.0, 0.0));
    }
}
```
