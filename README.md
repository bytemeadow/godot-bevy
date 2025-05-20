# Godot-Bevy

Godot-Bevy is a Rust library that brings [Bevy's](https://bevyengine.org/) powerful Entity Component System (ECS) to the versatile [Godot Game Engine](https://godotengine.org/). Use Bevy's ergonomic and high-performance Rust ECS within your Godot projects to get the best of both worlds.

## Features

- Seamlessly integrate Bevy ECS in Godot 4 projects
- Use Bevy systems to control Godot nodes
- Spawn Godot scenes from Bevy
- Maintain clean separation between ECS logic and Godot's scene tree
- Leverage the full power of Bevy's Rust-based ECS while using Godot's editor and rendering capabilities
- Systems can be scheduled for the visual or physics frame

## Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
godot-bevy = "0.1.0"
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
    app.add_systems(Update, my_system);
}

fn my_system() {
    println!("Hello from Bevy!");
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

Check out the examples directory for complete sample projects.

## Inspiration and Acknowledgements

This library is inspired by and builds upon the work of [bevy_godot](https://github.com/rand0m-cloud/bevy_godot), which provided similar functionality for Godot 3. Godot-Bevy extends this concept to support Godot 4 and Bevy 0.16.

## Version Compatibility Matrix

| Godot-Bevy | Bevy | Godot-Rust | Godot |
|------------|------|------------|-------|
| 0.1.x      | 0.16 | 0.2.4      | 4.2.x |

## License

godot-bevy is distributed under the terms of both the MIT license and the Apache License (Version 2.0).
See [LICENSE-APACHE](./LICENSE-APACHE) and [LICENSE-MIT](./LICENSE-MIT) for details. Opening a pull
request is assumed to signal agreement with these licensing terms.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
