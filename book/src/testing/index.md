# Integration Testing

Testing game logic can be tricky. Unit tests work great for pure Rust functions, but godot-bevy code interacts deeply with both Godot's runtime and Bevy's ECS. That's where integration testing comes in.

The `godot-bevy-test` crate provides a framework for writing tests that run inside Godot with real frame progression. Your tests have full access to both Bevy's ECS and Godot's scene tree, letting you verify that your game logic actually works in the runtime environment.

## Why Integration Tests?

Consider testing a player movement system:

```rust
fn player_movement(
    time: Res<Time>,
    mut query: Query<(&mut Transform, &GodotNodeHandle), With<Player>>,
) {
    for (mut transform, handle) in query.iter_mut() {
        transform.translation.x += 100.0 * time.delta_secs();
    }
}
```

A unit test can't verify this works correctly because:
- `Time` comes from Bevy's runtime
- `GodotNodeHandle` requires a real Godot node
- Transform sync needs Godot's scene tree
- Frame timing depends on Godot's main loop

Integration tests solve this by running your code in the actual Godot environment.

## Setting Up Integration Tests

### 1. Create a Test Crate

Integration tests live in a separate crate that compiles to a GDExtension library:

```toml
# my-game-tests/Cargo.toml
[package]
name = "my-game-tests"
version = "0.1.0"
edition = "2024"

[lib]
crate-type = ["cdylib"]

[dependencies]
godot = "0.4"
godot-bevy = "0.9"
godot-bevy-test = "0.9"
bevy = { version = "0.17", default-features = false }

# Import your game crate to test its systems
my-game = { path = "../my-game" }
```

### 2. Create the Test Entry Point

```rust
// my-game-tests/src/lib.rs
use godot::init::{ExtensionLibrary, gdextension};
use godot_bevy_test::prelude::*;

// This macro creates the Godot class that runs tests
godot_bevy_test::declare_test_runner!();

// Include test modules
mod movement_tests;
mod combat_tests;

#[gdextension(entry_symbol = my_game_tests)]
unsafe impl ExtensionLibrary for IntegrationTests {}
```

### 3. Set Up the Godot Project

Create a minimal Godot project to run the tests:

```
my-game-tests/
├── rust/
│   └── ... (your test crate)
└── godot/
    ├── project.godot
    └── my-game-tests.gdextension
```

Your `.gdextension` file should point to your test library:

```ini
[configuration]
entry_symbol = "my_game_tests"
compatibility_minimum = 4.3

[libraries]
linux.debug.x86_64 = "res://../target/debug/libmy_game_tests.so"
macos.debug = "res://../target/debug/libmy_game_tests.dylib"
windows.debug.x86_64 = "res://../target/debug/my_game_tests.dll"
```

## Writing Tests

### Basic Async Test

Most tests are async because they need to wait for Godot frames:

```rust
use godot_bevy_test::prelude::*;
use bevy::prelude::*;

#[itest(async)]
fn test_entity_spawns(ctx: &TestContext) -> godot::task::TaskHandle {
    godot::task::spawn(async move {
        // Create a test app with your plugins
        let mut app = TestApp::new(&ctx, |app| {
            app.add_plugins(MyGamePlugin);
        }).await;

        // Run one frame to initialize
        app.update().await;

        // Spawn an entity
        app.with_world_mut(|world| {
            world.spawn((Player::default(), Transform::default()));
        });

        // Run another frame
        app.update().await;

        // Verify the entity exists
        let count = app.with_world(|world| {
            world.query::<&Player>().iter(world).count()
        });
        
        assert_eq!(count, 1, "Player should exist");
    })
}
```

### Using bevy_app_test! for Quick Tests

For simpler tests, the `bevy_app_test!` macro reduces boilerplate:

```rust
#[itest(async)]
fn test_system_runs_each_frame(ctx: &TestContext) -> godot::task::TaskHandle {
    bevy_app_test!(ctx, counter, |app| {
        #[derive(Resource)]
        struct FrameCount(Counter);
        
        app.insert_resource(FrameCount(counter.clone()));
        app.add_systems(Update, |count: Res<FrameCount>| {
            count.0.increment();
        });
    }, async {
        await_frames(5).await;
        assert!(counter.get() >= 4, "System should run each frame");
    })
}
```

### Skip and Focus

During development, you can skip or focus tests:

```rust
#[itest(skip)]              // Skip this test
fn test_not_ready_yet(ctx: &TestContext) {
    // Work in progress
}

#[itest(focus)]             // Only run focused tests
fn test_debugging_this(ctx: &TestContext) {
    // When any test has focus, only focused tests run
}

#[itest(async, skip)]       // Combine attributes
fn test_flaky(ctx: &TestContext) -> godot::task::TaskHandle {
    // ...
}
```

## Running Tests

Build and run tests in headless mode:

```bash
cd my-game-tests

# Build the test library
cargo build

# Run tests (adjust path to your Godot binary)
godot4 --headless --path godot --quit-after 5000
```

You'll see output like:

```
Run godot-bevy async integration tests...
  Found 5 async tests in 2 files.
  test_entity_spawns ... ok
  test_transform_syncs ... ok
  test_system_runs_each_frame ... ok
  test_not_ready_yet ... [SKIP]
  test_player_movement ... ok

Test result:
  4 passed, 1 skipped in 0.42s
All tests passed!
```

### Test Script

Create a shell script for convenience:

```bash
#!/bin/bash
# run-tests.sh
set -e

cd "$(dirname "$0")"
cargo build

# Cross-platform temp file for exit code
EXIT_FILE="${TMPDIR:-/tmp}/godot_test_exit_code"
rm -f "$EXIT_FILE"
export GODOT_TEST_EXIT_CODE_PATH="$EXIT_FILE"

godot4 --headless --path godot --quit-after 5000

if [ -f "$EXIT_FILE" ]; then
    exit $(cat "$EXIT_FILE")
else
    exit 1
fi
```

## Benchmarks

The framework also supports benchmarks:

```rust
use godot_bevy_test::bench;

#[bench]
fn benchmark_spawning() -> i32 {
    // Code to benchmark - must return a value to prevent optimization
    let mut count = 0;
    for _ in 0..1000 {
        count += 1;
    }
    count
}

#[bench(repeat = 50)]       // Custom iteration count
fn benchmark_expensive_op() -> i32 {
    expensive_operation();
    42
}
```

Run benchmarks with a separate runner:

```bash
godot4 --headless --path godot -s addons/godot-bevy/test/BenchRunner.tscn --quit-after 30000
```

## Best Practices

### 1. Test Real Behavior

Don't mock Godot - use real nodes and scene tree:

```rust
// Good: Real Godot node
let mut node = Node2D::new_alloc();
ctx.scene_tree.clone().add_child(&node);

// Bad: Trying to fake it
// let fake_handle = GodotNodeHandle::fake(); // Don't do this
```

### 2. Clean Up After Tests

Always clean up nodes you create:

```rust
#[itest(async)]
fn test_with_cleanup(ctx: &TestContext) -> godot::task::TaskHandle {
    godot::task::spawn(async move {
        let mut node = Node2D::new_alloc();
        ctx.scene_tree.clone().add_child(&node);

        let mut app = TestApp::new(&ctx, |_| {}).await;
        
        // ... test code ...

        // Clean up: BevyApp first, then nodes
        app.cleanup();
        node.queue_free();
        await_frame().await;
    })
}
```

### 3. Wait for Frame Processing

Operations often need a frame to complete:

```rust
// Spawn entity
app.with_world_mut(|world| { world.spawn(MyComponent); });

// Wait for systems to process
app.update().await;

// Now query the result
```

### 4. Use TestApp::cleanup() Before Freeing Nodes

If your test creates Godot nodes that are tracked by Bevy, clean up the BevyApp first:

```rust
// Wrong order: may crash
node.queue_free();
app.cleanup();  // Transform sync might access freed node!

// Correct order
app.cleanup();  // Stop Bevy systems first
node.queue_free();
```
