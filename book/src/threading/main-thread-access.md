# Thread Safety and Godot APIs

Godot's APIs are not thread-safe and must be called exclusively from the main thread. This creates an important constraint when working with Bevy's multi-threaded ECS, where systems typically run in parallel across multiple threads.

## The Main Thread Requirement

Any system that interacts with Godot APIs—such as calling methods on `Node`, accessing scene tree properties, or manipulating UI elements—must run on the main thread. This includes:

- Scene tree operations (`add_child`, `queue_free`, etc.)
- Transform modifications on Godot nodes
- UI updates (setting text, visibility, etc.)
- Audio playback controls
- Input handling via Godot's `Input` singleton
- File I/O operations through Godot's resource system

## The `#[godot_main_thread]` Macro

The `#[godot_main_thread]` attribute macro provides a clean way to mark systems that require main thread execution:

```rust
use godot_bevy::prelude::*;

#[godot_main_thread]
fn update_ui_labels(
    mut query: Query<&mut GodotNodeHandle, With<PlayerStats>>,
    stats: Res<GameStats>,
) {
    for mut handle in query.iter_mut() {
        if let Some(mut label) = handle.try_get::<Label>() {
            label.set_text(&format!("Score: {}", stats.score));
        }
    }
}
```

The macro automatically adds a `NonSend<MainThreadMarker>` parameter to the system, which forces Bevy to schedule it on the main thread. This approach requires no imports and keeps the function signature clean.

## Best Practices: Minimize Main Thread Usage

While the macro makes main thread access convenient, systems running on the main thread cannot execute in parallel with other systems. This can become a performance bottleneck in complex applications.

### Recommended Architecture

The most efficient approach is to minimize main thread systems by using an event-driven architecture:

1. **Multi-threaded systems** handle game logic and emit events
2. **Main thread systems** consume events and update Godot APIs

```rust
// Multi-threaded: Game logic emits events
fn handle_player_damage(
    mut events: EventWriter<PlayerDamagedEvent>,
    mut query: Query<&mut Health, With<Player>>,
) {
    for mut health in query.iter_mut() {
        if health.current <= 0 {
            events.send(PlayerDamagedEvent {
                new_health: health.current,
            });
        }
    }
}

// Main thread: Consume events and update UI
#[godot_main_thread]
fn update_health_display(
    mut events: EventReader<PlayerDamagedEvent>,
    mut ui_query: Query<&mut GodotNodeHandle, With<HealthBar>>,
) {
    for event in events.read() {
        for mut handle in ui_query.iter_mut() {
            if let Some(mut progress_bar) = handle.try_get::<ProgressBar>() {
                progress_bar.set_value(event.new_health as f64);
            }
        }
    }
}
```

### Benefits of Event-Driven Architecture

- **Better parallelization**: Core game logic runs on multiple threads
- **Cleaner separation**: Business logic decoupled from presentation layer
- **Easier testing**: Game logic systems can be tested without Godot APIs
- **Reduced main thread contention**: Fewer systems competing for main thread time

## Command Pattern Alternative

For complex UI operations, consider implementing a command pattern where multi-threaded systems queue commands that main thread systems process:

```rust
#[derive(Event)]
enum UICommand {
    SetText { target: Entity, text: String },
    SetVisible { target: Entity, visible: bool },
    PlayAnimation { target: Entity, animation: String },
}

// Multi-threaded: Queue UI commands
fn game_logic_system(mut commands: EventWriter<UICommand>) {
    commands.send(UICommand::SetText {
        target: ui_entity,
        text: "Level Complete!".to_string(),
    });
}

// Main thread: Process UI commands
#[godot_main_thread]
fn process_ui_commands(
    mut commands: EventReader<UICommand>,
    query: Query<&GodotNodeHandle>,
) {
    for command in commands.read() {
        match command {
            UICommand::SetText { target, text } => {
                if let Ok(handle) = query.get(*target) {
                    if let Some(mut label) = handle.clone().try_get::<Label>() {
                        label.set_text(text);
                    }
                }
            }
            // Handle other commands...
        }
    }
}
```

## When to Use Main Thread Systems

Direct main thread access is appropriate for:

- **Initialization systems** that set up scene tree structures
- **Simple, infrequent updates** where the complexity of events isn't justified
- **Debugging and development tools** where performance is less critical
- **Legacy code migration** where immediate event refactoring isn't feasible

## Performance Considerations

Keep main thread systems lightweight:

- Batch operations when possible
- Use early returns to skip unnecessary work
- Consider frame-rate limiting for expensive operations
- Profile your application to identify main thread bottlenecks

Remember that Godot's frame rate is tied to main thread performance, so keeping main thread systems efficient directly impacts the user experience.