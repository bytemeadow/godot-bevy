# Installation

This guide will walk you through setting up godot-bevy in a Godot project.

## Prerequisites

Before you begin, ensure you have:

- **Rust 1.87.0 or later** - [Install Rust](https://rustup.rs/)
- **Godot 4.3** - [Download Godot](https://godotengine.org/download)
- Basic familiarity with both Rust and Godot

## Create a New Project

### 1. Set Up Godot Project

First, create a new Godot project through the Godot editor:

1. Open Godot and click "New Project"
2. Choose a project name and location
3. Select "Compatibility" renderer for maximum platform support
4. Click "Create & Edit"

### 2. Set Up Rust Project

In your Godot project directory, create a new Rust library:

```bash
cd /path/to/your/godot/project
cargo init --lib rust
cd rust
```

### 3. Configure Cargo.toml

Edit `rust/Cargo.toml`:

```toml
[package]
name = "your_game_name"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
godot-bevy = "0.6.2"
bevy = { version = "0.16", default-features = false }
godot = "0.3"
```

## Configure Godot Integration

### 1. Create Extension File

Create `rust.gdextension` in your Godot project root:

```ini
[configuration]
entry_symbol = "gdext_rust_init"
compatibility_minimum = 4.3
reloadable = true

[libraries]
macos.debug = "res://rust/target/debug/libyour_game_name.dylib"
macos.release = "res://rust/target/release/libyour_game_name.dylib"
windows.debug.x86_32 = "res://rust/target/debug/your_game_name.dll"
windows.release.x86_32 = "res://rust/target/release/your_game_name.dll"
windows.debug.x86_64 = "res://rust/target/debug/your_game_name.dll"
windows.release.x86_64 = "res://rust/target/release/your_game_name.dll"
linux.debug.x86_64 = "res://rust/target/debug/libyour_game_name.so"
linux.release.x86_64 = "res://rust/target/release/libyour_game_name.so"
linux.debug.arm64 = "res://rust/target/debug/libyour_game_name.so"
linux.release.arm64 = "res://rust/target/release/libyour_game_name.so"
linux.debug.rv64 = "res://rust/target/debug/libyour_game_name.so"
linux.release.rv64 = "res://rust/target/release/libyour_game_name.so"
```

Replace `your_game_name` with your actual crate name from `Cargo.toml`.

### 2. Create BevyApp Autoload

1. In Godot, create a new scene
2. Add a `BevyApp` node as the root
3. Save it as `bevy_app_singleton.tscn`
4. Go to Project → Project Settings → Autoload
5. Add the scene with name "BevyAppSingleton"

## Write Your First Code

Edit `rust/src/lib.rs`:

```rust
use bevy::prelude::*;
use godot_bevy::prelude::*;

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_systems(Startup, hello_world);
}

fn hello_world() {
    godot::prelude::godot_print!("Hello from godot-bevy!");
}
```

## Build and Run

### 1. Build the Rust Library

```bash
cd rust
cargo build
```

### 2. Run in Godot

1. Return to the Godot editor
2. Press F5 or click the play button
3. You should see "Hello from godot-bevy!" in the output console

## Troubleshooting

### Common Issues

**"Can't open dynamic library"**
- Ensure the paths in `rust.gdextension` match your library output
- Check that you've built the Rust project
- On macOS, you may need to allow the library in System Preferences

**"BevyApp not found"**
- Make sure godot-bevy is properly added to your dependencies
- Rebuild the Rust project
- Restart the Godot editor

**Build errors**
- Verify your Rust version: `rustc --version`
- Ensure all dependencies are compatible
- Check for typos in the crate name

## Next Steps

Congratulations! You've successfully set up godot-bevy. Continue to [Your First Project](./first-project.md) to build something more substantial.