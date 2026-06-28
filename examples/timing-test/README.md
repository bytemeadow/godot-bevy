# Timing Test Example

This example demonstrates the timing behavior of godot-bevy integration, showing how Bevy schedules run within Godot's frame callbacks.

> 📖 **For detailed information about timing and schedules**, see the [Frame Execution Model](https://bytemeadow.github.io/godot-bevy-book?page=timing/index.html) book chapter.

## What This Example Tests

This example helps you understand:

- **When different Bevy schedules execute** (First, PreUpdate, Update, FixedUpdate, PostUpdate, Last)
- **How often each schedule runs** relative to Godot's frame rate
- **The relationship between visual frames and physics frames**
- **How FixedUpdate runs on Godot's physics clock**

## What You'll See

The example logs periodic messages showing:

```
🚀 Timing Test Started!
📊 Watching for timing behavior...
⏱️  prefix (First/PreUpdate) + FixedUpdate run in physics_process; suffix (Update/PostUpdate/Last) runs in process
🔍 DEBUG: First Schedule #60: prefix_runs: 60, Time: 1.00s
📺 First Schedule Run #120: Time: 2.00s (First runs in the physics_process prefix)
🔄 PreUpdate at 3.00s (prefix_runs: 180)
📋 Update running at 4.00s (Update runs in the process suffix)
⚡ FixedUpdate #60: physics_process_calls: 60, Godot delta: 0.0167s
📤 PostUpdate running at 5.00s (PostUpdate runs in the process suffix)
🏁 Last Schedule: Update runs: 360, Fixed updates: 360, Time: 6.00s
```

## Key Observations

### Frame Rates
- **Visual frames**: Run at your display's refresh rate (60-144 FPS)
- **Physics frames**: Run at Godot's physics tick rate (default 60 Hz)
- **FixedUpdate**: Ticks on Godot's authoritative physics clock — one clock, not two. `Res<Time>.delta_secs()` in a `FixedUpdate` system is Godot's physics delta.

### Schedule Usage Guidelines

```rust
// For general game logic, UI, rendering - runs in visual frames (_process)
app.add_systems(Update, gameplay_system);

// For physics/deterministic logic - runs on Godot's physics clock (_physics_process)
app.add_systems(FixedUpdate, physics_simulation);
```

## Running This Example

1. **Build**: `cargo build`
2. **Run**: You can either:
    1. Open the Godot project and run the scene
    1. Run: `cargo run`. NOTE: This requires the Godot binary, which we attempt
       to locate either through your environment's path or by searching common
       locations. If this doesn't work, update your path to include Godot. If
       this fails for other reasons, it may be because your version of Godot
       is different than the one the example was built with, in that case,
       try opening the Godot project first.
3. **Observe**: Watch the console output for timing patterns

## Understanding the Output

- **High visual frame rates** (100+ FPS) are normal and indicate good performance
- **FixedUpdate** may run 0, 1, or 2+ times per visual frame depending on the physics-to-display rate ratio
- **Timing consistency** shows that each schedule runs when expected

This example is particularly useful for:
- Understanding when to use each Bevy schedule
- Debugging timing-related issues
- Verifying frame rate expectations
- Learning about fixed timestep vs variable timestep systems

### Scheduling Relationships

godot-bevy splits Bevy's `Main` schedule across Godot's two callbacks: the prefix (`First`, `PreUpdate`, `StateTransition`) and `FixedMain` run in `_physics_process`; the suffix (`Update`, `PostUpdate`, `Last`) runs in `_process`. This gives native Bevy ordering across a render frame:

- `PreUpdate` runs before the fixed steps, so input and Godot→ECS transform reads are visible to `FixedUpdate` the same frame.
- Transform changes written in `FixedLast` are read back into ECS by the next render frame's `PreUpdate`.
