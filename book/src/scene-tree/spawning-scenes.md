# Spawning Godot scenes with `NodeTreeView`

`GodotScene` is a Bevy `Component` that lets us attach and instantiate Godot scene files (`.tscn`) to Bevy entities.
When we add a `GodotScene` to an entity, it spawns that scene in Godot's scene tree and links it to our Bevy entity,
letting us combine Godot's visual editor with Bevy's ECS architecture.

When we spawn scenes, we almost always need to reach into that scene’s node tree to:
- grab child nodes like sprites, notifiers, or UI controls
- connect signals
- drive animations or physics bodies

Doing this manually with raw `GodotNodeHandle` lookups quickly becomes repetitive and fragile.  
The `#[derive(NodeTreeView)]` macro gives us a **typed, ergonomic view** of a scene’s node tree, driven by node paths.

This page explains how to:
1. Define a `NodeTreeView` for a scene
2. Spawn the scene via `GodotScene`
3. Use the generated view to access nodes and connect signals



## 1. Spawn Godot scenes with `GodotScene`

To spawn a Godot scene from Bevy, insert `GodotScene` into our entity:

```rust,ignore
# use bevy::prelude::*;
# use godot_bevy::prelude::{GodotResource, GodotScene};
# use bevy::state::app::StatesPlugin;
# use bevy_asset_loader::asset_collection::AssetCollection;
#
# #[derive(Debug, Default, Clone, Eq, PartialEq, Hash, States)]
# enum GameState {
#     #[default]
#     Loading,
#     InGame,
# }
# 
# fn plugin(app: &mut App) {
#     app.add_plugins(StatesPlugin)
#         .init_state::<GameState>()
#         .add_loading_state(
#             LoadingState::new(GameState::Loading).continue_to_state(GameState::InGame),
#         );
#     app.configure_loading_state(
#         LoadingStateConfig::new(GameState::Loading).load_collection::<PickupAssets>(),
#     );
#     app.add_message::<PickupBodyEntered>();
#     app.add_systems(Update, pickup_system);
# }
# 
/// This example uses `bevy_asset_loader` to load the
/// scene file as a packed scene at startup.
#[derive(AssetCollection, Resource)]
pub struct CharacterAssets {
    #[asset(path = "scenes/character.tscn")]
    pub character_scene: Handle<GodotResource>,
}

fn spawn_character(mut commands: Commands, assets: Res<CharacterAssets>) {
    commands
        .spawn_empty()
        // Add additional Bevy components here (e.g. position, gameplay data, etc.)
        .insert(Transform::default())
        .insert(
            // Attach the Godot scene to this Bevy entity
            GodotScene::from_handle(assets.character_scene.clone())
            // Optionally, connect signals here with the
            // `with_signal_connection` builder method discussed below. 
        );
}
```

At this point, the Bevy entity is linked to the Godot scene instance.  
Now we would like to access nodes inside that scene.




## 2. Access scene children with `NodeTreeView`

Let's assume we have a Godot scene with the following node structure:

- `Node2D` (the “character”)
  - `AnimatedSprite2D`
  - `VisibleOnScreenNotifier2D`

We can describe the nodes of the scene we want to access by their path like so:

```rust,ignore
# use godot_bevy::interop::GodotNodeHandle;
# use godot_bevy::prelude::NodeTreeView;
#
#[derive(NodeTreeView)]
pub struct CharacterNodes {
    #[node("AnimatedSprite2D")]
    pub animated_sprite: GodotNodeHandle,

    #[node("VisibleOnScreenNotifier2D")]
    pub visibility_notifier: GodotNodeHandle,
}
```

The `NodeTreeView` field types can be `GodotNodeHandle` or `Option<GodotNodeHandle>`.

The `#[node("<node_path>")]` attribute supports wildcards (`*`).
See below or the `NodeTreeView` docs for more details.

Then we can access the tree view in our system like this:

```rust,ignore
fn new_character_initialize(
    entities: Query<&GodotNodeHandle, Added<Character>>,
    mut godot: GodotAccess,
) {
    for handle in &entities {
        let character = godot.get::<RigidBody2D>(*handle);
        let character_nodes = CharacterNodes::from_node(character).unwrap();
    }
}
```

### Path patterns

Node paths support simple patterns to avoid hard-coding full names:

- `/root/*/HUD/CurrentLevel` - matches any single node name where * appears
- `/root/Level*/HUD/CurrentLevel` - matches node names starting with "Level"
- `*/HUD/CurrentLevel` - matches relative to the base node

### Generated path constants

For each `#[node("<node_path>")]` field, `NodeTreeView` generates a public string constant named  
`<UPPERCASE_FIELD_NAME>_PATH` inside our struct’s `impl`.

Given the `CharacterNodes` example above, the macro generates an impl like:

```rust,ignore
impl CharacterNodes {
    pub const ANIMATED_SPRITE_PATH: &'static str = "AnimatedSprite2D";
    pub const VISIBILITY_NOTIFIER_PATH: &'static str = "VisibleOnScreenNotifier2D";
}
```

These constants are very convenient when we need to refer to the same path in multiple places, especially when
connecting signals from a spawned scene (covered below).




## 3. Connect signals to scene children using `GodotScene::with_signal_connection`

When spawning scenes, we often want to connect signals to child nodes.

There are three useful resources when connecting signals:
- `GodotScene`'s `with_signal_connection` builder method.
- `NodeTreeView`'s generated path constants.
- `godot_bevy::interop::<GODOT_NODE_TYPE>Signals` types which contain string constants for all signals of a given Godot node type.

Here is an example using the `CharacterNodes` `NodeTreeView` from above and
the `VisibleOnScreenNotifier2DSignals::SCREEN_EXITED` string constant to
connect the `VisibleOnScreenNotifier2D`'s `screen_exited` signal to a Bevy message.

```rust,ignore
use godot_bevy::interop::VisibleOnScreenNotifier2DSignals;
use godot_bevy::prelude::GodotScene;
use bevy::ecs::entity::Entity;
use bevy::prelude::Message;

#[derive(Message, Debug, Clone, Copy)]
pub struct CharacterScreenExited {
    pub entity: Entity,
}

fn spawn_character_with_signals(mut commands: Commands, assets: Res<CharacterAssets>) {
    commands
        .spawn_empty()
        .insert(Transform::default())
        .insert(
            GodotScene::from_handle(assets.character_scene.clone())
                .with_signal_connection(
                    
                    // Use the NodeTreeView-generated path constant:
                    CharacterNodes::VISIBILITY_NOTIFIER_PATH,
                    
                    // The Godot signal we want to connect:
                    VisibleOnScreenNotifier2DSignals::SCREEN_EXITED,
                    
                    // Closure to turn a Godot signal into a Bevy message:
                    |_args, _node_handle, entity| {
                        Some(CharacterScreenExited {
                            entity: entity.expect("entity was provided"),
                        })
                    },
                ),
        );
}
```
