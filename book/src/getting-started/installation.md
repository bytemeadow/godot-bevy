# Installation

This guide will walk you through setting up godot-bevy in a Godot project.

## Prerequisites

Before you begin, ensure you have:

- **Rust 1.87.0 or later** - [Install Rust](https://rustup.rs/)
- **Godot 4.3** - [Download Godot](https://godotengine.org/download)
- Basic familiarity with both Rust and Godot

## Installation Methods

There are two ways to set up godot-bevy in your project:

1. **[Plugin Installation (Recommended)](#plugin-installation)** - Use the godot-bevy editor plugin for automatic setup
2. **[Manual Installation](#manual-installation)** - Set up the project manually

## Plugin Installation

The easiest way to get started is using the godot-bevy editor plugin, which automatically generates the Rust project and configures the BevyApp singleton.

### 1. Install the Plugin

1. Download the `addons/godot-bevy` folder from the [godot-bevy repository](https://github.com/bytemeadow/godot-bevy)
2. Copy it to your Godot project's `addons/` directory
3. In Godot, go to **Project > Project Settings > Plugins**
4. Enable the "Godot-Bevy Integration" plugin

### 2. Create Your Project

1. Go to **Project > Tools > Setup godot-bevy Project**
2. Configure your project settings:
   - **Project name**: Used for the Rust crate name
   - **godot-bevy version**: Library version (default: 0.9.0)  
   - **Release build**: Whether to build in release mode initially
3. Click **"Create Project"**

The plugin will automatically:
- Create a `rust/` directory with Cargo.toml and lib.rs
- Generate the `.gdextension` file with correct platform paths
- Create and register the BevyApp singleton scene
- Build the Rust project
- Restart the editor to apply changes

### 3. Run Your Project

After the editor restarts:
1. Press **F5** or click the play button
2. You should see "Hello from Bevy ECS!" in the output console every second

The generated `rust/src/lib.rs` includes a complete example.

### 4. Plugin Features

The plugin provides additional useful features:

- **Add BevyApp Singleton Only**: If you already have a Rust project, use **Project > Tools > Add BevyApp Singleton** to just create and register the singleton
- **Build Rust Project**: Use **Project > Tools > Build Rust Project** to rebuild without restarting the editor
- **Bulk Transform Optimization**: The generated singleton includes optimized bulk transform methods that godot-bevy automatically detects and uses for better performance

## Manual Installation

If you prefer to set up everything manually, follow these steps:

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
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
godot-bevy = "0.8.4"
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
4. Go to Project → Project Settings → Globals → Autoload
5. Add the scene with name "BevyAppSingleton"

## Write Your First Code

Edit `rust/src/lib.rs`:

```rust
use godot::prelude::*;
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

### Plugin Installation Issues

**"Plugin not found" or "Plugin failed to load"**

- Ensure the `addons/godot-bevy` folder is in the correct location
- Check that all plugin files are present (plugin.cfg, plugin.gd, etc.)
- Restart the Godot editor after copying the plugin files

**"Setup godot-bevy Project" menu item missing**

- Verify the plugin is enabled in Project Settings > Plugins
- Check the Godot console for plugin error messages
- Try disabling and re-enabling the plugin

**Plugin setup fails or hangs**

- Ensure you have `cargo` installed and available in your system PATH
- Check that you have write permissions in the project directory
- Look for error messages in the Godot output console

### Manual Installation Issues

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

Congratulations! You've successfully set up godot-bevy using either the plugin or manual installation method. 

The plugin installation automatically includes the new bulk transform API optimizations, while manual installations can add these by updating their BevyApp singleton scene.

Continue to [Basic Concepts](./basic-concepts.md) to learn more about godot-bevy's architecture and capabilities.
