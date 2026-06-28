# Bevy vs Godot Input

godot-bevy offers two distinct approaches to handling input: Bevy's built-in input system and godot-bevy's bridged Godot input system. Understanding when to use each is crucial for building the right game experience.

## Two Input Systems

### Bevy's Built-in Input

Use Bevy's standard input resources for simple, direct input handling:

```rust
fn movement_system(
    keys: Res<ButtonInput<KeyCode>>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    for mut transform in query.iter_mut() {
        if keys.pressed(KeyCode::ArrowLeft) {
            transform.translation.x -= 200.0;
        }
        if keys.pressed(KeyCode::ArrowRight) {
            transform.translation.x += 200.0;
        }
    }
}
```

### godot-bevy's Bridged Input

Use godot-bevy's event-based system for more advanced input handling:

```rust
fn movement_system(
    mut events: MessageReader<ActionInput>,
    mut query: Query<&mut Transform, With<Player>>,
) {
    for event in events.read() {
        if event.pressed {
            match event.action.as_str() {
                "move_left" => {
                    // Handle left movement
                }
                "move_right" => {
                    // Handle right movement
                }
                _ => {}
            }
        }
    }
}
```

## When to Use Each System

### 🚀 Use Bevy Input For:

**Simple desktop games and rapid prototyping**

✅ **Advantages:**
- **Zero setup** - works immediately
- **State-based queries** - easy "is key held?" checks
- **Rich API** - `just_pressed()`, `pressed()`, `just_released()`
- **Direct and fast** - no event processing overhead
- **Familiar** - standard Bevy patterns

❌ **Limitations:**
- **Desktop-focused** - limited mobile/console support
- **Hardcoded keys** - players can't remap controls
- **No Godot integration** - can't use input maps

**Example use cases:**
- Game jams and prototypes
- Desktop-only games
- Simple control schemes
- Internal tools

### 🎮 Use godot-bevy Input For:

**Production games and cross-platform releases**

✅ **Advantages:**
- **Cross-platform** - desktop, mobile, console support
- **User remappable** - integrates with Godot's input maps
- **Touch support** - native mobile input handling
- **Action-based** - semantic controls ("jump" vs "spacebar")
- **Flexible** - supports complex input schemes

❌ **Trade-offs:**
- **Event-based** - requires more complex state tracking
- **Setup required** - need to define input maps in Godot
- **More complex** - steeper learning curve

**Example use cases:**
- Commercial releases
- Mobile games
- Console ports
- Games with complex controls

## Input Event Processing

godot-bevy processes Godot's dual input system intelligently to prevent duplicate events:

- **Normal Input Events**: Generate `ActionInput` events for mapped keys/buttons
- **Unhandled Input Events**: Generate raw `GodotKeyboardInput`, `GodotMouseButtonInput`, etc. for unmapped inputs

This ensures:
- ✅ **No duplicate events** - each physical input generates exactly one event
- ✅ **Proper input flow** - mapped inputs become actions, unmapped inputs become raw events
- ✅ **Clean event streams** - predictable, non-redundant event processing

```rust
// For a key mapped to "jump" action in Godot's Input Map:
// ✅ Generates ONE ActionInput { action: "jump", pressed: true }
// ❌ Does NOT generate duplicate GodotKeyboardInput events

// For an unmapped key (e.g., 'Q' with no action mapping):
// ✅ Generates ONE GodotKeyboardInput { keycode: Q, pressed: true }
// ❌ Does NOT generate ActionInput events
```

## Available Input Events

godot-bevy provides several input event types:

### ActionInput
The most important event type - maps to Godot's input actions:

```rust
fn handle_actions(mut events: MessageReader<ActionInput>) {
    for event in events.read() {
        println!("Action: {}, Pressed: {}, Strength: {}", 
                 event.action, event.pressed, event.strength);
    }
}
```

### GodotKeyboardInput
Direct keyboard events:

```rust
fn handle_keyboard(mut events: MessageReader<GodotKeyboardInput>) {
    for event in events.read() {
        if event.pressed && event.keycode == Key::SPACE {
            println!("Space pressed!");
        }
    }
}
```

### GodotMouseButtonInput
Mouse button events:

```rust
fn handle_mouse(mut events: MessageReader<GodotMouseButtonInput>) {
    for event in events.read() {
        println!("Mouse button: {:?} at {:?}", 
                 event.button, event.position);
    }
}
```

### GodotMouseMotion
Mouse movement events:

```rust
fn handle_mouse_motion(mut events: MessageReader<GodotMouseMotion>) {
    for event in events.read() {
        println!("Mouse moved by: {:?}", event.delta);
    }
}
```

## Gamepad input

Three ways to read a gamepad, depending on what you want:

- **Gameplay (recommended)** -- bind the action in Godot's Input Map and read it through `GodotActions` (`actions.pressed("jump")`, `actions.strength("accelerate")`). Works in `Update` and `FixedUpdate`, and reuses Godot's controller remapping.
- **Bevy-native `Query<&Gamepad>`** -- the default `bevy_gamepad` feature pulls in Bevy's gilrs backend, which populates the `Gamepad` entity directly from the OS. No godot-bevy code involved; use it exactly as in any Bevy app.
- **Raw events + Godot device id** -- the `GamepadButtonInput` / `GamepadAxisInput` messages carry the Godot device id and work on every platform, including WASM (where gilrs isn't available).

Don't feed Godot's gamepad into Bevy's `Gamepad` entity yourself while `bevy_gamepad` is on -- you'd get two entities per controller and doubled input. gilrs already owns that path.

## Quick Reference

| Feature | Bevy Input | godot-bevy Input |
|---------|------------|------------------|
| Setup complexity | None | Moderate |
| Cross-platform | Limited | Full |
| User remapping | No | Yes |
| Touch support | No | Yes |
| State queries | Easy | Manual tracking |
| Performance | Fastest | Fast |
| Godot integration | None | Full |

## Choosing Your Approach

### Start with Bevy Input if:
- Building a prototype or game jam entry
- Targeting desktop only
- Using simple controls
- Want immediate results

### Use godot-bevy Input if:
- Building for release
- Need cross-platform support  
- Want user-configurable controls
- Using complex input schemes
- Targeting mobile/console

## Mixing Both Systems

You can use both systems in the same project:

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_systems(Update, (
        // Debug controls with Bevy input
        debug_controls,
        // Game controls with godot-bevy input
        game_controls,
    ));
}

fn debug_controls(keys: Res<ButtonInput<KeyCode>>) {
    if keys.just_pressed(KeyCode::F1) {
        // Toggle debug overlay
    }
}

fn game_controls(mut events: MessageReader<ActionInput>) {
    for event in events.read() {
        // Handle game actions
    }
}
```

This gives you the best of both worlds: simple debug controls and flexible game controls.

## Fixed-timestep input under Godot-owned physics

godot-bevy drives Bevy's `RunFixedMainLoop` schedule from Godot's `_physics_process`. Third-party input plugins that hook `BeforeFixedMainLoop` / `AfterFixedMainLoop` -- e.g. [`leafwing-input-manager`](https://github.com/Leafwing-Studios/leafwing-input-manager) -- work out of the box.

### `ButtonInput` in `FixedUpdate`

Keyboard and mouse `ButtonInput` edges (`just_pressed`/`just_released`) work in `Update` and `FixedUpdate`. The bridge emits Bevy `KeyboardInput` events; `keyboard_input_system` in `PreUpdate` populates the resource. Because `PreUpdate` runs in the physics-process prefix -- before the fixed steps -- edges are already set by the time `FixedUpdate` runs for the frame.

Two caveats, both because `just_pressed` is a one-render-frame edge (cleared each frame by `keyboard_input_system` in `PreUpdate`):

- **More physics than display** (e.g. 60 Hz display / 120 Hz physics): when N steps fire in one render frame, `just_pressed` is `true` in every one of those N `FixedUpdate` calls.
- **More display than physics** (e.g. 144 Hz display / 60 Hz physics): a render frame can run **zero** physics steps. If the edge lands on a step-less frame, `FixedUpdate` never sees it -- by the next frame the edge is already cleared. This is the same caveat stock Bevy has with raw `ButtonInput` in `FixedUpdate`.

`GodotActions` (below) uses Godot's per-tick edge state and has neither issue -- it is the right tool for fixed-rate gameplay input.

```rust
// Visible in FixedUpdate -- but only when a physics step runs on the edge's
// render frame (see the zero-step caveat above); use GodotActions for fixed-rate.
app.add_systems(FixedUpdate, |keys: Res<ButtonInput<KeyCode>>| {
    if keys.just_pressed(KeyCode::Space) { /* may be missed on a 0-step frame */ }
});

// Also works: edges in Update (once per render frame, unambiguous)
app.add_systems(Update, |keys: Res<ButtonInput<KeyCode>>| {
    if keys.just_pressed(KeyCode::Space) { /* correct */ }
});

// Preferred for fixed-rate gameplay: GodotActions is clock-aware and per-tick
app.add_systems(FixedUpdate, |actions: Res<GodotActions>| {
    if actions.just_pressed("jump") { /* correct, exactly once per physics tick */ }
});
```

For fixed-rate gameplay input, `GodotActions` is the preferred tool -- it tracks per-tick edges independently across the process and physics clocks.

### GodotActions

`GodotActions` lets you read Godot's `InputMap` actions identically in `Update`, `FixedUpdate`, or a helper shared by both. The resource tracks the executing clock via an active-clock flag set by the schedule driver, so the same `Res<GodotActions>` read returns the correct snapshot for whichever schedule is running.

Opt in per-app:

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotActionsPlugin);
}
```

Same function, both schedules -- no change needed:

```rust
fn jump_system(actions: Res<GodotActions>) {
    if actions.just_pressed("jump") { ... }
}

app.add_systems(Update, jump_system);
app.add_systems(FixedUpdate, jump_system); // same fn, correct in both
```

For frequently-called code, use a typed `Action` handle instead of `&str` -- no string hash per call, and typo-resistant:

```rust
let jump = Action::new("jump"); // construct once, store in a Resource for cross-frame reuse
if actions.just_pressed(&jump) { ... }
```

The `&str` overload warns once in debug if the action isn't in the `InputMap`, so typos surface instead of silently returning `false`.

All accessors:

```rust
actions.pressed("run")
actions.just_pressed("jump")
actions.just_released("attack")
actions.strength("fire")            // deadzoned [0.0, 1.0]
actions.raw_strength("fire")        // before deadzone
actions.axis("move_left", "move_right")                              // [-1.0, 1.0]
actions.vector("move_left", "move_right", "move_up", "move_down")   // Vec2
```

If you add custom `First` systems that read `GodotActions`, order them after the poll:

```rust
app.add_systems(First, my_system.after(GodotInputSet));
```

**Limitations:**

- **Physics `just_pressed` lags one tick.** Godot stamps physics action edges with a `+1` frame offset -- the same lag as `is_action_just_pressed` in GDScript's `_physics_process`. Intentional; matches GDScript.
- **Action set is cached on first poll.** Actions added to the `InputMap` after startup aren't picked up.
- **`axis`/`vector` use per-action InputMap deadzones.** `vector()` does not replicate Godot's circular `get_vector` deadzone -- each action's deadzone is applied independently.

### Third-party: `bevy_enhanced_input`

[`bevy_enhanced_input`](https://github.com/projectharmonia/bevy_enhanced_input) supports fixed-rate input contexts natively and derives its own edge state per physics step:

```rust
app.add_input_context_to::<FixedPreUpdate, MyInputContext>();
```

It covers button, key, and gamepad bindings. Use it for Bevy-centric action definitions; use `GodotActions` for Godot InputMap integration.

**Analog caveat:** mouse-motion and scroll-wheel bindings consumed in `FixedPreUpdate` draw from accumulated-delta resources that are frame-scoped. When N physics steps run in one render frame, each step reads the full accumulated delta -- multiplying the effect by N. Consume analog inputs at render rate or divide by the physics step count.

### Anchor cadence

`BeforeFixedMainLoop` and `AfterFixedMainLoop` run **once per physics step**, not once per render frame -- N times when N steps fire in a single frame. Idempotent per-step work (e.g. leafwing's input-buffer swap) is fine; systems that assume once-per-frame batch accumulation will over-apply.

## Migration

### Phase 1 input changes (keyboard bridge + type renames)

Three things changed in this release; each may require a small update.

**1. keyboard edges now work in `Update`**

The bridge previously pressed `ButtonInput<KeyCode>` directly, which meant `just_pressed`/`just_released` were always false -- they were cleared before `Update` ever ran. The bridge now emits Bevy `KeyboardInput` events instead, so Bevy's own `PreUpdate` systems populate the resource correctly. `pressed()`, `just_pressed()`, and `just_released()` all work in `Update`. No code change needed -- this is a fix.

**2. third-party `EventReader<KeyboardInput>` consumers now receive events**

If you have egui text fields, an accessibility plugin, a key-rebinding UI, or a hand-rolled Bevy keyboard bridge, they will start receiving `KeyboardInput` events for the first time. Dormant text widgets can activate; a hand-rolled bridge that also reads raw Godot events may double-handle. Check for anything in your dependency tree that reads `EventReader<bevy_input::keyboard::KeyboardInput>` and confirm the behavior is correct.

**3. godot-bevy message types are renamed**

The types that shadowed Bevy's same-named types are now `Godot`-prefixed. Update your imports:

| old | new |
|-----|-----|
| `KeyboardInput` (godot-bevy) | `GodotKeyboardInput` |
| `MouseButtonInput` (godot-bevy) | `GodotMouseButtonInput` |
| `MouseMotion` (godot-bevy) | `GodotMouseMotion` |
| `MouseButton` (godot-bevy) | `GodotMouseButton` |

```rust
// before
use godot_bevy::plugins::input::{KeyboardInput, MouseButtonInput, MouseMotion, MouseButton};

// after
use godot_bevy::plugins::input::{
    GodotKeyboardInput, GodotMouseButtonInput, GodotMouseMotion, GodotMouseButton,
};
```

### Phase 2 input changes (GodotActions)

`GodotActionsPlugin` is additive -- nothing existing breaks. If you were accumulating `ActionInput` messages to track held state or detect first-press, replace with `Res<GodotActions>`:

```rust
// before -- message-based, Update only
fn my_system(mut reader: MessageReader<ActionInput>) {
    for event in reader.read() {
        if event.action == "jump" && event.pressed { ... }
    }
}

// after -- works in Update and FixedUpdate
fn my_system(actions: Res<GodotActions>) {
    if actions.just_pressed("jump") { ... }
}
```

`MessageReader<ActionInput>` continues to work; removal is a future phase.

## Troubleshooting

### Duplicate Events (Fixed in v0.7.0+)

If you're seeing duplicate `ActionInput` events for the same key press, you may be using an older version of godot-bevy. This was fixed in version 0.7.0 through improved input event processing.

**Symptoms:**
```rust
// Old behavior (before v0.7.0):
🎮 Action: 'jump' pressed    // First event
🎮 Action: 'jump' pressed    // Duplicate event (unwanted)
```

**Solution:** Update to godot-bevy v0.7.0 or later where input processing was improved to eliminate duplicates.

### Mouse Events Only on Movement

`GodotMouseMotion` events are only generated when the mouse actually moves. If you need continuous mouse position tracking, consider using Godot's `Input.get_global_mouse_position()` in a system that runs every frame.
