# Godot-Bevy Integration Plugin

This Godot editor plugin provides tools to seamlessly integrate Bevy ECS with your Godot 4 projects.

## Features

### 🚀 Project Scaffolding
- **One-click Rust project setup** - Creates complete Cargo.toml, lib.rs, and build scripts
- **Customizable plugin selection** - Choose between GodotDefaultPlugins or pick individual features
- **Platform-aware configuration** - Generates correct .gdextension file for all platforms

### 🎮 BevyApp Singleton Management
- **Automatic registration** - BevyAppSingleton is automatically added to autoload when plugin is enabled
- **No manual setup required** - Singleton is managed by the plugin lifecycle
- **Scene-based configuration** - Easy to modify and extend

### 🛠️ Development Tools
- **Build script generation** - Creates platform-specific build scripts
- **Feature flag management** - Visual selection of godot-bevy features

## Installation

1. Copy the `addons/godot-bevy` folder to your Godot project's `addons` directory
2. Enable the plugin in Project Settings > Plugins
3. The BevyAppSingleton will be automatically registered as an autoload

## Usage

### Setting Up a New Project

1. Go to **Project > Tools > Setup Godot-Bevy Project**
2. Configure your project:
   - Project name (used for Rust crate name)
   - godot-bevy version
   - Plugin configuration (defaults or custom)
   - Build type (debug/release)
3. Click "Create Project"

The plugin will:
- Create a `rust/` directory with your Bevy project
- Generate Cargo.toml with selected features
- Create a starter lib.rs with example system
- Set up the .gdextension file
- Build the Rust project automatically
- Restart the editor to apply changes

### BevyApp Singleton

The BevyAppSingleton is automatically managed:
- **Automatically registered** when the plugin is enabled
- **Automatically removed** when the plugin is disabled
- **No manual setup required** - just enable the plugin!

## Project Structure

```
your_project/
├── addons/
│   └── godot-bevy/
│       ├── bevy_app_singleton.tscn (auto-registered)
│       └── ... (plugin files)
├── rust/
│   ├── Cargo.toml
│   └── src/
│       └── lib.rs
└── rust.gdextension
```

## Building Your Project

After setup, build your Rust project:

```bash
# Linux/macOS
./build.sh

# Windows
build.bat

# Or manually
cd rust
cargo build --release
```

## Customization

The generated lib.rs includes:
- Basic Bevy app setup with selected plugins
- Example "Hello World" system
- Proper GDExtension initialization

Modify it to add your game logic!

## Plugin Configuration

When not using defaults, you can individually select:
- `GodotAssetsPlugin` - Asset loading through Bevy
- `GodotTransformSyncPlugin` - Transform synchronization
- `GodotCollisionsPlugin` - Collision detection
- `GodotSignalsPlugin` - Signal event bridge
- `BevyInputBridgePlugin` - Bevy input API
- `GodotAudioPlugin` - Audio system
- `GodotPackedScenePlugin` - Scene spawning
- `bevy_gamepad` - Gamepad support feature flag