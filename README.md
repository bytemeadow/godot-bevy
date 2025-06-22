# godot-bevy

[![Discord](https://img.shields.io/discord/1379465862800736258.svg?color=7289da&&logo=discord)](https://discord.gg/gqkeBsH93H)
[![Current Crates.io Version](https://img.shields.io/crates/v/godot-bevy.svg)](https://crates.io/crates/godot-bevy)
[![Documentation](https://img.shields.io/badge/docs-latest-blue)](https://docs.rs/godot-bevy/latest/godot_bevy/)
[![Book](https://img.shields.io/badge/book-read-green)](https://godot-rust.github.io/godot-bevy)
[![Test Status](https://github.com/bytemeadow/godot-bevy/actions/workflows/ci.yml/badge.svg)](https://github.com/bytemeadow/godot-bevy/actions/workflows/ci.yml)
[![Rust Version](https://img.shields.io/badge/Rust-1.87.0+-blue)](https://releases.rs/docs/1.87.0)
![license](https://shields.io/badge/license-MIT%2FApache--2.0-blue)

**godot-bevy** brings Bevy's powerful ECS to Godot, allowing you to write high-performance game logic in Rust while leveraging Godot's excellent editor and rendering capabilities.

---

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

## 📚 Documentation

**[Read the godot-bevy Book →](https://bytemeadow.github.io/godot-bevy)**

The book covers everything you need to know:
- Installation and setup
- Core concepts and architecture
- Transform system and physics
- Input handling
- Examples and best practices

## 🚀 Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
godot-bevy = "0.7.0"
bevy = { version = "0.16", default-features = false }
godot = "0.3"
```

Basic example:

```rust
use bevy::prelude::*;
use godot_bevy::prelude::*;

#[bevy_app]
fn build_app(app: &mut App) {
    // Print to the Godot console:
    // (https://docs.rs/godot-core/0.3.1/godot_core/macro.godot_print.html)
    godot_print!("Hello from Godot-Bevy!");

    // This line runs the `position_system` function every Godot render frame.
    //
    // Read more about Bevy "Systems" here:
    // (https://bevy.org/learn/quick-start/getting-started/ecs/).
    //
    // The `Update` schedule parameter is provided by Godot-Bevy.
    // It runs the system during Godot's `_process` update cycle.
    //
    // Read more about other schedules provided by Godot-Bevy here:
    // (https://github.com/bytemeadow/godot-bevy/blob/main/docs/TIMING_AND_SCHEDULES.md).
    app.add_systems(Update, position_system);
}

// A system is a normal Rust function. This system moves all Node2Ds to the right, such as Sprite2Ds.
//
// The `transform` parameter is a Bevy `Query` that matches all `Transform2D` components.
// `Transform2D` is a Godot-Bevy-provided component that matches all Node2Ds in the scene.
// (https://docs.rs/godot-bevy/latest/godot_bevy/plugins/core/transforms/struct.Transform2D.html)
//
// For more information on Bevy Components, Systems, and Queries, see:
// (https://bevy.org/learn/quick-start/getting-started/ecs/).
fn position_system(mut transform: Query<&mut Transform2D>) {
    // For single matches, you can use `single_mut()` instead:
    // `if let Ok(mut transform) = transform.single_mut() {`
    for mut transform in transform.iter_mut() {
        // Move the node to the right.
        transform.as_godot_mut().origin.x += 1.0;
    }
}

```

## 🎮 Examples

Check out the [examples](./examples) directory:
- **[2D Platformer](./examples/platformer-2d)** - Physics-based platformer
- **[Dodge the Creeps](./examples/dodge-the-creeps-2d)** - Classic arcade game
- **[Simple Movement](./examples/simple-node2d-movement)** - Basic transform usage

## ✨ Inspiration and Acknowledgements

This library is inspired by and builds upon the work of [bevy_godot](https://github.com/rand0m-cloud/bevy_godot), which provided similar functionality for Godot 3. `godot-bevy` extends this concept to support Godot 4 and Bevy 0.16.

**Alternative**: If you're looking for a different approach to `godot-bevy`, check out [bevy_godot4](https://github.com/jrockett6/bevy_godot4). For a comparison of the differences between these libraries, see [Issue #2](https://github.com/bytemeadow/godot-bevy/issues/2).

## ⊹ Version Compatibility Matrix

| `godot-bevy` | Bevy | Godot-Rust | Godot |
|------------|------|------------|-------|
| 0.7.x      | 0.16 | 0.3      | 4.4.x |

## 🦀 MSRV

The minimum supported Rust version is 1.87.0.

The MSRV is the minimum Rust version that can be used to compile each crate.

## 📕 License

godot-bevy is distributed under the terms of both the MIT license and the Apache License (Version 2.0).
See [LICENSE-APACHE](./LICENSE-APACHE) and [LICENSE-MIT](./LICENSE-MIT) for details. Opening a pull
request is assumed to signal agreement with these licensing terms.

## 🤝 Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
