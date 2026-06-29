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
- ✅ Per-axis co-authorship -- Godot and Bevy can drive different axes of the same node
- ✅ Bevy reads the latest Godot value *before* your systems run (read every physics step in `FixedFirst`; on 0-step frames, in `PreUpdate`)
- ❌ Highest performance cost

**Use when:**
- Migrating from GDScript to ECS
- Using Godot's AnimationPlayer
- Mixing ECS and GDScript logic

#### Co-authorship semantics

The Godot→Bevy read is primary in `FixedFirst`, running once per physics step before your
`FixedUpdate` systems, so a node moved from Godot -- by GDScript, an `AnimationPlayer`, or
physics -- *between* steps stays visible every step (matching the `FixedLast` write cadence)
rather than being clobbered by a stale whole-transform write. On a render frame with no
physics step the read falls back to `PreUpdate`, keeping idle frames covered. Either way the
last read precedes the `Update` suffix, so your `Update` systems see Godot's latest value the
same frame. The Bevy→Godot write runs in `FixedLast` and pushes only what Bevy changed,
tracked against a per-entity value shadow. So a single node can be co-authored
**per axis**:

```gdscript
# quad.gd -- Godot drives x
func _process(_dt): position.x = sin(t) * 100.0
```
```rust
// Bevy drives y; x stays whatever Godot set it to
fn move_y(mut q: Query<&mut Transform, With<Quad>>) {
    for mut t in &mut q { t.translation.y = cos(time) * 100.0; }
}
```

What this guarantees and what it doesn't:

- **Translation and scale** are co-authored **per component** (`x`/`y`/`z` independently).
- **Rotation** is whole -- quaternion components aren't independently meaningful, so rotation
  is authored by one side at a time (2D rotation is a single angle anyway).
- If **both sides change the same axis in the same frame**, Bevy wins.
- A value authored in Godot's **idle phase** (`_process`, `AnimationPlayer` in idle) is seen
  by Bevy the next frame, since Bevy's primary read runs in the physics phase, before Godot's
  idle phase within a frame -- the same one-frame relationship any Godot `_physics_process`
  reader has with an idle-phase writer.

> **Freshness trade:** because the read is primary in `FixedFirst`, any *prefix* schedule
> (`First`, `PreUpdate`, `StateTransition`) on a frame with one or more physics steps sees
> **last frame's** synced `Transform` -- the fresh Godot value isn't merged until
> `FixedFirst` runs. Read this-frame's Godot value in `FixedUpdate` onward or in the
> `Update` suffix, not in a prefix schedule.

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
Write Systems: Running (FixedLast schedule)
Read Systems: Not running
Memory Usage: ~48 bytes per entity
CPU Usage: O(changed entities)
```

### Two-Way Mode Performance
```
Transform Components: Created
Write Systems: Running (FixedLast schedule)
Read Systems: Running (FixedFirst primary, PreUpdate 0-step fallback)
Memory Usage: ~48 bytes per entity
CPU Usage: O(all entities with transforms)
```

## Implementation Details

### System Execution Order

**Write Systems (ECS → Godot)**
- Schedule: `FixedLast` (physics rate, once per fixed step)
- Only processes changed transforms
- Runs for both OneWay and TwoWay modes

**Read Systems (Godot → ECS)**
- Schedule: `FixedFirst` (every physics step) and `PreUpdate` (0-step frames only)
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

## Render interpolation

godot-bevy drives `FixedMain` directly from Godot's `_physics_process`, so `Time<Fixed>::overstep_fraction()` is always `0.0` -- there is no fractional leftover between the physics clock and the render clock. Interpolation plugins that read this value to ease between fixed steps -- `bevy_transform_interpolation`, avian's `PhysicsInterpolationPlugin` -- will snap positions on every frame rather than smooth them. Neither is supported.

Use Godot's built-in physics interpolation instead:

**Project Settings → Physics → Common → Physics Interpolation** = `true`

or in `project.godot`:

```ini
[physics]
common/physics_interpolation=true
```

When a node teleports (respawn, warp), call `reset_physics_interpolation()` on it immediately after moving so Godot doesn't interpolate through the jump.

The same caveat applies to overstep-based camera-smoothing systems placed in the `BeforeFixedMainLoop` / `AfterFixedMainLoop` sets, and to `bevy_gizmos`' fixed gizmo-context -- both are untested and unsupported under Godot-owned physics. In practice this is largely moot since godot-bevy renders through Godot rather than `bevy_render`.

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
- An ECS system writing the **same axis** the animation drives will win the conflict (Bevy-wins);
  let each side own different axes, or move that ECS logic off the animated axis. ECS writing
  *other* axes is fine -- co-authorship is per-axis (see above).
- For rotation specifically, only one side should author it (rotation is whole, not per-axis)
