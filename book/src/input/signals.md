# Signal Handling

Godot signals are a core communication mechanism in the Godot engine. godot-bevy bridges those signals into Bevy events so your ECS systems can react to UI, gameplay, and scene-tree events in a type-safe way.

This page focuses on the typed signals API (recommended). A legacy API remains available but is deprecated; see the Legacy section below.

## Quick Start (Typed)

1) Define a Bevy event for your case:

```rust
use bevy::prelude::*;
use godot_bevy::prelude::*;

#[derive(Event, Debug, Clone)]
struct StartGameRequested;
```

2) Register the typed plugin for your event type:

```rust
fn build_app(app: &mut App) {
    app.add_plugins(GodotTypedSignalsPlugin::<StartGameRequested>::default());
}
```

3) Connect a Godot signal and map it to your event:

```rust
fn connect_button(
    mut buttons: Query<&mut GodotNodeHandle, With<Button>>, 
    typed: TypedGodotSignals<StartGameRequested>,
) {
    for mut handle in &mut buttons {
        typed.connect_map(&mut handle, "pressed", None, |_args, _node, _ent| StartGameRequested);
    }
}
```

4) Listen for the event anywhere:

```rust
fn on_start(mut ev: EventReader<StartGameRequested>) {
    for _ in ev.read() {
        // Start the game!
    }
}
```

## Multiple Typed Events

Use one plugin per event type. You can map the same Godot signal to multiple typed events if you like:

```rust
#[derive(Event, Debug, Clone)] struct ToggleFullscreen;
#[derive(Event, Debug, Clone)] struct QuitRequested { source: GodotNodeHandle }

fn setup(app: &mut App) {
    app.add_plugins(GodotTypedSignalsPlugin::<ToggleFullscreen>::default())
       .add_plugins(GodotTypedSignalsPlugin::<QuitRequested>::default());
}

fn connect_menu(
    mut menu: Query<(&mut GodotNodeHandle, &MenuTag)>,
    toggle: TypedGodotSignals<ToggleFullscreen>,
    quit: TypedGodotSignals<QuitRequested>,
) {
    for (mut button, tag) in &mut menu {
        match tag {
            MenuTag::Fullscreen => {
                toggle.connect_map(&mut button, "pressed", None, |_a, _n, _e| ToggleFullscreen);
            }
            MenuTag::Quit => {
                quit.connect_map(&mut button, "pressed", None, |_a, n, _e| QuitRequested { source: n.clone() });
            }
        }
    }
}
```

## Passing Context (Node, Entity, Arguments)

The mapper closure receives:

- `args: &[Variant]`: raw Godot arguments (clone if you need detailed parsing)
- `node: &GodotNodeHandle`: emitting node; clone into your event if useful
- `entity: Option<Entity>`: Bevy entity if you passed `Some(entity)` to `connect_map`

Example adding the entity:

```rust
#[derive(Event, Debug, Clone, Copy)]
struct AreaExited(Entity);

fn connect_area(
    mut q: Query<(Entity, &mut GodotNodeHandle), With<Area2D>>, 
    typed: TypedGodotSignals<AreaExited>,
) {
    for (entity, mut area) in &mut q {
        typed.connect_map(&mut area, "body_exited", Some(entity), |_a, _n, e| AreaExited(e.unwrap()));
    }
}
```

## Deferred Connections (Typed)

When spawning entities before their `GodotNodeHandle` is ready, you can defer connections. Add `TypedDeferredSignalConnections<T>` with a signal-to-event mapper; the `GodotTypedSignalsPlugin<T>` wires it once the handle appears.

```rust
#[derive(Component)] struct MyArea;
#[derive(Event, Debug, Clone, Copy)] struct BodyEntered(Entity);

fn setup(app: &mut App) {
    app.add_plugins(GodotTypedSignalsPlugin::<BodyEntered>::default());
}

fn spawn_area(mut commands: Commands) {
    commands.spawn((
        MyArea,
        // Defer until GodotNodeHandle is available on this entity
        TypedDeferredSignalConnections::<BodyEntered>::with_connection("body_entered", |_a, _n, e| BodyEntered(e.unwrap())),
    ));
}
```

## Legacy API (Deprecated)

The legacy API (`GodotSignals`, `GodotSignal`, `connect_godot_signal`) remains available but is deprecated. Prefer the typed API above. Minimal usage for migration:

```rust
fn connect_legacy(mut q: Query<&mut GodotNodeHandle, With<Button>>, legacy: GodotSignals) {
    for mut handle in &mut q { legacy.connect(&mut handle, "pressed"); }
}

fn read_legacy(mut ev: EventReader<GodotSignal>) {
    for s in ev.read() {
        if s.name == "pressed" { /* ... */ }
    }
}
```

For physics signals (collisions), use the collisions plugin/events instead of raw signals when possible.
