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

**Watchers** (`godot-bevy/src/watchers/`): Thread-safe event bridges:
- `SceneTreeWatcher` - Monitors Godot scene tree changes
- `GodotSignalWatcher` - Converts Godot signals to Bevy events  
- `GodotInputWatcher` - Bridges Godot input events to Bevy

### Plugin Architecture

**Opt-in Plugin System**: Following Bevy's philosophy, godot-bevy now provides granular plugin control. By default, only minimal core functionality is included.

- **`GodotPlugin`**: Now minimal by default - only includes `GodotCorePlugins` (scene tree, assets, basic setup)
- **`GodotCorePlugins`**: Minimal required functionality 
- **`GodotDefaultPlugins`**: All functionality enabled (use for easy migration)
- **Individual plugins**: 
  - `GodotTransformsPlugin` (move/position nodes from Bevy)
  - `GodotAudioPlugin` (play sounds/music from Bevy) 
  - `GodotSignalsPlugin` (respond to Godot signals in Bevy)
  - `GodotCollisionsPlugin` (detect collisions in Bevy)
  - `GodotInputEventPlugin` (handle input from Godot)
  - `BevyInputBridgePlugin` (use Bevy's input API)
  - `GodotPackedScenePlugin` (spawn scenes dynamically)

**Example usage:**
```rust
// Default (minimal) - only core functionality
#[bevy_app]
fn build_app(app: &mut App) {
    // GodotPlugin is already added by #[bevy_app]
    // Only scene tree and assets are available
}

// Add specific features as needed
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotTransformsPlugin)      // Transform sync
        .add_plugins(GodotAudioPlugin)          // Audio system
        .add_plugins(BevyInputBridgePlugin);    // Input (auto-includes GodotInputEventPlugin)
}

// Everything (for easy migration from older versions)
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotDefaultPlugins);
}
```

**Breaking Change**: `GodotPlugin` now only includes core functionality by default. If your code stops working after upgrading, add `app.add_plugins(GodotDefaultPlugins)` for the old behavior, or better yet, add only the specific plugins you need.

**Audio System** (`godot-bevy/src/plugins/audio/`): Channel-based audio API with spatial audio support using Godot's audio engine. Add with `GodotAudioPlugin`.

**Asset Management** (`godot-bevy/src/plugins/assets.rs`): Unified asset loading that abstracts differences between development and exported game environments. Always included in `GodotCorePlugins`.

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

**Transform Synchronization**: Automatic synchronization between Bevy `Transform` components and Godot node transforms. You can select for this synchronization to be disabled, just sync Bevy Transforms to Godot Transforms, or sync bi-directionally.

**Signal Integration**: Godot signals become Bevy events via `EventReader<GodotSignal>`, enabling ECS systems to respond to UI interactions and game events.

**Node Queries**: Query Godot nodes directly from Bevy systems using `Query<&mut GodotNodeHandle>` and cast to specific Godot types.

**Asset Loading**: Use Bevy's `AssetServer` to load Godot resources (`Handle<GodotResource>`) which works consistently in development and exported games.

## Testing and CI

The project uses GitHub Actions CI that runs on Linux, macOS, and Windows:
- Code formatting with `cargo fmt`
- Linting with `cargo clippy` (warnings treated as errors)
- Full test suite with `cargo test`
- Release builds for all platforms
- Example project builds and Godot exports

CI configuration is in `.github/workflows/ci.yml` and must pass for all PRs.

### Scene Tree Testing

The scene tree integration can be tested using `godot-bevy-testability` even in headless mode. Key findings:

**What Works:**
- Node creation automatically creates corresponding Bevy entities with `GodotNodeHandle` components
- Entity-node mapping works correctly via `InstanceId`
- Scene tree events are processed and entities are created/destroyed appropriately
- Tests can verify component synchronization (Name, markers, etc.)

**Testing Approach:**
- Use `BevyGodotTestContextExt::setup_full_integration()` to initialize the scene tree plugin
- Create nodes with `env.add_node_to_scene()` helper
- Verify entities with queries for `GodotNodeHandle` components
- Test both creation and destruction lifecycles

**Headless Mode Limitations:**
- `node.is_inside_tree()` returns `false` in headless test environment
- Root Window's internal tree pointer is null, but scene tree events still work
- Direct Godot signals may not fire, but the plugin's event system functions correctly

**Example Test Pattern:**
```rust
fn test_node_creates_entity(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    let mut env = ctx.setup_full_integration();
    let mut node = godot::classes::Node3D::new_alloc();
    env.add_node_to_scene(node.clone());
    ctx.app.update();

    // Verify entity was created
    let entity = find_entity_for_node(ctx, node.instance_id())
        .expect("Entity should be created");
    // Test components...
}
```

## Performance Best Practices

### PackedArray Pattern for Maximum Performance

When transferring bulk data between Rust and GDScript, always use PackedArrays instead of individual Variant conversions to avoid expensive FFI calls.

**Pattern:**
1. **GDScript side**: Collect data into PackedArrays
2. **Transfer**: Pass PackedArrays via single `call()` or return in Dictionary
3. **Rust side**: Process PackedArrays directly

**Example Implementation:**
```rust
// Rust: Collect data in Vec/slice, convert to PackedArray
let ids = PackedInt64Array::from(instance_ids.as_slice());
let positions = PackedVector3Array::from(positions.as_slice());
watcher.call("bulk_update", &[ids.to_variant(), positions.to_variant()]);

// Rust: Process received PackedArrays
let result_dict = watcher.call("analyze_data", &[]).to::<Dictionary>();
let ids = result_dict.get("ids").unwrap().to::<PackedInt64Array>();
let types = result_dict.get("types").unwrap().to::<PackedStringArray>();
for i in 0..ids.len() {
    if let (Some(id), Some(type_name)) = (ids.get(i), types.get(i)) {
        // Process data efficiently
    }
}
```

**GDScript Pattern:**
```gdscript
# Collect into PackedArrays
var ids = PackedInt64Array()
var types = PackedStringArray()
for node in nodes:
    ids.append(node.get_instance_id())
    types.append(node.get_class())

# Return as Dictionary with PackedArrays
return {"ids": ids, "types": types}
```

**Performance Benefits:**
- Eliminates per-element Variant conversion FFI calls
- Reduces N×FFI to 1×FFI (where N = number of elements)  
- Can achieve 10-50x performance improvement for bulk operations
- Used extensively in transform sync system and optimized scene tree analysis

**When to Use:**
- Bulk data transfer (>10 elements)
- Performance-critical paths
- Scene tree operations, transform updates, collision data
- Any scenario with repeated Variant conversions
