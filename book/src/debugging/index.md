# Debugging

Godot-bevy's entity inspector shows Bevy entities, component values and scene-tree
relationships in the Godot editor while your game runs.

## Entity inspector

The **Entities** tab is in the editor's left dock, next to Scene/Import. Expand an
entity to see its components and registered reflected values. Hover a component
for its full type path. Node-backed entities show a Godot node icon; the hierarchy
follows `GodotChildOf`/`GodotChildren`.

### Enabling the inspector

`GodotDefaultPlugins` includes the inspector. To add it individually:

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotDebuggerPlugin);
}
```

### Registering components

Enable automatic reflection registration on your Bevy dependency:

```toml
bevy = { version = "0.19", default-features = false, features = ["reflect_auto_register"] }
```

Derive `Reflect` on components you want to inspect. Add `#[reflect(Component)]`
to allow the runtime inspection service to edit their supported leaves:

```rust
use bevy::prelude::*;

#[derive(Component, Reflect)]
#[reflect(Component)]
pub struct Speed(pub f32);
```

The platformer example uses this automatic path for `Speed`, `JumpVelocity`,
`Gravity` and `Player`. Registered values already appear in the dock; editing
controls for the new service will arrive in a later editor update.

For web and static builds, keep explicit registration as a fallback when automatic
registration has not been verified with your build and loader:

```rust
app.register_type::<Speed>();
```

Registration also includes reflected dependencies. Spawning a component or exporting
a Godot property does not register its type. Unregistered components remain listed;
`#[reflect(ignore)]` fields are hidden. Components without `ReflectComponent` can
still be read when registered, but cannot be edited generically.
`#[reflect(@InspectorReadOnly)]` shows a field without allowing edits;
`#[reflect(@InspectorRange::new(min, max))]` bounds scalar edits (both are in the prelude).

### Using the inspector

1. Enable the godot-bevy addon in your Godot project.
2. Open the Entities tab and run the game.
3. Expand an entity to inspect its components and children.

If an entity appears under the wrong parent, check that its Godot node was in the
scene tree when the entity was created. After reparenting, allow a frame for the
relationship update. This tree uses Godot's relationships, separate from Bevy's
`ChildOf`/`Children`.

### Configuration and cost

The current dock receives a snapshot every 0.5 seconds while a debugger is attached.
For larger worlds, increase its refresh interval or disable the service:

```rust
fn configure_debugger(mut config: ResMut<DebuggerConfig>) {
    config.enabled = cfg!(debug_assertions);
    config.update_interval = 1.0;
}
```

The request service sends summary notifications only after a subscription, with one
initial snapshot followed by entity additions, removals, renames and reparentings.
It reads full component values on request. `DebuggerConfig::value_limits` bounds
collection elements and nesting depth, with explicit truncation markers. The legacy
dock keeps its snapshot stream until it switches to subscriptions.
