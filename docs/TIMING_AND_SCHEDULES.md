# Timing and Schedules in Godot-Bevy

This document explains how godot-bevy integrates with Godot's frame timing and provides guidance on when to use different Bevy schedules.

## Frame Execution Model

Godot-Bevy provides a clean integration with Godot's frame timing:

### Visual Frame (`_process`)
- **What runs**: `app.update()` - the complete Bevy update cycle
- **Includes**: `First`, `PreUpdate`, `Update`, `FixedUpdate`, `PostUpdate`, `Last`
- **Frequency**: Godot's visual framerate (typically 60-120 FPS)
- **Use for**: Game logic, UI updates, rendering-related systems

### Physics Frame (`_physics_process`)
- **What runs**: `PhysicsUpdate` schedule only
- **Frequency**: Godot's physics tickrate (by default 60 Hz)
- **Use for**: Systems that need to sync with Godot's physics timing
- **Important**: Executes independently of visual frames - can run between any visual frame schedules

## Scheduling Relationships

### No Strong Ordering Guarantees
`PhysicsUpdate` and the visual frame schedules (`First`, `PreUpdate`, `Update`, etc.) run independently. A physics frame might execute:
- Before a visual frame starts
- Between `PreUpdate` and `Update` 
- After `Last` completes
- Multiple times between visual frames (if physics rate > visual rate)

### Frame Rate Relationships
- **Visual frames**: Run at your display's refresh rate (60-144 FPS)
- **Physics frames**: Run at Godot's physics tick rate (usually 60 Hz)
- **FixedUpdate**: Maintains Bevy's internal timing (64 Hz by default)

## Data Flow and Synchronization

### Transform Updates
The library handles transform synchronization intelligently:
- **PreUpdate**: Reads Godot transforms into Bevy components (unless recently changed by Bevy)
- **Last**: Writes Bevy transform changes back to Godot nodes
- **PhysicsUpdate**: Can modify transforms, which will be detected and synchronized in the next visual frame's `Last` schedule

### Data Synchronization Timing
Changes made in one schedule are visible in others, but with frame delays:
- Transform changes in `PhysicsUpdate` → visible in next visual frame's `Update`
- Transform changes in `Update` → visible in same or next `PhysicsUpdate`
- The transform sync systems (`PreUpdate`/`Last`) handle the bidirectional synchronization

## Best Practices

### ✅ Safe Patterns
- Update transforms in `PhysicsUpdate`, read them in `Update` (next visual frame)
- Update transforms in `Update`, read them in `PhysicsUpdate` (same or next physics frame)

### ⚠️ Patterns to Avoid
- Modifying the same transform in both `PhysicsUpdate` and visual frame schedules simultaneously
- Expecting immediate synchronization within the same frame - changes propagate on the next frame cycle

## Usage Guidelines

```rust
// Game logic that must run once per render frame
app.add_systems(Update, my_gameplay_system);

// Game logic - Bevy's built-in fixed timestep
app.add_systems(FixedUpdate, my_physics_simulation);

// Godot-specific physics - synchronized with Godot's physics
app.add_systems(PhysicsUpdate, godot_movement_system);
```

### Delta Time Considerations
- **Systems in `PhysicsUpdate`**: Use `PhysicsDelta` for Godot's physics delta time
- **Systems in standard schedules**: Use `Res<Time>`

## Schedule Usage Examples

### General Game Logic (Update)
```rust
// For general game logic, UI, rendering - runs in visual frames
app.add_systems(Update, gameplay_system);
```

### Physics Simulation (FixedUpdate)
```rust
// For gameplay logic, AI - Bevy's built-in fixed timestep
app.add_systems(FixedUpdate, physics_simulation);
```

### Godot Physics Integration (PhysicsUpdate)
```rust
// For Godot-specific physics - synchronized with Godot's physics
app.add_systems(PhysicsUpdate, godot_movement_system);
```

## Debugging and Understanding

### High-Level Observations
- **High visual frame rates** (100+ FPS) are normal and indicate good performance
- **FixedUpdate** may run 0, 1, or 2+ times per visual frame to maintain consistent timing
- **PhysicsUpdate** runs independently at Godot's physics rate
- **Timing consistency** shows that each schedule runs when expected

### Common Use Cases
This timing model is particularly useful for:
- Understanding when to use each Bevy schedule
- Debugging timing-related issues
- Verifying frame rate expectations
- Learning about fixed timestep vs variable timestep systems 