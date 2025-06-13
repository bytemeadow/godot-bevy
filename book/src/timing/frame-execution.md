# Frame Execution Model

Understanding how godot-bevy integrates with Godot's frame timing is crucial for building performant games. This chapter explains the execution model and how different schedules interact.

## Two Types of Frames

### Visual Frames (`_process`)

Visual frames run at your display's refresh rate and handle the main Bevy update cycle.

**What runs:** The complete `app.update()` cycle
- `First`
- `PreUpdate` 
- `Update`
- `FixedUpdate`
- `PostUpdate`
- `Last`

**Frequency:** Matches Godot's visual framerate (typically 60-144 FPS)

**Use for:**
- Game logic
- UI updates
- Rendering-related systems
- Most gameplay code

### Physics Frames (`_physics_process`)

Physics frames run at Godot's fixed physics tick rate.

**What runs:** Only the `PhysicsUpdate` schedule

**Frequency:** Godot's physics tick rate (default 60 Hz)

**Use for:**
- Physics calculations
- Movement that needs to sync with Godot physics
- Collision detection
- Anything that must run at a fixed rate

## Schedule Execution Order

### Within Visual Frames

```
Visual Frame Start
    ├── First
    ├── PreUpdate (reads Godot → ECS transforms)
    ├── Update (your game logic)
    ├── FixedUpdate (0, 1, or multiple times)
    ├── PostUpdate
    └── Last (writes ECS → Godot transforms)
Visual Frame End
```

### Independent Physics Frames

```
Physics Frame Start
    └── PhysicsUpdate (your physics logic)
Physics Frame End
```

⚠️ **Important:** Physics frames run independently and can execute:
- Before a visual frame starts
- Between any visual frame schedules
- After a visual frame completes
- Multiple times between visual frames

## Frame Rate Relationships

Different parts of your game run at different rates:

| Schedule | Rate | Use Case |
|----------|------|----------|
| Visual schedules | Display refresh (60-144 Hz) | Rendering, UI, general logic |
| PhysicsUpdate | Physics tick (60 Hz) | Godot physics integration |
| FixedUpdate | Bevy's rate (64 Hz default) | Consistent gameplay simulation |

## Practical Example

Here's how different systems should be scheduled:

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    // Visual frame systems
    app.add_systems(Update, (
        ui_system,
        camera_follow,
        animation_system,
    ));
    
    // Fixed timestep for consistent simulation
    app.add_systems(FixedUpdate, (
        ai_behavior,
        cooldown_timers,
    ));
    
    // Godot physics integration
    app.add_systems(PhysicsUpdate, (
        character_movement,
        collision_response,
    ));
}
```

## Delta Time Usage

Different schedules require different delta time sources:

### In Update Systems

```rust
fn movement_system(
    time: Res<Time>,
    mut query: Query<&mut Transform2D>,
) {
    let delta = time.delta_seconds();
    // Use Bevy's time for visual frame systems
}
```

### In PhysicsUpdate Systems

```rust
fn physics_movement(
    physics_delta: Res<PhysicsDelta>,
    mut query: Query<&mut Transform2D>,
) {
    let delta = physics_delta.delta_seconds;
    // Use Godot's physics delta for physics systems
}
```

## Common Pitfalls

### ❌ Don't modify the same data in multiple schedules

```rust
// BAD: Conflicting modifications
app.add_systems(Update, move_player);
app.add_systems(PhysicsUpdate, also_move_player); // Conflicts!
```

### ❌ Don't expect immediate cross-schedule visibility

```rust
// BAD: Expecting immediate updates
fn physics_system() {
    // Set position in PhysicsUpdate
}

fn visual_system() {
    // Won't see physics changes until next frame!
}
```

### ✅ Do use appropriate schedules for each task

```rust
// GOOD: Clear separation of concerns
app.add_systems(Update, render_effects);
app.add_systems(PhysicsUpdate, apply_physics);
app.add_systems(FixedUpdate, update_ai);
```

## Performance Considerations

1. **Visual frames** can vary widely (30-144+ FPS)
2. **PhysicsUpdate** provides consistent timing for physics
3. **FixedUpdate** may run multiple times per visual frame to catch up
4. Transform syncing happens at schedule boundaries

> **Note:** Scene tree entities are initialized during `PreStartup`, before any `Startup` systems run. This means you can safely query Godot scene entities in your `Startup` systems! See [Scene Tree Initialization and Timing](../scene-tree/timing.md) for details.

## Next Steps

- Learn about the [PhysicsUpdate Schedule](./physics-update.md) in detail
- Understand [Transform Syncing](../transforms/sync-modes.md) timing
- Explore [Performance Optimization](../reference/performance.md) techniques