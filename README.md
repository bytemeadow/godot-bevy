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
- **Resource Management**: Load and manage Godot resources (scenes, textures, etc.) from ECS
- **Audio System**: Dual-mode audio with one-shot sound effects and persistent audio sources
- **Node Groups Integration**: Work with Godot node groups in your Bevy systems
- **Smart Scheduling**: Physics-rate vs visual-rate system execution with proper timing
- **Godot Input Events**: Thread-safe Godot input events delivered as Bevy Events

## Quick Start

### Installation

Add the following to your `Cargo.toml`:

```toml
[dependencies]
godot-bevy = "0.4.0"
bevy = { version = "0.16", default-features = false }
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

### Project Setup

1. **Add a `BevyAppSingleton` autoload** in your Godot project settings, which is a Godot scene containing a single root node of type BevyApp, which you can see examples of in `examples/`.
2. **Interact with Godot from Bevy**:

```rust
fn spawn_godot_scene(mut commands: Commands) {
    commands.spawn(GodotScene::from_path("res://my_scene.tscn")
        .with_translation3d(Vector3::new(0.0, 1.0, 0.0)));
}
```

### Asset Management

**The library provides unified asset loading that works consistently in both development and exported games**. While Godot packages assets differently when exporting (filesystem vs .pck files), `godot-bevy` abstracts this complexity away.

Use the `GodotResourceLoader` for all asset loading needs:

```rust
fn load_assets(godot_loader: Res<GodotResourceLoader>) {
    // Load any Godot resource type - works identically in development and exported games
    if let Some(texture) = godot_loader.load::<ImageTexture>("art/player.png") {
        // Use the texture...
    }
    
    if let Some(audio) = godot_loader.load::<AudioStream>("audio/music.ogg") {
        // Use the audio...
    }
    
    // Check if resources exist
    if godot_loader.exists("scenes/enemy.tscn") {
        // Resource is available
    }
}
```

**Key Benefits:**
- Works identically in development and exported games
- Supports all Godot resource types (textures, audio, scenes, etc.)
- Provides `exists()` method for robust error handling
- Leverages Godot's native resource system

The `GodotAssetsPlugin` provides the `GodotResourceLoader` resource for seamless integration with Godot's asset pipeline.

### Audio System

The library provides a convenient audio API using Godot's audio engine that works identically in development, Godot editor, and exported games.

#### Key Features
- **Preloaded assets** - Load once, play multiple times for efficiency
- **Direct play** - Convenient one-shot sound loading and playing
- **Sound management** - Control playing sounds (stop, check status, etc.)
- **Looping support** - Automatic loop configuration for background music
- **Volume and pitch control** - Full audio parameter control

#### Quick Start

```rust
use godot_bevy::prelude::*;

fn audio_system(
    mut audio: ResMut<AudioManager>,
    godot_loader: Res<GodotResourceLoader>,
) {
    // Preload assets for efficiency (recommended for repeated sounds)
    let music_handle = audio.load("audio/background.ogg", &godot_loader).unwrap();
    let sound_id = audio.play_handle_with_settings(
        &music_handle,
        SoundSettings::new().volume(0.5).looped()
    ).unwrap();
    
    // Direct play (convenient for occasional sounds)
    audio.play_with_settings(
        "audio/jump.wav", 
        SoundSettings::new().volume(0.8)
    ).unwrap();
    
    // Control playing sounds
    if audio.is_playing(sound_id) {
        audio.stop(sound_id).unwrap();
    }
}
```

#### Audio Patterns

**Preloaded Assets** (efficient for repeated sounds):
```rust
// Load once during startup
fn load_audio_assets(
    mut audio: ResMut<AudioManager>,
    godot_loader: Res<GodotResourceLoader>,
) {
    let music_handle = audio.load("audio/background.ogg", &godot_loader).unwrap();
    // Store handle in a resource for later use
}

// Play multiple times with no loading overhead
fn play_background_music(mut audio: ResMut<AudioManager>, music_handle: Res<AudioHandle>) {
    audio.play_handle_with_settings(&music_handle, SoundSettings::new().looped()).unwrap();
}
```

**Direct Play** (convenient for one-offs):
```rust
fn play_sound_effects(mut audio: ResMut<AudioManager>) {
    // Simple one-shot sounds
    audio.play("audio/jump.wav").unwrap();
    audio.play("audio/coin.ogg").unwrap();
    
    // With custom settings
    audio.play_with_settings(
        "audio/explosion.wav",
        SoundSettings::new().volume(0.7).pitch(1.2)
    ).unwrap();
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