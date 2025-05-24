# Godot-Bevy

Godot-Bevy is a Rust library that brings [Bevy's](https://bevyengine.org/) powerful Entity Component System (ECS) to the versatile [Godot Game Engine](https://godotengine.org/). Use Bevy's ergonomic and high-performance Rust ECS within your Godot projects to get the best of both worlds.

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
- **Resource Management**: Load and manage Godot resources (scenes, textures, etc.) from ECS
- **Node Groups Integration**: Work with Godot node groups in your Bevy systems
- **Smart Scheduling**: Physics-rate vs visual-rate system execution with proper timing

## Timing and Schedules

Godot-Bevy provides a clean integration with Godot's frame timing:

- **Visual Frame (`_process`)**: Runs `app.update()` with all standard Bevy schedules
  - `Update`, `FixedUpdate`, `PreUpdate`, `PostUpdate`, etc.
  - Runs at Godot's visual framerate (typically 60-120 FPS)

- **Physics Frame (`_physics_process`)**: Runs the `PhysicsUpdate` schedule only
  - For systems that need to sync with Godot's physics timing
  - Runs at Godot's physics tickrate (typically 60 Hz)

### Usage Guidelines

```rust
// Game logic that must run once per render frame
app.add_systems(Update, my_gameplay_system);

// Game logic - Bevy's built-in fixed timestep
app.add_systems(FixedUpdate, my_physics_simulation);

// Godot-specific physics - synchronized with Godot's physics
app.add_systems(PhysicsUpdate, godot_movement_system);
```

**Note**: Systems in `PhysicsUpdate` should use `SystemDeltaTimer` for accurate delta time, while systems in standard schedules use `Res<Time>`.

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
godot-bevy = "0.3.0"
bevy = "0.16"
godot = "0.2.4"
```

## Getting Started

### 1. Create a Bevy App

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

### 2. Set up the Godot project

Add a `BevyAppSingleton` autoload in your Godot project settings that has the `BevyApp` node.

### 3. Interact with Godot from Bevy

```rust
fn spawn_godot_scene(mut commands: Commands) {
    commands.spawn(GodotScene::from_path("res://my_scene.tscn")
        .with_translation3d(Vector3::new(0.0, 1.0, 0.0)));
}
```

## Documentation

For detailed documentation and examples, see the [API documentation](https://docs.rs/godot-bevy).

## Examples

The `examples/` directory contains complete sample projects demonstrating different aspects of godot-bevy:

- **`dodge-the-creeps-2d/`**: A complete 2D game showing ECS-driven gameplay, collision handling, and state management
- **`timing-test/`**: Demonstrates the timing behavior and schedule execution patterns for debugging and understanding

Each example includes both Rust code and a complete Godot project ready to run.

## Inspiration and Acknowledgements

This library is inspired by and builds upon the work of [bevy_godot](https://
github.com/rand0m-cloud/bevy_godot), which provided similar functionality for 
Godot 3. Godot-Bevy extends this concept to support Godot 4 and Bevy 0.16.

## Version Compatibility Matrix

| Godot-Bevy | Bevy | Godot-Rust | Godot |
|------------|------|------------|-------|
| 0.3.x      | 0.16 | 0.2.4      | 4.2.x |

## License

godot-bevy is distributed under the terms of both the MIT license and the Apache License (Version 2.0).
See [LICENSE-APACHE](./LICENSE-APACHE) and [LICENSE-MIT](./LICENSE-MIT) for details. Opening a pull
request is assumed to signal agreement with these licensing terms.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
