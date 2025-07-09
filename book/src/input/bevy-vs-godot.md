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
    mut events: EventReader<ActionInput>,
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
- **Unhandled Input Events**: Generate raw `KeyboardInput`, `MouseButtonInput`, etc. for unmapped inputs

This ensures:
- ✅ **No duplicate events** - each physical input generates exactly one event
- ✅ **Proper input flow** - mapped inputs become actions, unmapped inputs become raw events
- ✅ **Clean event streams** - predictable, non-redundant event processing

```rust
// For a key mapped to "jump" action in Godot's Input Map:
// ✅ Generates ONE ActionInput { action: "jump", pressed: true }
// ❌ Does NOT generate duplicate KeyboardInput events

// For an unmapped key (e.g., 'Q' with no action mapping):
// ✅ Generates ONE KeyboardInput { keycode: Q, pressed: true }
// ❌ Does NOT generate ActionInput events
```

## Available Input Events

godot-bevy provides several input event types:

### ActionInput
The most important event type - maps to Godot's input actions:

```rust
fn handle_actions(mut events: EventReader<ActionInput>) {
    for event in events.read() {
        println!("Action: {}, Pressed: {}, Strength: {}", 
                 event.action, event.pressed, event.strength);
    }
}
```

### KeyboardInput
Direct keyboard events:

```rust
fn handle_keyboard(mut events: EventReader<KeyboardInput>) {
    for event in events.read() {
        if event.pressed && event.keycode == Key::SPACE {
            println!("Space pressed!");
        }
    }
}
```

### MouseButtonInput
Mouse button events:

```rust
fn handle_mouse(mut events: EventReader<MouseButtonInput>) {
    for event in events.read() {
        println!("Mouse button: {:?} at {:?}", 
                 event.button_index, event.position);
    }
}
```

### MouseMotion
Mouse movement events:

```rust
fn handle_mouse_motion(mut events: EventReader<MouseMotion>) {
    for event in events.read() {
        println!("Mouse moved by: {:?}", event.relative);
    }
}
```

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

fn game_controls(mut events: EventReader<ActionInput>) {
    for event in events.read() {
        // Handle game actions
    }
}
```

This gives you the best of both worlds: simple debug controls and flexible game controls.

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

`MouseMotion` events are only generated when the mouse actually moves. If you need continuous mouse position tracking, consider using Godot's `Input.get_global_mouse_position()` in a system that runs every frame.
