# Frame Execution Model

Understanding how godot-bevy integrates with Godot's frame timing is crucial for building performant games.

## Two Godot Callbacks, One Bevy Frame

godot-bevy splits Bevy's standard `Main` schedule across Godot's two frame callbacks at the fixed-loop boundary. `app.update()` is never called in production: the prefix and the fixed loop run in `_physics_process`, and the suffix runs in `_process`.

### Physics Frames (`_physics_process`)

The **prefix** of the main schedule plus the fixed loop run here, on Godot's physics clock.

**What runs:**
- `First`
- `PreUpdate` (reads Godot → ECS transforms, once per render frame)
- `StateTransition`
- `FixedMain` (zero or more times per render frame)
  - `FixedFirst` (TwoWay: re-reads Godot → ECS per physics step, steps 2..N)
  - `FixedPreUpdate`
  - `FixedUpdate` (your physics logic)
  - `FixedPostUpdate`
  - `FixedLast` (writes ECS → Godot transforms at physics rate)

The prefix (`First` → `StateTransition`) runs once per render frame, on the first physics step; `FixedMain` runs once per physics step.

**Frequency:** Godot's physics tick rate (default 60 Hz)

**Use for:**
- Physics calculations
- Movement that needs to sync with Godot physics
- Collision detection
- Anything that must run at a fixed, deterministic rate

### Visual Frames (`_process`)

The **suffix** of the main schedule runs here, then `clear_trackers` fires once for the whole render frame.

**What runs:**
- `Update`
- `PostUpdate`
- `Last`

**Frequency:** Matches Godot's visual framerate (typically 60–144 FPS)

**Use for:**
- Game logic
- UI updates
- Rendering-related systems
- Most gameplay code

On a render frame with no physics step, the prefix runs in `_process` before the suffix, so a frame is never skipped.

## Schedule Execution Order

### Physics Frame (`_physics_process`)

```
Physics Frame Start
    ├── First           ┐
    ├── PreUpdate       │ prefix: once per render frame (reads Godot → ECS)
    ├── StateTransition ┘
    └── FixedMain       (once per physics step)
        ├── FixedFirst      (TwoWay: re-reads Godot → ECS, steps 2..N)
        ├── FixedPreUpdate
        ├── FixedUpdate     (your physics/fixed logic)
        ├── FixedPostUpdate
        └── FixedLast       (writes ECS → Godot transforms)
Physics Frame End
```

### Visual Frame (`_process`)

```
Visual Frame Start
    ├── Update     (your visual-rate logic)
    ├── PostUpdate
    ├── Last
    └── clear_trackers (once per render frame)
Visual Frame End
```

Physics steps run on Godot's authoritative clock: each render frame drives the prefix and 0, 1, or N `FixedMain` steps in `_physics_process` before the suffix runs in `_process` — a deterministic order, not independent schedules.

### `BeforeFixedMainLoop` / `AfterFixedMainLoop` anchors

godot-bevy runs the whole `RunFixedMainLoop` schedule once per physics step, so the `BeforeFixedMainLoop` and `AfterFixedMainLoop` anchor sets fire **once per step** — 0, 1, or N times per render frame — not Bevy's stock once per frame. Order ecosystem systems (e.g. leafwing's input-buffer swap) against them with that cadence in mind.

A `GodotActions` read inside either anchor sees the **process-clock** snapshot: the active clock is flipped to physics only around `FixedMain` itself. Read actions in `FixedUpdate` (physics snapshot) or `Update` (process snapshot), not in the anchors.

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

> **Note:** Bevy-side interpolation plugins (e.g. avian's `PhysicsInterpolationPlugin`, `bevy_transform_interpolation`) and rollback netcode (`bevy_ggrs`) are not supported in godot-bevy's transform sync path — use Godot's built-in interpolation instead.

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

Godot transforms written in `FixedLast` aren't read back into ECS until the next render frame's `PreUpdate` (which runs in the physics prefix).

## Performance Considerations

1. **Visual frames** vary widely (30–144+ FPS)
2. **FixedUpdate** runs at a constant rate driven by Godot's physics clock
3. Transform syncing from ECS → Godot happens in `FixedLast`; Godot → ECS happens in `PreUpdate` (and per physics step in `FixedFirst` under TwoWay)

> **Note:** Scene tree entities are initialized during `PreStartup`, before any `Startup` systems run. You can safely query Godot scene entities in `Startup` systems. See [Scene Tree Initialization and Timing](../scene-tree/timing.md) for details.
