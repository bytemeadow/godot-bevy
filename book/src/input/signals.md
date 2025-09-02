# Signal Handling

Godot signals are a core communication mechanism in the Godot engine, allowing nodes to notify other parts of the game when events occur. godot-bevy bridges Godot signals into Bevy's event system, enabling ECS systems to respond to UI interactions, collision events, and other Godot-specific events.

## How Signal Bridging Works

When you connect a Godot signal through godot-bevy, the signal is automatically converted into a `GodotSignal` event that can be read by Bevy systems using `EventReader<GodotSignal>`. The signal source node and arguments are preserved and passed along with the event.

## Basic Signal Connection

To connect to a Godot signal, use the `GodotSignals` system parameter to connect to any node's signal:

```rust
use bevy::prelude::*;
use godot_bevy::prelude::*;

fn connect_button_signals(
    mut buttons: Query<&mut GodotNodeHandle, With<Button>>,
    signals: GodotSignals,
) {
    for mut handle in buttons.iter_mut() {
        signals.connect(&mut handle, "pressed");
        signals.connect(&mut handle, "mouse_entered");
        signals.connect(&mut handle, "mouse_exited");
    }
}
```

You can also connect multiple signals at once:

```rust
fn connect_area_signals(
    mut areas: Query<&mut GodotNodeHandle, With<DetectionArea>>,
    signals: GodotSignals,
) {
    for mut handle in areas.iter_mut() {
        signals.connect_many(&mut handle, &[
            "body_entered",
            "body_exited",
            "area_entered",
            "area_exited",
        ]);
    }
}
```

## Deferred Signal Connections

When spawning entities that will have Godot nodes, you can queue signal connections to be made once the `GodotNodeHandle` becomes available:

```rust
fn spawn_interactive_button(mut commands: Commands) {
    commands.spawn((
        ButtonBundle::default(),
        // Signal will be connected when GodotNodeHandle is available
        DeferredSignalConnections::single("pressed"),
    ));
}

fn spawn_detection_area(mut commands: Commands) {
    commands.spawn((
        AreaBundle::default(),
        DeferredSignalConnections::new(vec![
            "body_entered".into(),
            "body_exited".into(),
        ]),
    ));
}
```

## Reading Signal Events with Extension Syntax

godot-bevy provides a clean, chainable syntax for handling signals through the `GodotSignalReaderExt` trait:

```rust
use godot_bevy::prelude::*;

fn handle_button_presses(mut events: EventReader<GodotSignal>) {
    events.handle_signal("pressed").any(|signal| {
        println!("Button pressed!");
        
        // Access the source node directly
        let mut button = signal.source_node.get::<Button>();
        button.set_text("Clicked!".into());
    });
}
```

### Handling Multiple Signal Types

```rust
fn handle_ui_signals(mut events: EventReader<GodotSignal>) {
    // Handle multiple signal types at once
    events.handle_signals(&["pressed", "released", "toggled"])
        .any(|signal| {
            println!("UI interaction: {}", signal.signal_name);
        });
    
    // Custom predicate for filtering
    events.handle_matching(|signal| {
        signal.signal_name.starts_with("custom_") && 
        !signal.argument_strings.is_empty()
    }).any(|signal| {
        println!("Custom signal with args: {}", signal.signal_name);
    });
}
```

### Signal Routing by Node

You can handle signals based on which node they came from:

```rust
#[derive(Resource)]
struct MenuButtons {
    start_button: Option<GodotNodeHandle>,
    quit_button: Option<GodotNodeHandle>,
    settings_button: Option<GodotNodeHandle>,
}

fn handle_menu_buttons(
    menu: Res<MenuButtons>,
    mut events: EventReader<GodotSignal>,
    mut app_state: ResMut<NextState<GameState>>,
) {
    if let (Some(start), Some(quit), Some(settings)) = 
        (&menu.start_button, &menu.quit_button, &menu.settings_button) 
    {
        events.handle_signal("pressed")
            .from_node(start, |_| {
                app_state.set(GameState::Playing);
            })
            .from_node(quit, |_| {
                std::process::exit(0);
            })
            .from_node(settings, |_| {
                app_state.set(GameState::Settings);
            });
    }
}
```

## Direct Signal Handling

For simpler cases, you can read signals directly without the extension syntax:

```rust
fn handle_signals_directly(mut events: EventReader<GodotSignal>) {
    for signal in events.read() {
        if signal.is_from("pressed") {
            println!("Button pressed!");
        }
        
        if signal.is_from("text_changed") {
            // Signal arguments are provided as debug strings
            if let Some(new_text) = signal.get_arg_string(0) {
                println!("Text changed to: {}", new_text);
            }
        }
    }
}
```

## Finding Entity Owners

When you need to know which Bevy entity owns a signal's source node:

```rust
#[derive(Component)]
struct MyButton {
    click_count: u32,
}

fn handle_button_clicks(
    mut events: EventReader<GodotSignal>,
    mut buttons: Query<(Entity, &GodotNodeHandle, &mut MyButton)>,
) {
    for signal in events.read() {
        if signal.is_from("pressed") {
            // Find which entity owns this signal's source node
            for (entity, handle, mut button) in buttons.iter_mut() {
                if *handle == signal.source_node {
                    button.click_count += 1;
                    println!("Entity {:?} clicked {} times", entity, button.click_count);
                    break;
                }
            }
        }
    }
}
```

Or use the helper method:

```rust
fn handle_button_with_helper(
    mut events: EventReader<GodotSignal>,
    node_query: Query<(Entity, &GodotNodeHandle)>,
    mut buttons: Query<&mut MyButton>,
) {
    for signal in events.read() {
        if signal.is_from("pressed") {
            if let Some(entity) = signal.find_entity(&node_query) {
                if let Ok(mut button) = buttons.get_mut(entity) {
                    button.click_count += 1;
                }
            }
        }
    }
}
```

## Custom Extension Traits

You can create your own extension traits for domain-specific signal handling:

```rust
trait GameSignalExt {
    fn handle_combat_signals(&mut self) -> SignalMatcher<'_>;
    fn handle_ui_signals(&mut self) -> SignalMatcher<'_>;
}

impl GameSignalExt for EventReader<'_, '_, GodotSignal> {
    fn handle_combat_signals(&mut self) -> SignalMatcher<'_> {
        self.handle_matching(|signal| {
            matches!(signal.signal_name.as_str(), 
                "attack" | "defend" | "cast_spell" | "take_damage")
        })
    }
    
    fn handle_ui_signals(&mut self) -> SignalMatcher<'_> {
        self.handle_signals(&["pressed", "released", "toggled", "value_changed"])
    }
}

// Use your custom extension
fn game_system(mut events: EventReader<GodotSignal>) {
    events.handle_combat_signals().any(|signal| {
        println!("Combat event: {}", signal.signal_name);
    });
}
```

## Common Signal Patterns

### UI Signals
```rust
fn handle_ui_interactions(mut events: EventReader<GodotSignal>) {
    events.handle_signal("pressed").any(|_| {
        println!("Button clicked!");
    });
    
    events.handle_signal("toggled").any(|signal| {
        // Check button state if needed
        let button = signal.source_node.get::<CheckBox>();
        println!("Checkbox is now: {}", button.is_pressed());
    });
    
    events.handle_signal("text_changed").any(|signal| {
        if let Some(text) = signal.get_arg_string(0) {
            println!("Text changed to: {}", text);
        }
    });
}
```

### Physics Signals

For physics-related events like collisions, godot-bevy provides the dedicated `Collisions` resource that is more efficient than signals. Use signals only for custom physics events:

```rust
// For standard collisions, use the Collisions resource (more efficient)
fn check_collisions(
    mut query: Query<(&mut GodotNodeHandle, &Collisions), With<Player>>,
) {
    if let Ok((mut player, collisions)) = query.single_mut() {
        if !collisions.colliding().is_empty() {
            player.get::<Node2D>().set_visible(false);
        }
    }
}

// For custom physics signals with additional data
fn handle_custom_physics(mut events: EventReader<GodotSignal>) {
    events.handle_signal("projectile_hit").any(|signal| {
        if let Some(damage_str) = signal.get_arg_string(0) {
            println!("Projectile hit for damage: {}", damage_str);
        }
    });
}
```

## Best Practices

### 1. **Use Extension Syntax for Cleaner Code**
The extension syntax provides cleaner, more maintainable code:

```rust
// Clean and declarative
events.handle_signal("pressed")
    .from_node(&start_button, |_| { /* start game */ })
    .from_node(&quit_button, |_| { /* quit game */ });

// Instead of nested if statements
for signal in events.read() {
    if signal.is_from("pressed") {
        if signal.source_node == start_button {
            // start game
        } else if signal.source_node == quit_button {
            // quit game
        }
    }
}
```

### 2. **One-time Connection Setup**
Ensure signals are connected only once:

```rust
#[derive(Resource, Default)]
struct SignalConnectionState {
    connected: bool,
}

fn setup_signals(
    mut state: ResMut<SignalConnectionState>,
    mut handles: Query<&mut GodotNodeHandle>,
    signals: GodotSignals,
) {
    if !state.connected {
        for mut handle in handles.iter_mut() {
            signals.connect(&mut handle, "pressed");
        }
        state.connected = true;
    }
}
```

### 3. **Use Deferred Connections for Spawned Entities**
When spawning entities, use `DeferredSignalConnections`:

```rust
commands.spawn((
    MyBundle::default(),
    DeferredSignalConnections::single("pressed"),
));
```

### 4. **Create Custom Matchers for Complex Logic**
For complex signal routing, create custom signal matchers:

```rust
fn create_custom_matcher<'a>(
    events: &mut EventReader<GodotSignal>,
    important_nodes: &[GodotNodeHandle],
) -> SignalMatcher<'a> {
    SignalMatcher::from_signals(
        events.read()
            .filter(|s| {
                s.is_from("pressed") && 
                important_nodes.contains(&s.source_node)
            })
            .collect()
    )
}
```

## Signal API Reference

### GodotSignal Fields
- `signal_name: String` - The name of the signal that was emitted
- `source_node: GodotNodeHandle` - The Godot node that emitted the signal
- `argument_strings: Vec<String>` - String representations of signal arguments

### GodotSignal Methods
- `is_from(&str) -> bool` - Check if signal has a specific name
- `is_from_node(&GodotNodeHandle) -> bool` - Check if signal came from a specific node
- `is_from_node_signal(&GodotNodeHandle, &str) -> bool` - Check both node and signal name
- `get_arg_string(index) -> Option<&str>` - Get argument as string by index
- `find_entity(&Query) -> Option<Entity>` - Find the entity that owns the source node

### GodotSignalReaderExt Methods
- `handle_signal(&str) -> SignalMatcher` - Filter to specific signal name
- `handle_signals(&[&str]) -> SignalMatcher` - Filter to multiple signal names
- `handle_matching(predicate) -> SignalMatcher` - Filter with custom predicate
- `handle_all() -> SignalMatcher` - Get all signals as a matcher

### SignalMatcher Methods
- `from_node(node, handler)` - Handle signals from specific node
- `from_any_node(nodes, handler)` - Handle signals from any of the nodes
- `matching(predicate)` - Additional filtering
- `any(handler)` - Handle all remaining signals
- `first(handler)` - Handle only the first signal
- `count()` - Get number of signals
- `is_empty()` - Check if matcher has signals
- `iter()` - Iterate over signals