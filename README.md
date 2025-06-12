# godot-bevy

[![Discord](https://img.shields.io/discord/1379465862800736258.svg?color=7289da&&logo=discord)](https://discord.gg/gqkeBsH93H)
[![Current Crates.io Version](https://img.shields.io/crates/v/godot-bevy.svg)](https://crates.io/crates/godot-bevy)
[![Documentation](https://img.shields.io/badge/docs-latest-blue)](https://docs.rs/godot-bevy/latest/godot_bevy/)
[![Book](https://img.shields.io/badge/book-read-green)](https://godot-rust.github.io/godot-bevy)
[![Test Status](https://github.com/godot-rust/godot-bevy/actions/workflows/ci.yml/badge.svg)](https://github.com/godot-rust/godot-bevy/actions/workflows/ci.yml)

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

**[Read the godot-bevy Book →](https://dcvz.github.io/godot-bevy)**

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
godot-bevy = "0.6.2"
bevy = { version = "0.16", default-features = false }
godot = "0.3.0"
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
    // (https://github.com/dcvz/godot-bevy/blob/main/docs/TIMING_AND_SCHEDULES.md).
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

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md).

### Development Setup

1. Fork and clone the repository
2. Install Rust 1.87+ and Godot 4.3
3. Run tests: `cargo test`
4. Build examples: `cargo build --examples`

## 📄 License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.

## 🙏 Acknowledgments

Special thanks to:
- The [Bevy](https://bevyengine.org/) team for their amazing ECS
- The [godot-rust](https://godot-rust.github.io/) team for the Godot bindings
- [Blaze](https://runblaze.dev) for CI runner support
- Our [contributors](https://github.com/godot-rust/godot-bevy/graphs/contributors) and community