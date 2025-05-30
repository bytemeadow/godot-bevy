# godot-bevy

`godot-bevy` is a Rust library that brings [Bevy's](https://bevyengine.org/) powerful Entity Component System (ECS) to the versatile [Godot Game Engine](https://godotengine.org/). Use Bevy's ergonomic and high-performance Rust ECS within your Godot projects to get the best of both worlds.

<div align="left" valign="middle">
<a href="https://runblaze.dev">
 <picture>
   <source media="(prefers-color-scheme: dark)" srcset="https://www.runblaze.dev/logo_dark.png">
   <img align="right" src="https://www.runblaze.dev/logo_light.png" height="102px"/>
 </picture>
</a>

<br style="display: none;"/>

_Special thanks to [Blaze](https://runblaze.dev) for their support of this project. They provide high-performance Linux (AMD64 & ARM64) and Apple Silicon macOS runners for GitHub Actions, greatly reducing our automated build times._

</div>

## Features

- **Deep ECS Integration**: True Bevy ECS systems controlling Godot nodes, not just bindings
- **Bidirectional Transform Sync**: Seamless Transform2D/3D synchronization between Bevy and Godot
- **Godot Signals in ECS**: Listen to and respond to Godot signals from Bevy systems
- **Collision Event Handling**: React to Godot collision events in your ECS systems
- **Scene Tree Queries**: Query and manipulate Godot's scene tree from Bevy
- **Resource Management**: Load and manage Godot resources (scenes, textures, etc.) from ECS via AssetServer
- **Audio System**: Dual-mode audio with one-shot sound effects and persistent audio sources
- **Node Groups Integration**: Work with Godot node groups in your Bevy systems
- **Smart Scheduling**: Physics-rate vs visual-rate system execution with proper timing
- **Godot Input Events**: Thread-safe Godot input events delivered as Bevy Events

## Projects Using `godot-bevy`

We'd love to showcase projects built with godot-bevy! If you're using this library in your game or project, please consider adding it to our [showcase](docs/SHOWCASE.md).

*Are you building something with godot-bevy? [Submit your project →](docs/SHOWCASE.md)*

## Recommended Workflow

**godot-bevy is a library for Godot developers who want to leverage Bevy's powerful ECS system within their Godot projects.** This is not a Godot plugin for Bevy users, but rather a way to bring the best of both worlds together with Godot as the foundation.

We encourage a **Godot-first approach** where you:

- **Design scenes and nodes in Godot** - Use Godot's excellent scene editor, node system, and visual tools for level design, UI, and content creation
- **Manage assets in Godot** - Import textures, audio, 3D models, and configure them using Godot's import system and project settings  
- **Use Bevy ECS for logic** - Write your game systems, components, and logic using Bevy's high-performance, data-oriented ECS
- **Consume Godot resources from ECS** - Load and use Godot-managed assets seamlessly within your Bevy systems

This approach gives you the **visual authoring power of Godot** combined with the **performance and architectural benefits of Bevy's ECS**, while maintaining a single source of truth for your game's content and configuration.

The library handles all the complex bridging between these two paradigms, so you can focus on building your game rather than managing integration details.

## Quick Start

### Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
godot-bevy = "0.4.0"
bevy = { version = "0.16.0", default-features = false }
godot = "0.2.4"
```

### Basic Usage

```rust
use bevy::prelude::*;
use godot_bevy::prelude::*;

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_systems(Update, handle_button_clicks)
        .add_systems(PhysicsUpdate, move_player);
}

// React to Godot UI signals in your ECS
fn handle_button_clicks(mut events: EventReader<GodotSignal>) {
    for signal in events.read() {
        if signal.name == "pressed" {
            println!("Button clicked! Entity: {:?}", signal.origin);
        }
    }
}

// Move player with physics timing
fn move_player(
    mut player: Query<(&Player, &mut Transform2D)>,
    mut system_delta: SystemDeltaTimer,
) {
    if let Ok((player_data, mut transform)) = player.single_mut() {
        let mut velocity = Vector2::ZERO;
        
        if Input::singleton().is_action_pressed("move_right") {
            velocity.x += 1.0;
        }
        if Input::singleton().is_action_pressed("move_left") {
            velocity.x -= 1.0;
        }
        
        if velocity.length() > 0.0 {
            velocity = velocity.normalized() * player_data.speed;
            transform.origin += velocity * system_delta.delta_seconds();
        }
    }
}
```

### Node Queries

One of godot-bevy's most powerful features is the ability to **query and interact with Godot nodes directly through Bevy's ECS**. Every Godot node becomes a Bevy entity that you can query, modify, and control.

#### Querying

```rust
use godot::classes::{Label, Button, Area2D, Node2D};
use godot_bevy::prelude::*;

// Query nodes with your custom components, then cast to specific Godot types
fn update_player_ui(
    mut player: Query<&mut GodotNodeHandle, With<Player>>,
    player_health: Query<&Health, With<Player>>,
) -> Result {
    if let (Ok(mut player_handle), Ok(health)) = (player.single_mut(), player_health.get_single()) {
        // Cast the handle to your custom Godot class
        let player_node = player_handle.get::<MyPlayerNode>();
        
        // Or cast to built-in Godot classes
        let area2d = player_handle.get::<Area2D>();
        let overlapping_bodies = area2d.get_overlapping_bodies().len();
        
        if overlapping_bodies > 0 {
            println!("Player is colliding with {} bodies", overlapping_bodies);
        }
    }
    
    Ok(())
}
```

#### Working with Node Groups and Entity Queries

```rust
// Query entities and check Godot node groups
fn handle_enemy_groups(
    mut commands: Commands, 
    entities: Query<(Entity, &mut GodotNodeHandle)>
) -> Result {
    for (entity, mut node_handle) in entities.iter() {
        let node = node_handle.get::<Node>();
        
        // Check if node is in a specific Godot group
        if node.is_in_group("enemies".into()) {
            // Add ECS components based on node groups
            commands.entity(entity).insert(Enemy { health: 100 });
        }
    }
    
    Ok(())
}
```

#### Finding Nodes by Name

```rust
// Use a helper trait to find entities by node name
fn setup_start_position(
    mut player: Query<&mut Transform2D, With<Player>>,
    entities: Query<(&Name, &mut GodotNodeHandle)>,
) -> Result {
    if let Ok(mut transform) = player.single_mut() {
        // Find the start position node by name
        let start_pos_handle = entities
            .iter()
            .find_entity_by_name("StartPosition")
            .unwrap();
            
        let start_node = start_pos_handle.get::<Node2D>();
        let position = start_node.get_position();
        
        // Set player position from Godot node
        transform.as_godot_mut().origin = position;
    }
    
    Ok(())
}
```

### Asset Management

**The library provides unified asset loading that works consistently in both development and exported games**. While Godot packages assets differently when exporting (filesystem vs .pck files), `godot-bevy` abstracts this complexity away.

#### Quick Start

Use Bevy's `AssetServer`  for modern, non-blocking asset loading:

```rust
use bevy::asset::{AssetServer, Assets, Handle};
use godot_bevy::prelude::*;

fn load_assets(asset_server: Res<AssetServer>) {
    // Load any Godot resource through Bevy's asset system (async, non-blocking)
    let scene: Handle<GodotResource> = asset_server.load("scenes/player.tscn");
    let audio: Handle<GodotResource> = asset_server.load("audio/music.ogg");
    let texture: Handle<GodotResource> = asset_server.load("art/player.png");
    
    // Works with bevy_asset_loader for loading states
}

fn use_loaded_assets(
    mut assets: ResMut<Assets<GodotResource>>,
    scene_handle: Handle<GodotResource>, // Your loaded handle
) {
    if let Some(asset) = assets.get_mut(&scene_handle) {
        // Cast to specific Godot types as needed
        if let Some(scene) = asset.try_cast::<PackedScene>() {
            // Use the scene...
        }
        if let Some(audio) = asset.try_cast::<AudioStream>() {
            // Use the audio...
        }
    }
}
```

### Audio System

The library provides a convenient audio API using Godot's audio engine

#### Quick Start

**AssetCollection Loading** (recommended):
```rust
#[derive(AssetCollection, Resource)]
struct GameAudio {
    #[asset(path = "audio/background.ogg")]
    background_music: Handle<GodotResource>,
    #[asset(path = "audio/gameover.wav")]
    death_sound: Handle<GodotResource>,
}

// Add to your loading state
app.add_loading_state(
    LoadingState::new(GameState::Loading)
        .continue_to_state(GameState::Menu)
        .load_collection::<GameAudio>()
);
```

**Direct AssetServer Loading**:
```rust
fn load_audio_on_demand(asset_server: Res<AssetServer>) {
    let music: Handle<GodotResource> = asset_server.load("audio/battle.ogg");
    // Store handle somewhere for later use
}
```

## Documentation

### Core Concepts
- **[Timing and Schedules](docs/TIMING_AND_SCHEDULES.md)** - Understanding frame timing, schedule execution, and data synchronization
- **[Input Systems](docs/INPUT_SYSTEMS.md)** - Choosing between Bevy's built-in input and Godot's bridged input system

### API Reference
For detailed API documentation, see [docs.rs/godot-bevy](https://docs.rs/godot-bevy).

## Examples

The `examples/` directory contains complete sample projects demonstrating different aspects of godot-bevy:

- **[`dodge-the-creeps-2d/`](examples/dodge-the-creeps-2d/)**: A complete 2D game showing ECS-driven gameplay, collision handling, audio system, and state management
- **[`timing-test/`](examples/timing-test/)**: Demonstrates the timing behavior and schedule execution patterns for debugging and understanding
- **[`input-event-demo/`](examples/input-event-demo/)**: Shows the thread-safe input event system and cross-platform input handling

Each example includes both Rust code and a complete Godot project ready to run.

## Inspiration and Acknowledgements

This library is inspired by and builds upon the work of [bevy_godot](https://github.com/rand0m-cloud/bevy_godot), which provided similar functionality for Godot 3. `godot-bevy` extends this concept to support Godot 4 and Bevy 0.16.

**Alternative**: If you're looking for a different approach to `godot-bevy`, check out [bevy_godot4](https://github.com/jrockett6/bevy_godot4). For a comparison of the differences between these libraries, see [Issue #2](https://github.com/dcvz/godot-bevy/issues/2).

## Version Compatibility Matrix

| `godot-bevy` | Bevy | Godot-Rust | Godot |
|------------|------|------------|-------|
| 0.4.x      | 0.16 | 0.2.4      | 4.2.x |

## License

godot-bevy is distributed under the terms of both the MIT license and the Apache License (Version 2.0).
See [LICENSE-APACHE](./LICENSE-APACHE) and [LICENSE-MIT](./LICENSE-MIT) for details. Opening a pull
request is assumed to signal agreement with these licensing terms.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
