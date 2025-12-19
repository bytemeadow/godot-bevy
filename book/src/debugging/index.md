# Debugging

Godot-bevy includes a built-in entity inspector that displays your Bevy ECS state directly in the Godot editor. When running your game, you can see all entities, their components, and parent-child relationships in real time.

## Entity Inspector

The inspector appears as a "Entities" tab next to the Scene tab in the editor's left dock. It shows:

- All Bevy entities with their names and appropriate icons
- Entity hierarchy (scene tree via `GodotChildOf`/`GodotChildren`)
- Components attached to each entity with type-specific icons
- Entities with Godot nodes show their node type icon (e.g., Node2D, Sprite2D)

### Enabling the Inspector

The inspector is included in `GodotDefaultPlugins`:

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotDefaultPlugins);
}
```

Or add it individually:

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotDebuggerPlugin);
}
```

### Configuration

Control the inspector through the `DebuggerConfig` resource:

```rust
fn configure_debugger(mut config: ResMut<DebuggerConfig>) {
    config.enabled = true;       // Toggle on/off
    config.update_interval = 0.5; // Seconds between updates
}
```

### Using the Inspector

1. Open your project in Godot
2. Look for the "Bevy" tab in the left dock (next to Scene/Import)
3. Run your game
4. The inspector populates with your ECS state

Entities display as a tree. Click to expand and see:
- Child entities (nested under parents)
- Components (shown in blue, with full type path on hover)

Entity icons indicate the Godot node type when a marker component is present (e.g., `Node2DMarker` shows the Node2D icon). Entities with a `GodotNodeHandle` but no specific marker show the Godot logo.

### Debugging Hierarchy Issues

The inspector mirrors the Godot scene tree via `GodotChildOf`/`GodotChildren`, not Bevy's
built-in `ChildOf`/`Children`. If an entity appears at the wrong level:

1. Verify the Godot node was in the scene tree when the entity was created
2. If you reparent nodes, wait a frame for the hierarchy update to process

### Performance Considerations

The inspector sends data every 0.5 seconds by default. For games with thousands of entities, you may want to increase the interval or disable it in release builds:

```rust
fn setup_debugger(mut config: ResMut<DebuggerConfig>) {
    #[cfg(debug_assertions)]
    {
        config.enabled = true;
        config.update_interval = 1.0; // Slower updates for large scenes
    }
    
    #[cfg(not(debug_assertions))]
    {
        config.enabled = false;
    }
}
```
