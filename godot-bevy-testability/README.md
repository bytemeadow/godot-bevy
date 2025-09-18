# Godot-Bevy Testability

**Testing framework for Bevy ECS systems with embedded Godot runtime**

This crate provides specialized testing utilities for projects using [godot-bevy](../godot-bevy), enabling you to test Bevy systems that interact with Godot nodes and resources using an embedded Godot runtime.

## Overview

Built on top of [`godot-testability-runtime`](https://crates.io/crates/godot-testability-runtime) v0.1.1+, this crate provides:

- **Scene Tree Integration Testing** - Test entity creation and synchronization with Godot's scene tree
- **Transform Sync Testing** - Validate transform synchronization between Godot nodes and Bevy entities
- **Plugin Testing** - Test godot-bevy plugins with full scene tree integration
- **Class Registration** - Support for custom Godot classes in tests (BevyApp, SceneTreeWatcher, etc.)

## Quick Start

Add to your `Cargo.toml`:

```toml
[dev-dependencies]
godot-bevy-testability = "0.1.0"

[[test]]
name = "my_integration_tests"
harness = false  # Required for custom test runner
```

Write integration tests (`tests/my_integration_tests.rs`):

```rust
use godot_bevy_testability::*;
use bevy::prelude::*;
use godot::prelude::*;

fn test_scene_tree_integration(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    // Set up full scene tree integration
    let mut env = ctx.setup_full_integration();

    // Add your plugin
    ctx.app.add_plugins(MyPlugin);

    // Create and add a Godot node to the scene tree
    let mut node = Node3D::new_alloc();
    node.set_name("TestNode");
    node.set_position(Vector3::new(10.0, 20.0, 30.0));

    // Add to scene tree - this triggers scene tree events
    env.add_node_to_scene(node.clone());

    // Update the app to process events
    ctx.app.update();

    // Query the Bevy world for the created entity
    let world = ctx.app.world_mut();
    let query = world.query_filtered::<&Transform, With<Node3DMarker>>();

    // Verify the entity was created with correct transform
    assert_eq!(query.iter(&world).count(), 1);

    Ok(())
}

// Use the test macro to create the test harness
bevy_godot_test_main! {
    test_scene_tree_integration,
    // Add more test functions here
}
```

## Key Features

### BevyGodotTestContext

The main test context provides access to both Bevy and Godot:

```rust
pub struct BevyGodotTestContext {
    pub app: App,                        // Bevy App instance
    pub scene_tree_ptr: *mut c_void,     // Godot SceneTree pointer
}
```

### Test Environment Setup

The `setup_full_integration()` helper sets up a complete test environment:

```rust
fn test_with_watchers(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    // Sets up SceneTreeWatcher and CollisionWatcher automatically
    let env = ctx.setup_full_integration();

    // Access the scene tree and watchers
    env.scene_tree         // Gd<SceneTree>
    env.scene_tree_watcher // Gd<SceneTreeWatcher>
    env.collision_watcher  // Gd<CollisionWatcher>

    // Manually send events if needed
    env.send_scene_tree_event(SceneTreeEvent { /* ... */ });

    Ok(())
}
```

### Transform Sync Testing

Test transform synchronization between Godot and Bevy:

```rust
use godot_bevy::plugins::transforms::{GodotTransformSyncPlugin, TransformSyncMode};

fn test_transform_sync(ctx: &mut BevyGodotTestContext) -> TestResult<()> {
    let env = ctx.setup_full_integration();

    // Add transform sync plugin with desired mode
    ctx.app.add_plugins(GodotTransformSyncPlugin {
        sync_mode: TransformSyncMode::BevyToGodot,
    });

    // Create a node with position
    let mut node = Node3D::new_alloc();
    node.set_position(Vector3::new(10.0, 20.0, 30.0));
    env.add_node_to_scene(node.clone());

    ctx.app.update();

    // Query for the synced entity
    let world = ctx.app.world_mut();
    let query = world.query::<&Transform>();
    for transform in query.iter(&world) {
        assert_eq!(transform.translation, Vec3::new(10.0, 20.0, 30.0));
    }

    Ok(())
}
```

## Architecture

```
┌─────────────────────────────────────────┐
│         Your Test Code                  │
├─────────────────────────────────────────┤
│      godot-bevy-testability             │
│   • BevyGodotTestContext                │
│   • Test environment helpers            │
│   • bevy_godot_test_main! macro        │
├─────────────────────────────────────────┤
│      godot-testability-runtime 0.1.1+   │
│   • Embedded Godot runtime              │
│   • Class registration support          │
│   • Test harness infrastructure        │
├─────────────────────────────────────────┤
│      libgodot.dylib (4.3.1)            │
│   • Embedded Godot engine              │
│   • Scene tree and node system         │
└─────────────────────────────────────────┘
```

## Platform Support

**Important**: Integration tests that use the embedded Godot runtime currently only work on **macOS**. This is because godot-testability-runtime downloads libgodot.dylib from SwiftGodotKit releases.

- ✅ **macOS**: Full support for all integration tests
- ❌ **Linux**: Integration tests not supported (unit tests work fine)
- ❌ **Windows**: Integration tests not supported (unit tests work fine)

## API Version Compatibility

**Important**: The embedded Godot runtime is version 4.3.1, so integration tests must use the `api-4-3` feature:

```toml
# In your test files or CI (macOS only)
cargo test --features api-4-3 --test my_integration_tests
```

For unit tests and builds that don't use the embedded runtime, you can use any supported API version (4.1 through 4.4) on all platforms.

## Examples

Check out the test files in the godot-bevy repository:

```bash
# Run transform sync tests
cargo test --features api-4-3 --test full_integration_transform_tests

# Run basic integration tests
cargo test --features api-4-3 --test bevy_godot_integration_tests
```

## Writing Tests

### Test Structure

1. Create a test file with `harness = false` in Cargo.toml
2. Define test functions that take `&mut BevyGodotTestContext`
3. Use the `bevy_godot_test_main!` macro to generate the test harness

### Available Helpers

- `ctx.setup_full_integration()` - Set up scene tree with watchers
- `ctx.initialize_godot_bevy_resources()` - Initialize basic resources
- `env.add_node_to_scene()` - Add nodes to the scene tree
- `env.send_scene_tree_event()` - Manually trigger events

### Testing Tips

1. **Always use `api-4-3` feature** for integration tests
2. **Call `ctx.app.update()`** after scene changes to process events
3. **Use `setup_full_integration()`** for tests needing scene tree integration
4. **Check transform sync modes** when testing position/rotation/scale

## Troubleshooting

### "Cannot get class" Errors

The test environment supports custom Godot classes (BevyApp, SceneTreeWatcher, etc.) through class registration in godot-testability-runtime 0.1.1+.

### Version Mismatch Errors

If you see "gdext was compiled against newer Godot version", ensure you're using the `api-4-3` feature flag for integration tests.

### Scene Tree Path Errors

The scene tree plugin looks for watchers at both `/root/BevyAppSingleton/...` and `BevyAppSingleton/...` to support both production and test environments.

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE))
- MIT License ([LICENSE-MIT](../LICENSE-MIT))

at your option.