# Thread Safety and Godot APIs

Some Godot APIs are not thread-safe and must be called exclusively from the main thread. This creates an important constraint when working with Bevy's multi-threaded ECS, where systems typically run in parallel across multiple threads. For additional details, see [Thread-safe APIs — Godot Engine](https://docs.godotengine.org/en/stable/tutorials/performance/thread_safe_apis.html).

## The Main Thread Requirement

Any system that interacts with Godot APIs—such as calling methods on `Node`, accessing scene tree properties, or manipulating UI elements—must run on the main thread. This includes:

- Scene tree operations (`add_child`, `queue_free`, etc.)
- Transform modifications on Godot nodes
- UI updates (setting text, visibility, etc.)
- Audio playback controls
- Input handling via Godot's `Input` singleton
- File I/O operations through Godot's resource system

## Main-thread access with `GodotAccess`

Use the `GodotAccess` SystemParam whenever you need to call Godot APIs. It carries a `NonSend` guard, so any system that includes it is scheduled on the main thread:

```rust
use godot_bevy::prelude::*;

fn update_ui_labels(
    query: Query<&GodotNodeHandle, With<PlayerStats>>,
    stats: Res<GameStats>,
    mut godot: GodotAccess,
) {
    for handle in &query {
        if let Some(mut label) = godot.try_get::<Label>(*handle) {
            label.set_text(&format!("Score: {}", stats.score));
        }
    }
}
```

`SceneTreeRef` is also a `NonSend` SystemParam. If a system already takes `SceneTreeRef`, it is pinned to the main thread and you do not need an extra `GodotAccess` parameter unless you actually call Godot APIs.

## Best Practices: Minimize Systems That Call Godot APIs

While `GodotAccess` makes Godot API access explicit, systems that use it cannot execute in parallel with other main thread-assigned systems. This can become a performance bottleneck in complex applications, as all systems requiring Godot API access must wait their turn to execute sequentially on this single thread.

### Recommended Architecture

The most efficient approach is to minimize main thread systems by using an event-driven architecture:

1. **Multi-threaded systems** handle game logic and emit events
2. **Main thread systems** consume events and update Godot APIs

### Benefits of Event-Driven Architecture

- **Better parallelization**: Core game logic runs on multiple threads
- **Cleaner separation**: Business logic decoupled from presentation layer
- **Easier testing**: Game logic systems can be tested without Godot APIs
- **Reduced main thread contention**: Fewer systems competing for main thread time
