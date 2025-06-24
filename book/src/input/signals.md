# Signal Handling

Godot signals are a core communication mechanism in the Godot engine, allowing nodes to notify other parts of the game when events occur. godot-bevy bridges Godot signals into Bevy's event system, enabling ECS systems to respond to UI interactions, collision events, and other Godot-specific events.

## How Signal Bridging Works

When you connect a Godot signal through godot-bevy, the signal is automatically converted into a `GodotSignal` event that can be read by Bevy systems using `EventReader<GodotSignal>`. This includes support for signals with arguments - the signal arguments are preserved and passed along with the event.

## Basic Signal Connection

To connect to a Godot signal, use the `GodotSignals` resource to connect to any node's signal:

```rust
use bevy::prelude::*;
use godot_bevy::prelude::*;

fn connect_signals(
    mut scene_tree: SceneTreeRef,
    signals: GodotSignals,
) {
    if let Some(root) = scene_tree.get().get_root() {
        if let Some(button) = root.try_get_node_as::<Button>("UI/MyButton") {
            let mut handle = GodotNodeHandle::from_instance_id(button.instance_id());
            signals.connect(&mut handle, "pressed");
        }
    }
}
```

## Reading Signal Events

Once connected, signals become `GodotSignal` events that you can read in any Bevy system:

```rust
fn handle_signals(mut signal_events: EventReader<GodotSignal>) {
    for signal in signal_events.read() {
        match signal.name.as_str() {
            "pressed" => {
                println!("Button was pressed!");
            }
            "toggled" => {
                println!("Toggle button changed state");
            }
            _ => {}
        }
    }
}
```

## Signals with Arguments

Many Godot signals carry arguments that provide additional context about the event. godot-bevy preserves these arguments and makes them available through the `arguments` field:

```rust
fn handle_input_signals(mut signal_events: EventReader<GodotSignal>) {
    for signal in signal_events.read() {
        if signal.name == "input_event" {
            println!("Received input_event signal with {} arguments", signal.arguments.len());

            // CollisionObject2D.input_event has 3 arguments: viewport, event, shape_idx
            if signal.arguments.len() >= 2 {
                // The second argument is the InputEvent
                let event_arg = &signal.arguments[1];

                // Parse the event argument to determine event type
                if event_arg.value.contains("InputEventMouseButton") {
                    println!("Mouse button event detected!");

                    if event_arg.value.contains("pressed=true") {
                        if event_arg.value.contains("button_index=1") {
                            println!("Left mouse button clicked!");
                        } else if event_arg.value.contains("button_index=2") {
                            println!("Right mouse button clicked!");
                        }
                    }
                } else if event_arg.value.contains("InputEventMouseMotion") {
                    println!("Mouse motion over area");
                }
            }
        }
    }
}
```

## Signal Arguments Structure

Signal arguments are provided as a `Vec<SignalArgument>` where each `SignalArgument` has:

- `value`: A `String` representation of the argument's value
- Additional metadata about the argument type (implementation details may vary)

For complex signal arguments like `InputEvent`, you'll typically need to parse the `value` string to extract the information you need, as shown in the examples above.

## Common Signal Patterns

### UI Signals
```rust
// Button pressed
if signal.name == "pressed" {
    println!("Button clicked!");
}

// CheckBox toggled
if signal.name == "toggled" && signal.arguments.len() > 0 {
    let pressed = signal.arguments[0].value.contains("true");
    println!("Checkbox is now: {}", if pressed { "checked" } else { "unchecked" });
}

// LineEdit text changed
if signal.name == "text_changed" && signal.arguments.len() > 0 {
    println!("Text changed to: {}", signal.arguments[0].value);
}
```

### Physics Signals

For physics-related events like collisions, godot-bevy provides dedicated resources that are more efficient than signals. Instead of connecting to physics signals, use the `Collisions` resource:

```rust
// Instead of using signals for collision detection, use the Collisions resource
fn check_player_death(
    mut player: Query<(&mut GodotNodeHandle, &Collisions), With<Player>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if let Ok((mut player, collisions)) = player.single_mut() {
        if collisions.colliding().is_empty() {
            return;
        }

        player.get::<Node2D>().set_visible(false);
        next_state.set(GameState::GameOver);
    }
}
```

The `Collisions` resource provides direct access to collision state without the overhead of signal processing, making it ideal for gameplay-critical physics events.

For non-gameplay physics events that need custom data, signals are still appropriate:
```rust
// Custom physics events that carry additional data
if signal.name == "projectile_hit" && signal.arguments.len() > 0 {
    let damage = signal.arguments[0].value.parse::<f32>().unwrap_or(0.0);
    println!("Projectile hit for {} damage", damage);
}
```

## Best Practices

### 1. **One-time Connection Setup**
Use a resource or local state to ensure signals are connected only once:

```rust
#[derive(Resource, Default)]
struct SignalConnectionState {
    connected: bool,
}

fn setup_signals(
    mut state: ResMut<SignalConnectionState>,
    // ... other parameters
) {
    if !state.connected {
        // Connect signals
        state.connected = true;
    }
}
```

### 2. **Signal Name Matching**
Use string matching or consider creating an enum for frequently used signals:

```rust
#[derive(Debug, PartialEq)]
enum GameSignal {
    ButtonPressed,
    PlayerHit,
    AreaEntered,
    Unknown(String),
}

impl From<&str> for GameSignal {
    fn from(name: &str) -> Self {
        match name {
            "pressed" => Self::ButtonPressed,
            "player_hit" => Self::PlayerHit,
            "body_entered" => Self::AreaEntered,
            other => Self::Unknown(other.to_string()),
        }
    }
}
```
