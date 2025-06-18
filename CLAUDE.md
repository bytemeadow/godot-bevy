# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is `godot-bevy`, a Rust library that bridges Bevy's Entity Component System (ECS) with Godot 4. The project enables Rust developers to leverage Bevy's high-performance ECS within Godot projects, creating a powerful combination of Godot's visual authoring tools with Bevy's data-oriented architecture.

## Development Commands

### Build and Test
```bash
# Format code (run before commits)
cargo fmt --all

# Lint code (must pass CI)
cargo clippy --all-targets --all-features

# Run tests
cargo test

# Build release version
cargo build --release
```

### Example Projects
```bash
# Build a specific example (replace {example} with project name)
cargo build --release --manifest-path examples/{example}/rust/Cargo.toml

# Build performance test with validation
./examples/boids-perf-test/build.sh
```

## Architecture Overview

### Core Components

**BevyApp** (`godot-bevy/src/app.rs`): The central bridge between Godot and Bevy. This Godot node (`BevyApp`) hosts the entire Bevy App instance and coordinates between Godot's frame lifecycle and Bevy's ECS update cycles.

**Dual Schedule System**: The library runs two separate Bevy schedules:
- `Update` schedule runs during Godot's `_process()` at display framerate
- `PhysicsUpdate` schedule runs during Godot's `_physics_process()` at fixed physics rate (60Hz)

**Bridge System** (`godot-bevy/src/bridge/`): Manages bidirectional communication between Godot nodes and Bevy entities:
- `GodotNodeHandle` - Bevy component that provides access to Godot nodes from ECS
- `GodotResourceHandle` - Manages Godot resources within Bevy's asset system
- Automatic transform synchronization between Bevy and Godot coordinate systems

**NodeRegistry** (`godot-bevy/src/node_registry.rs`): Eliminates ECS query conflicts when accessing multiple node types:
- Maps entities to their `GodotNodeHandle` instances for fast, conflict-free access
- Provides `access()` (panicking) and `try_access()` (safe) methods for node retrieval
- Automatically registers entities during scene tree parsing and cleans up on node removal
- Enables multiple systems to access different node types without `ParamSet` workarounds

**Watchers** (`godot-bevy/src/watchers/`): Thread-safe event bridges:
- `SceneTreeWatcher` - Monitors Godot scene tree changes
- `GodotSignalWatcher` - Converts Godot signals to Bevy events  
- `GodotInputWatcher` - Bridges Godot input events to Bevy

### Plugin Architecture

**GodotPlugin**: Main plugin that registers all core systems and auto-discovers `AutoSyncBundle` plugins for custom Godot node types.

**Audio System** (`godot-bevy/src/plugins/audio/`): Channel-based audio API with spatial audio support using Godot's audio engine.

**Asset Management** (`godot-bevy/src/plugins/assets.rs`): Unified asset loading that abstracts differences between development and exported game environments.

### AutoSync System

The `autosync` system (`godot-bevy/src/autosync.rs`) automatically registers custom Godot node types with their corresponding Bevy bundles using the `#[derive(BevyBundle)]` macro, enabling seamless integration between Godot editor-placed nodes and ECS components.

## Development Workflow

### Godot-First Approach
The library is designed for a Godot-first workflow:
1. Design scenes and place nodes in Godot editor
2. Define custom Godot node classes with `#[derive(BevyBundle)]` 
3. Write game logic as Bevy systems that operate on these entities
4. Use Godot for asset management, import settings, and visual authoring

### Working with Examples
Examples are structured as workspace members with separate Rust crates. Each example contains:
- `/rust/` - Bevy systems and game logic
- `/godot/` - Godot project with scenes and assets
- `BevyAppSingleton` autoload scene as the ECS entry point

## Key Integration Points

**Transform Synchronization**: Automatic bidirectional sync between Bevy `Transform2D`/`Transform3D` components and Godot node transforms.

**Signal Integration**: Godot signals become Bevy events via `EventReader<GodotSignal>`, enabling ECS systems to respond to UI interactions and game events.

**Node Queries**: Query Godot nodes directly from Bevy systems using `Query<&mut GodotNodeHandle>` and cast to specific Godot types.

**Asset Loading**: Use Bevy's `AssetServer` to load Godot resources (`Handle<GodotResource>`) which works consistently in development and exported games.

## ECS Query Patterns and Best Practices

### NodeRegistry vs GodotNodeHandle Queries

**NodeRegistry Pattern** (Recommended for multiple node types):
```rust
fn update_multiple_types(
    sprites: Query<Entity, (With<Sprite2DMarker>, With<GodotNodeHandle>)>,
    buttons: Query<Entity, (With<ButtonMarker>, With<GodotNodeHandle>)>,
    registry: NodeRegistryAccess,
) {
    for entity in sprites.iter() {
        let sprite = registry.access::<Sprite2D>(entity);
        // Work with sprite...
    }
    for entity in buttons.iter() {
        let button = registry.access::<Button>(entity);
        // Work with button...
    }
}
```

**Direct GodotNodeHandle Pattern** (Fine for single node types):
```rust
fn update_sprites(
    mut sprites: Query<&mut GodotNodeHandle, With<Sprite2DMarker>>,
) {
    for mut handle in sprites.iter_mut() {
        let sprite = handle.get::<Sprite2D>();
        // Work with sprite...
    }
}
```

### Query Conflict Resolution

- **Problem**: Multiple `Query<&mut GodotNodeHandle>` parameters with different markers cause Bevy ECS conflicts
- **Solution**: Use `NodeRegistryAccess` with entity-based queries instead of handle-based queries
- **Always include `With<GodotNodeHandle>`** when using NodeRegistry to ensure entities are ready

### Timing Considerations

**Scene Tree Initialization**: Entities created during `PreStartup` are available in `Startup` systems.

**Dynamic Scene Spawning**: When using `GodotScene::from_handle()`, there's a frame delay:
- Frame N: Entity created with `GodotScene` component  
- Frame N: `spawn_scene` system creates Godot node, adds `GodotNodeHandle`
- Frame N+1: Entity registered in NodeRegistry via scene tree events

**Best Practice**: Always use `With<GodotNodeHandle>` filter when working with NodeRegistry to avoid timing issues.

## Testing and CI

The project uses GitHub Actions CI that runs on Linux, macOS, and Windows:
- Code formatting with `cargo fmt`
- Linting with `cargo clippy` (warnings treated as errors)
- Full test suite with `cargo test`
- Release builds for all platforms
- Example project builds and Godot exports

CI configuration is in `.github/workflows/ci.yml` and must pass for all PRs.