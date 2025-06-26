# Thread Safety and Godot APIs

Some Godot APIs are not thread-safe and and must be called exclusively from the main thread. This creates an important constraint when working with Bevy's multi-threaded ECS, where systems typically run in parallel across multiple threads. For additional details, see [Thread-safe APIs — Godot Engine](https://docs.godotengine.org/en/stable/tutorials/performance/thread_safe_apis.html).

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

## Best Practices: Minimize Systems That Call Godot APIs

While the `#[godot_main_thread]` macro makes Godot API access convenient, systems assigned to the main thread cannot execute in parallel with other main thread-assigned systems. This can become a performance bottleneck in complex applications, as all systems requiring Godot API access must wait their turn to execute sequentially on this single thread.

### Recommended Architecture

The most efficient approach is to minimize main thread systems by using an event-driven architecture:

1. **Multi-threaded systems** handle game logic and emit events
2. **Main thread systems** consume events and update Godot APIs

### Benefits of Event-Driven Architecture

- **Better parallelization**: Core game logic runs on multiple threads
- **Cleaner separation**: Business logic decoupled from presentation layer
- **Easier testing**: Game logic systems can be tested without Godot APIs
- **Reduced main thread contention**: Fewer systems competing for main thread time
