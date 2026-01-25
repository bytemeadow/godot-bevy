# Signal Handling

Godot signals are a core communication mechanism in the Godot engine. godot-bevy bridges those signals into Bevy observers so your ECS systems can react to UI, gameplay, and scene-tree events in a type-safe, reactive way.

## Outline

- [Quick Start](#quick-start)
- [Multiple Signal Events](#multiple-signal-events)
- [Passing Context (Node, Entity, Arguments)](#passing-context-node-entity-arguments)
- [Connecting to Non-Entity Objects](#connecting-to-non-entity-objects)
- [Deferred Connections](#deferred-connections)
- [Attaching signals to Godot scenes](#attaching-signals-to-godot-scenes)
- [Signal Name Constants](#signal-name-constants)

## Quick Start

1) Define a Bevy event for your signal:

```rust
use bevy::prelude::*;
use godot_bevy::prelude::*;

#[derive(Event, Debug, Clone)]
struct StartGameRequested;
```

2) Register the signals plugin for your event type:

```rust
fn build_app(app: &mut App) {
    app.add_plugins(GodotSignalsPlugin::<StartGameRequested>::default());
}
```

3) Connect a Godot signal and map it to your event:

```rust
fn connect_button(
    buttons: Query<&GodotNodeHandle, With<Button>>,
    signals: GodotSignals<StartGameRequested>,
) {
    for handle in &buttons {
        signals.connect(
            *handle,
            "pressed",
            None,
            |_args, _node_handle, _ent| Some(StartGameRequested),
        );
    }
}
```

4) React to the event with an observer:

```rust
fn setup(app: &mut App) {
    app.add_observer(on_start_game);
}

fn on_start_game(
    _trigger: On<StartGameRequested>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    next_state.set(GameState::Playing);
}
```

Observers fire immediately when the signal is received, giving you reactive, push-based event handling rather than polling each frame.

## Multiple Signal Events

Use one plugin per event type. You can map the same Godot signal to multiple typed events if you like:

```rust
#[derive(Event, Debug, Clone)] struct ToggleFullscreen;
#[derive(Event, Debug, Clone)] struct QuitRequested { source: GodotNodeHandle }

fn setup(app: &mut App) {
    app.add_plugins(GodotSignalsPlugin::<ToggleFullscreen>::default())
       .add_plugins(GodotSignalsPlugin::<QuitRequested>::default())
       .add_observer(on_toggle_fullscreen)
       .add_observer(on_quit);
}

fn connect_menu(
    menu: Query<(&GodotNodeHandle, &MenuTag)>,
    toggle: GodotSignals<ToggleFullscreen>,
    quit: GodotSignals<QuitRequested>,
) {
    for (button, tag) in &menu {
        match tag {
            MenuTag::Fullscreen => {
                toggle.connect(
                    *button,
                    "pressed",
                    None,
                    |_a, _node_handle, _e| Some(ToggleFullscreen),
                );
            }
            MenuTag::Quit => {
                quit.connect(
                    *button,
                    "pressed",
                    None,
                    |_a, node_handle, _e| Some(QuitRequested { source: node_handle }),
                );
            }
        }
    }
}

fn on_toggle_fullscreen(_trigger: On<ToggleFullscreen>, mut godot: GodotAccess) {
    // Toggle fullscreen
}

fn on_quit(_trigger: On<QuitRequested>) {
    // Quit the game
}
```

## Passing Context (Node, Entity, Arguments)

The mapper closure receives:

- `args: &[Variant]`: raw Godot arguments (clone if you need detailed parsing)
- `node_handle: GodotNodeHandle`: emitting node handle (use it later with `GodotAccess`)
- `entity: Option<Entity>`: Bevy entity if you passed `Some(entity)` to `connect`

Important: the mapper runs inside the Godot signal callback. Do not call Godot APIs in the mapper; resolve the `node_handle` in an observer or system with `GodotAccess` on the main thread. Connections are queued and applied on the main thread; connections made during a frame take effect on the next frame. If you need same-frame connection, use `connect_immediate` with a `GodotAccess` parameter. See [Thread Safety and Godot APIs](../threading/index.md).

Example including the entity in the event:

```rust
#[derive(Event, Debug, Clone, Copy)]
struct AreaExited { entity: Entity }

fn connect_area(
    q: Query<(Entity, &GodotNodeHandle), With<Area2D>>,
    signals: GodotSignals<AreaExited>,
) {
    for (entity, area) in &q {
        signals.connect(
            *area,
            "body_exited",
            Some(entity),
            |_a, _node_handle, e| Some(AreaExited { entity: e.unwrap() }),
        );
    }
}

fn on_area_exited(trigger: On<AreaExited>, mut commands: Commands) {
    let entity = trigger.event().entity;
    commands.entity(entity).despawn();
}
```

## Connecting to Non-Entity Objects

The `connect` method works with `GodotNodeHandle`, which represents nodes tracked as ECS entities. However, some Godot objects like `SceneTree` are not tracked as entities. For these cases, use `connect_object`:

```rust
use bevy::prelude::*;
use godot_bevy::prelude::*;
use godot_bevy::interop::signal_names::SceneTreeSignals;

#[derive(Event, Debug, Clone)]
struct SceneChanged;

fn setup(app: &mut App) {
    app.add_plugins(GodotSignalsPlugin::<SceneChanged>::default())
       .add_systems(Startup, connect_scene_tree)
       .add_observer(on_scene_changed);
}

fn connect_scene_tree(
    signals: GodotSignals<SceneChanged>,
    mut scene_tree: SceneTreeRef,
) {
    let tree = scene_tree.get().clone();
    signals.connect_object(tree, SceneTreeSignals::SCENE_CHANGED, |_args| {
        Some(SceneChanged)
    });
}

fn on_scene_changed(_trigger: On<SceneChanged>) {
    println!("Scene changed!");
}
```

The `connect_object` method accepts any `Gd<T>` where `T` inherits from `Object`. This is useful for:

- **SceneTree signals** - `scene_changed`, `tree_changed`, `node_added`, etc.
- **Autoload singletons** - Custom autoloads that emit signals
- **Non-node objects** - Any Godot object that isn't tracked as an ECS entity

The mapper closure for `connect_object` is simpler than `connect` since there's no associated entity:

```rust
// connect_object mapper: just args
|args: &[Variant]| -> Option<MyEvent>

// connect mapper: args, node handle, and optional entity
|args: &[Variant], node_handle: GodotNodeHandle, entity: Option<Entity>| -> Option<MyEvent>
```

## Deferred Connections

When spawning entities before their `GodotNodeHandle` is ready, you can defer connections. Add `DeferredSignalConnections<T>` with a signal-to-event mapper; the `GodotTypedSignalsPlugin<T>` wires it once the handle appears.

```rust
#[derive(Component)] struct MyArea;
#[derive(Event, Debug, Clone, Copy)] struct BodyEntered { entity: Entity }

fn setup(app: &mut App) {
    app.add_plugins(GodotSignalsPlugin::<BodyEntered>::default())
       .add_observer(on_body_entered);
}

fn spawn_area(mut commands: Commands) {
    commands.spawn((
        MyArea,
        // Defer until GodotNodeHandle is available on this entity
        DeferredSignalConnections::<BodyEntered>::with_connection(
            "body_entered",
            |_a, _node_handle, e| Some(BodyEntered { entity: e.unwrap() }),
        ),
    ));
}

fn on_body_entered(trigger: On<BodyEntered>) {
    println!("Body entered area on entity {:?}", trigger.event().entity);
}
```

## Attaching signals to Godot scenes

When spawning an entity associated with a Godot scene, you can schedule
signals to be connected to children of the scene once the scene is spawned.
When inserting a `GodotScene` resource, use the `with_signal_connection` builder method to schedule connections.

The method arguments are similar to other typed signal constructors such as `connect`:
* `node_path` - Path relative to the scene root (e.g., "VBox/MyButton" or "." for root node).
  Argument supports the same syntax as [Node.get_node](https://docs.godotengine.org/en/stable/classes/class_node.html#class-node-method-get-node).
* `signal_name` - Name of the Godot signal to connect (e.g., "pressed").
* `mapper` - Closure that maps signal arguments to your typed event.
  * The closure receives three arguments: `args`, `node_handle`, and `entity`:
    - `args: &[Variant]`: raw Godot arguments (clone if you need detailed parsing).
    - `node_handle: GodotNodeHandle`: emitting node handle.
    - `entity: Option<Entity>`: Bevy entity the GodotScene component is attached to (Always Some).
  * The closure returns an optional Bevy Event, or None to not send the event.

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
                    // to be connected to PickupAreaEntered event
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

For physics signals (collisions), use the collisions plugin/events instead of raw signals when possible.

## Signal Name Constants

godot-bevy provides auto-generated constants for Godot signal names, offering type-safe, discoverable alternatives to string literals. These are located in `godot_bevy::interop::signal_names`.

```rust
use godot_bevy::interop::signal_names::{BaseButtonSignals, SceneTreeSignals, Area2DSignals};

// Instead of string literals:
signals.connect(button, "pressed", None, mapper);

// Use constants:
signals.connect(button, BaseButtonSignals::PRESSED, None, mapper);

// SceneTree signals
signals.connect_object(tree, SceneTreeSignals::SCENE_CHANGED, mapper);
signals.connect_object(tree, SceneTreeSignals::TREE_CHANGED, mapper);

// Area2D signals
signals.connect(area, Area2DSignals::BODY_ENTERED, Some(entity), mapper);
signals.connect(area, Area2DSignals::AREA_ENTERED, Some(entity), mapper);
```

Benefits of using constants:

- **Compile-time validation** - Typos are caught at compile time
- **IDE autocompletion** - Discover available signals easily
- **Documentation** - Constants include doc comments explaining when each signal fires

Common signal constant structs:

| Struct | Common Signals |
|--------|---------------|
| `BaseButtonSignals` | `PRESSED`, `BUTTON_UP`, `BUTTON_DOWN`, `TOGGLED` |
| `Area2DSignals` | `BODY_ENTERED`, `BODY_EXITED`, `AREA_ENTERED`, `AREA_EXITED` |
| `Area3DSignals` | `BODY_ENTERED`, `BODY_EXITED`, `AREA_ENTERED`, `AREA_EXITED` |
| `SceneTreeSignals` | `SCENE_CHANGED`, `TREE_CHANGED`, `NODE_ADDED`, `NODE_REMOVED` |
| `AnimationPlayerSignals` | `ANIMATION_FINISHED`, `ANIMATION_STARTED`, `ANIMATION_CHANGED` |
| `TimerSignals` | `TIMEOUT` |
| `VisibleOnScreenNotifier2DSignals` | `SCREEN_ENTERED`, `SCREEN_EXITED` |
