# Basic Concepts

Before diving into godot-bevy development, it's important to understand the key concepts that make this integration work.

## The Hybrid Architecture

godot-bevy creates a bridge between two powerful systems:

### Godot Side
- **Scene tree** with nodes
- **Visual editor** for level design
- **Asset pipeline** for resources
- **Rendering engine**
- **Physics engine**

### Bevy Side
- **Entity Component System (ECS)**
- **Systems** for game logic
- **Components** for data
- **Resources** for shared state
- **Schedules** for execution order

### The Bridge
godot-bevy seamlessly connects these worlds:
- Godot nodes ↔ ECS entities
- Node properties ↔ Components
- Signals → Events
- Resources ↔ Assets

## Core Components

### Entities
In godot-bevy, Godot nodes are automatically registered as ECS entities:

```rust
// When a node is added to the scene tree,
// it becomes queryable as an entity
fn find_player(
    query: Query<&Name, With<GodotNodeHandle>>,
) {
    for name in query.iter() {
        if name.as_str() == "Player" {
            // Found the player node!
        }
    }
}
```

### Components
Components store data on entities. godot-bevy provides several built-in components:

- `GodotNodeHandle` - Reference to the Godot node
- `Transform2D/3D` - Position, rotation, scale
- `Name` - Node name
- `Collisions` - Collision events
- `Groups` - Godot node groups

### Systems
Systems contain your game logic and run on a schedule:

```rust
fn movement_system(
    time: Res<Time>,
    mut query: Query<&mut Transform2D, With<Player>>,
) {
    for mut transform in query.iter_mut() {
        transform.as_bevy_mut().translation.x += 
            100.0 * time.delta_seconds();
    }
}
```

## The #[bevy_app] Macro

The entry point for godot-bevy is the `#[bevy_app]` macro:

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    // Configure your Bevy app here
    app.add_systems(Update, my_system);
}
```

This macro:
1. Creates the GDExtension entry point
2. Sets up the Bevy app
3. Integrates with Godot's lifecycle
4. Handles all the bridging magic

## Data Flow

Understanding how data flows between Godot and Bevy is crucial:

### Godot → Bevy
1. Node added to scene tree
2. Entity created with components
3. Signals converted to events
4. Input forwarded to systems

### Bevy → Godot
1. Transform components sync to nodes
2. Commands can modify scene tree
3. Resources can be loaded
4. Audio can be played

## Key Principles

### 1. Godot for Content, Bevy for Logic
- Design levels in Godot's editor
- Write game logic in Bevy systems
- Let each tool do what it does best

### 2. Components as the Source of Truth
- Store game state in components
- Use Godot nodes for presentation
- Sync only what's necessary

### 3. Systems for Everything
- Movement? System.
- Combat? System.
- UI updates? System.
- This promotes modularity and reusability

### 4. Leverage Both Ecosystems
- Use Godot's assets and tools
- Use Bevy's plugins and crates
- Don't reinvent what already exists

## Common Patterns

### Finding Nodes by Name
```rust
fn setup(
    mut query: Query<(&Name, Entity)>,
) {
    let player = query.iter()
        .find_entity_by_name("Player")
        .expect("Player node must exist");
}
```

### Reacting to Signals
```rust
fn handle_button_press(
    mut events: EventReader<GodotSignal>,
) {
    for signal in events.read() {
        if signal.name == "pressed" {
            // Button was pressed!
        }
    }
}
```

### Spawning Godot Scenes
```rust
# use bevy::app::{App, Plugin, Startup, Update};
# use bevy::asset::{AssetServer, Handle};
# use bevy::prelude::{Commands, Component, Res, Resource, Single, With};
# use godot_bevy::bridge::GodotNodeHandle;
# use godot_bevy::prelude::{GodotResource, GodotScene, Transform2D};
# 
# struct EnemyPlugin;
# 
# impl Plugin for EnemyPlugin {
#     fn build(&self, app: &mut App) {
#         app.add_systems(Startup, load_assets);
#         app.add_systems(Update, spawn_enemy);
#     }
# }
# 
# #[derive(Resource, Debug)]
# struct EnemyScene(Handle<GodotResource>);
# 
# #[derive(Component, Debug)]
# struct Enemy {
#     health: i32,
# }
# 
# #[derive(Component, Debug)]
# struct EnemySpawner;
# 
# fn load_assets(mut commands: Commands, server: Res<AssetServer>) {
#     let handle: Handle<GodotResource> = server.load("scenes/enemy.tscn");
#     commands.insert_resource(EnemyScene(handle));
# }
# 
fn spawn_enemy(
    mut commands: Commands,
    enemy_scene: Res<EnemyScene>,
    enemy_spawner: Single<&GodotNodeHandle, With<EnemySpawner>>,
) {
    commands.spawn((
        GodotScene::from_handle(enemy_scene.0.clone())
            .with_parent(enemy_spawner.into_inner().clone()),
        Enemy { health: 100 },
        Transform2D::default(),
    ));
}
```

## Next Steps

Now that you understand the basic concepts:
- Try the [examples](https://github.com/godot-rust/godot-bevy/tree/main/examples)
- Read about specific systems in detail
- Start building your game!

Remember: godot-bevy is about using the right tool for the right job. Embrace both Godot and Bevy's strengths!