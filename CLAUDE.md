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
- `GodotInputWatcher` - Bridges Godot input events to Bevy

### Plugin Architecture

**Opt-in Plugin System**: Following Bevy's philosophy, godot-bevy now provides granular plugin control. By default, only minimal core functionality is included.

- **`GodotPlugin`**: Now minimal by default - only includes `GodotCorePlugins` (scene tree, assets, basic setup)
- **`GodotCorePlugins`**: Minimal required functionality 
- **`GodotDefaultPlugins`**: All functionality enabled (use for easy migration)
- **Individual plugins**: 
  - `GodotTransformSyncPlugin` (move/position nodes from Bevy)
  - `GodotAudioPlugin` (play sounds/music from Bevy) 
  - `GodotTypedSignalsPlugin::<T>` (respond to Godot signals in Bevy)
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
    app.add_plugins(GodotTransformSyncPlugin::default()) // Transform sync
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

**Signal Integration**: Godot signals become typed Bevy messages via `GodotTypedSignalsPlugin::<T>` and `MessageReader<T>`,
enabling ECS systems to respond to UI interactions and game events.

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

### Debug vs Release FFI Performance

**Key Finding**: In **release builds**, individual Rust FFI calls are faster than bulk GDScript operations. In **debug builds**, the opposite is true due to unoptimized Rust FFI overhead.

**Implications:**
- Bulk GDScript helpers (`OptimizedBulkOperations`) are gated with `#[cfg(debug_assertions)]`
- Release builds use direct FFI calls for transforms and input checking
- Debug builds use GDScript bulk operations for faster iteration

**Example from transform sync:**
```rust
// In release: direct FFI is ~20% faster
#[cfg(not(debug_assertions))]
{
    pre_update_godot_transforms_individual(entities, &mut godot);
}

// In debug: GDScript bulk path is faster
#[cfg(debug_assertions)]
{
    if let Some(bulk_ops) = get_bulk_operations_node(&mut godot) {
        pre_update_godot_transforms_bulk(entities, bulk_ops);
        return;
    }
    pre_update_godot_transforms_individual(entities, &mut godot);
}
```

### OptimizedSceneTreeWatcher

The `OptimizedSceneTreeWatcher` (GDScript) pre-analyzes node metadata to avoid expensive FFI calls from Rust. This optimization is valuable in **both** debug and release builds.

**What it pre-analyzes:**
- `node_type` - Uses GDScript `is` checks (fast) instead of Rust `try_from_instance_id` (up to 199 FFI calls per node)
- `node_name` - Avoids `get_name()` FFI call
- `parent_id` - Avoids `get_parent().instance_id()` FFI calls  
- `collision_mask` - Avoids 4 `has_signal()` FFI calls

**Performance impact**: ~2x faster for scene tree entity creation compared to fallback FFI path.

**Benchmark validation (500 nodes, release build):**
| Approach | Time |
|----------|------|
| GDScript Full (all metadata) | ~203µs |
| GDScript Type + FFI (hybrid) | ~202µs |
| Pure FFI (metadata only) | ~381µs |

The current "GDScript Full" approach is optimal. Hybrid (type-only from GDScript) offers <1% improvement, not worth the added complexity. Pure FFI is ~2x slower, confirming bulk GDScript is worthwhile even in release builds for scene tree analysis.

## Benchmarking

### Running Benchmarks
```bash
cd itest
./run-benches.sh           # Build and run benchmarks
./run-benches.sh --skip-build  # Run without rebuilding (for CI)
```

### Benchmark Philosophy

Benchmarks should test **actual godot-bevy systems**, not raw FFI performance. Testing raw FFI just measures gdext, not our code.

**Good benchmark**: Runs the real `GodotTransformSyncPlugin` systems via `app.world_mut().run_schedule()`

**Bad benchmark**: Manually calls `node.set_position()` in a loop (just tests gdext FFI)

### Current Benchmarks

Located in `itest/rust/src/benchmarks.rs`:

| Benchmark | What it tests |
|-----------|---------------|
| `transform_sync_bevy_to_godot_3d` | Real Bevy→Godot sync system (1000 3D nodes) |
| `transform_sync_godot_to_bevy_3d` | Real Godot→Bevy sync system (1000 3D nodes) |
| `transform_sync_bevy_to_godot_2d` | Real Bevy→Godot sync system (1000 2D nodes) |
| `transform_sync_godot_to_bevy_2d` | Real Godot→Bevy sync system (1000 2D nodes) |
| `transform_sync_roundtrip_3d` | Full frame: PreUpdate → game logic → Last |
| `transform_sync_roundtrip_2d` | Full frame: PreUpdate → game logic → Last |

### Adding New Benchmarks

When adding benchmarks:
1. Create a real Bevy `App` with the plugin being tested
2. Initialize required schedules: `app.init_schedule(PreUpdate)`, `app.init_schedule(Last)`
3. Run actual schedules: `app.world_mut().run_schedule(Last)`
4. Don't duplicate system logic - run the real systems so benchmarks stay accurate when code changes
