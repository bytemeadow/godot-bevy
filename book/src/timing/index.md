# Frame Execution Model

Understanding how godot-bevy integrates with Godot's frame timing is crucial for building performant games.

## Two Types of Frames

### Visual Frames (`_process`)

Visual frames run at your display's refresh rate and drive the main Bevy update cycle.

**What runs:** The complete `app.update()` cycle
- `First`
- `PreUpdate` (reads Godot → ECS transforms)
- `Update`
- `FixedMain` (zero or more times — see below)
- `PostUpdate`
- `Last`

**Frequency:** Matches Godot's visual framerate (typically 60–144 FPS)

**Use for:**
- Game logic
- UI updates
- Rendering-related systems
- Most gameplay code

### Physics Frames (`_physics_process`)

Physics frames run at Godot's fixed physics tick rate and drive Bevy's `FixedMain` schedule.

**What runs:** The complete `FixedMain` schedule
- `FixedFirst`
- `FixedPreUpdate`
- `FixedUpdate` (your physics logic)
- `FixedPostUpdate`
- `FixedLast` (writes ECS → Godot transforms at physics rate)

**Frequency:** Godot's physics tick rate (default 60 Hz)

**Use for:**
- Physics calculations
- Movement that needs to sync with Godot physics
- Collision detection
- Anything that must run at a fixed, deterministic rate

## Schedule Execution Order

### Visual Frame

```
Visual Frame Start
    ├── First
    ├── PreUpdate  (reads Godot → ECS transforms)
    ├── Update     (your visual-rate logic)
    ├── FixedMain  (driven by _physics_process, not here)
    ├── PostUpdate
    └── Last
Visual Frame End
```

### Physics Frame

```
Physics Frame Start
    ├── FixedFirst
    ├── FixedPreUpdate
    ├── FixedUpdate   (your physics/fixed logic)
    ├── FixedPostUpdate
    └── FixedLast     (writes ECS → Godot transforms)
Physics Frame End
```

Physics frames run on Godot's authoritative clock, independently of visual frames. They can execute before, between, or after visual frames.

## Frame Rate Relationships

| Schedule | Rate | Use case |
|----------|------|----------|
| Visual schedules (`Update`, etc.) | Display refresh (60–144 Hz) | Rendering, UI, general logic |
| `FixedUpdate` / `FixedMain` | Godot's physics rate (default 60 Hz) | Physics, deterministic simulation |

> **Note:** Bevy's default `FixedUpdate` rate (64 Hz) is **not used**. godot-bevy drives `FixedMain` directly from `_physics_process`, so the rate is always Godot's physics rate — whatever is set in Project Settings → Physics → Common → Physics Ticks Per Second.

## Practical Example

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    // Visual-rate systems
    app.add_systems(Update, (
        ui_system,
        camera_follow,
        animation_system,
    ));

    // Fixed-rate physics systems (runs at Godot's physics rate, default 60 Hz)
    app.add_systems(FixedUpdate, (
        character_movement,
        collision_response,
        ai_behavior,
    ));
}
```

## Delta Time

### In `Update` Systems

```rust
fn movement_system(
    time: Res<Time>,
    mut query: Query<&mut Transform>,
) {
    let delta = time.delta_secs();
    // Visual-frame delta — varies with framerate
}
```

### In `FixedUpdate` Systems

```rust
fn physics_movement(
    time: Res<Time>,
    mut query: Query<&mut Transform>,
) {
    let delta = time.delta_secs();
    // Fixed delta — always equals Godot's physics delta (e.g. 1/60 s at 60 Hz)
    // Engine.time_scale is baked in, so slow-motion works automatically
}
```

`Res<Time>` works correctly in both schedules — in `FixedUpdate` it reports the fixed physics delta automatically. Do **not** hardcode `1.0 / 60.0`; read `time.delta_secs()` instead.

## Physics Interpolation

godot-bevy writes transforms at physics rate (`FixedLast`). To get visually smooth movement between physics ticks, enable Godot's built-in physics interpolation:

**Project Settings → Physics → Common → Physics Interpolation** = `true`

or in `project.godot`:

```ini
[physics]
common/physics_interpolation=true
```

When a node teleports (respawn, warp), call `reset_physics_interpolation()` on it immediately after moving so Godot doesn't interpolate through the jump.

> **Note:** Bevy-side interpolation plugins (e.g. `bevy_interpolation`) and rollback netcode are not supported in godot-bevy's transform sync path — use Godot's built-in interpolation instead.

## Common Pitfalls

### Don't put physics logic in `Update`

```rust
// BAD: Update runs at variable framerate; physics won't be deterministic
app.add_systems(Update, character_movement);

// GOOD: FixedUpdate runs at physics rate
app.add_systems(FixedUpdate, character_movement);
```

### Don't hardcode the physics timestep

```rust
// BAD: breaks if physics rate changes in Project Settings
let delta = 1.0 / 60.0;

// GOOD: always correct, honors time_scale
fn physics_system(time: Res<Time>) {
    let delta = time.delta_secs();
}
```

### Don't expect immediate cross-schedule visibility

Data written in `FixedLast` (Godot transforms) won't be visible to the next `_process` `PreUpdate` read until the following visual frame.

## Performance Considerations

1. **Visual frames** vary widely (30–144+ FPS)
2. **FixedUpdate** runs at a constant rate driven by Godot's physics clock
3. Transform syncing from ECS → Godot happens in `FixedLast`; Godot → ECS happens in `PreUpdate`

> **Note:** Scene tree entities are initialized during `PreStartup`, before any `Startup` systems run. You can safely query Godot scene entities in `Startup` systems. See [Scene Tree Initialization and Timing](../scene-tree/timing.md) for details.
