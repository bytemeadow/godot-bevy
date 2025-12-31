# Signal Handling

Godot signals are a core communication mechanism in the Godot engine. godot-bevy bridges those signals into Bevy events so your ECS systems can react to UI, gameplay, and scene-tree events in a type-safe way.

This page focuses on the typed signals API (recommended). A legacy API remains available but is deprecated; see the Legacy section below.

## Outline

- [Quick Start](#quick-start)
- [Multiple Typed Events](#multiple-typed-events)
- [Passing Context (Node, Entity, Arguments)](#passing-context-node-entity-arguments)
- [Deferred Connections](#deferred-connections)
- [Attaching signals to Godot scenes](#attaching-signals-to-godot-scenes)
- [Untyped Legacy API (Deprecated)](#untyped-legacy-api-deprecated)

## Quick Start

1) Define a Bevy message for your case:

```rust
use bevy::prelude::*;
use godot_bevy::prelude::*;

#[derive(Message, Debug, Clone)]
struct StartGameRequested;
```

2) Register the typed plugin for your message type:

```rust
fn build_app(app: &mut App) {
    app.add_plugins(GodotTypedSignalsPlugin::<StartGameRequested>::default());
}
```

3) Connect a Godot signal and map it to your message:

```rust
fn connect_button(
    buttons: Query<&GodotNodeHandle, With<Button>>,
    typed: TypedGodotSignals<StartGameRequested>,
    mut godot: GodotAccess,
) {
    for handle in &buttons {
        typed.connect_map(
            &mut godot,
            *handle,
            "pressed",
            None,
            |_args, _node_handle, _ent| Some(StartGameRequested),
        );
    }
}
```

4) Listen for the message anywhere:

```rust
fn on_start(mut ev: MessageReader<StartGameRequested>) {
    for _ in ev.read() {
        // Start the game!
    }
}
```

## Multiple Typed Events

Use one plugin per event type. You can map the same Godot signal to multiple typed events if you like:

```rust
#[derive(Message, Debug, Clone)] struct ToggleFullscreen;
#[derive(Message, Debug, Clone)] struct QuitRequested { source: GodotNodeHandle }

fn setup(app: &mut App) {
    app.add_plugins(GodotTypedSignalsPlugin::<ToggleFullscreen>::default())
       .add_plugins(GodotTypedSignalsPlugin::<QuitRequested>::default());
}

fn connect_menu(
    menu: Query<(&GodotNodeHandle, &MenuTag)>,
    toggle: TypedGodotSignals<ToggleFullscreen>,
    quit: TypedGodotSignals<QuitRequested>,
    mut godot: GodotAccess,
) {
    for (button, tag) in &menu {
        match tag {
            MenuTag::Fullscreen => {
                toggle.connect_map(
                    &mut godot,
                    *button,
                    "pressed",
                    None,
                    |_a, _node_handle, _e| Some(ToggleFullscreen),
                );
            }
            MenuTag::Quit => {
                quit.connect_map(
                    &mut godot,
                    *button,
                    "pressed",
                    None,
                    |_a, node_handle, _e| Some(QuitRequested { source: node_handle }),
                );
            }
        }
    }
}
```

## Passing Context (Node, Entity, Arguments)

The mapper closure receives:

- `args: &[Variant]`: raw Godot arguments (clone if you need detailed parsing)
- `node_handle: GodotNodeHandle`: emitting node handle (use it later with `GodotAccess`)
- `entity: Option<Entity>`: Bevy entity if you passed `Some(entity)` to `connect_map`

Important: the mapper runs inside the Godot signal callback. Do not call Godot APIs in the mapper; resolve the `node_handle` in a system with `GodotAccess` on the main thread. See [Thread Safety and Godot APIs](../threading/index.md).

Example adding the entity:

```rust
#[derive(Message, Debug, Clone, Copy)]
struct AreaExited(Entity);

fn connect_area(
    q: Query<(Entity, &GodotNodeHandle), With<Area2D>>,
    typed: TypedGodotSignals<AreaExited>,
    mut godot: GodotAccess,
) {
    for (entity, area) in &q {
        typed.connect_map(
            &mut godot,
            *area,
            "body_exited",
            Some(entity),
            |_a, _node_handle, e| Some(AreaExited(e.unwrap())),
        );
    }
}
```

## Deferred Connections

When spawning entities before their `GodotNodeHandle` is ready, you can defer connections. Add `TypedDeferredSignalConnections<T>` with a signal-to-event mapper; the `GodotTypedSignalsPlugin<T>` wires it once the handle appears.

```rust
#[derive(Component)] struct MyArea;
#[derive(Message, Debug, Clone, Copy)] struct BodyEntered(Entity);

fn setup(app: &mut App) {
    app.add_plugins(GodotTypedSignalsPlugin::<BodyEntered>::default());
}

fn spawn_area(mut commands: Commands) {
    commands.spawn((
        MyArea,
        // Defer until GodotNodeHandle is available on this entity
        TypedDeferredSignalConnections::<BodyEntered>::with_connection(
            "body_entered",
            |_a, _node_handle, e| Some(BodyEntered(e.unwrap())),
        ),
    ));
}
```

## Attaching signals to Godot scenes

When spawning an entity associated with a Godot scene, you can schedule
signals to be connected to children of the scene once the scene is spawned.
When inserting a `GodotScene` resource, use the `with_signal_connection` builder method to schedule connections.

The method arguments are similar to other typed signal constructors such as `connect_map`:
* `node_path` - Path relative to the scene root (e.g., "VBox/MyButton" or "." for root node).
  Argument supports the same syntax as [Node.get_node](https://docs.godotengine.org/en/stable/classes/class_node.html#class-node-method-get-node).
* `signal_name` - Name of the Godot signal to connect (e.g., "pressed").
* `mapper` - Closure that maps signal arguments to your typed message.
  * The closure receives three arguments: `args`, `node_handle`, and `entity`:
    - `args: &[Variant]`: raw Godot arguments (clone if you need detailed parsing).
    - `node_handle: GodotNodeHandle`: emitting node handle.
    - `entity: Option<Entity>`: Bevy entity the GodotScene component is attached to (Always Some).
  * The closure returns an optional Bevy Message, or None to not send the message.

```rust,ignore
impl Command for SpawnPickup {
    fn apply(self, world: &mut World) -> () {
        let assets = world.get_resource::<PickupAssets>().cloned();

        let mut pickup = world.spawn_empty();
        pickup
            .insert(Name::new("Pickup"))
            .insert(Transform::from_xyz(200.0, 200.0, 0.0));

        // Only insert GodotScene if Godot engine is running; useful when running tests without Godot.
        if let Some(assets) = assets {
            pickup.insert(
                GodotScene::from_handle(assets.scene.clone())
                
                    // Schedule the "area_entered" signal on the Area2D child
                    // to be connected to PickupAreaEntered message
                    .with_signal_connection(
                        "Area2D",
                        "area_entered",
                        |_args, _node_handle, _entity| {
                            // Pickup "area_entered" signal mapped
                            Some(PickupAreaEntered)
                        },
                ),
            );
        }
    }
}
```

## Untyped Legacy API (Deprecated)

The legacy API (`GodotSignals`, `GodotSignal`, `connect_godot_signal`) remains available but is deprecated. Prefer the typed API above. Minimal usage for migration:

```rust
fn connect_legacy(
    q: Query<&GodotNodeHandle, With<Button>>,
    legacy: GodotSignals,
    mut godot: GodotAccess,
) {
    for handle in &q {
        legacy.connect(&mut godot, *handle, "pressed");
    }
}

fn read_legacy(mut ev: MessageReader<GodotSignal>) {
    for s in ev.read() {
        if s.name == "pressed" { /* ... */ }
    }
}
```

For physics signals (collisions), use the collisions plugin/events instead of raw signals when possible.
