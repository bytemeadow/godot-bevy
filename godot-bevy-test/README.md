# godot-bevy-test

Integration testing framework for godot-bevy projects.

This crate provides a testing framework for writing integration tests that run inside Godot with full access to both Bevy ECS and Godot's runtime. Tests execute in headless mode with real frame progression, allowing you to verify your game logic works correctly in the actual runtime environment.

## Features

- **Real Godot Integration**: Tests run in Godot's headless mode with actual frame progression
- **Async Test Support**: Wait for frames, test across multiple update cycles
- **Bevy-style API**: Familiar `TestApp` pattern with world access
- **Benchmark Support**: Performance benchmarking with statistical analysis
- **Focus & Skip**: Easily focus on specific tests or skip work-in-progress
- **Cross-platform**: Works on Linux, macOS, and Windows

## Quick Start

### 1. Create a Test Crate

Create a separate crate for your integration tests:

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

# Your game crate (for testing your components/systems)
my-game = { path = "../my-game" }
```

### 2. Set Up the Test Entry Point

```rust
// my-game-tests/src/lib.rs
use godot::init::{ExtensionLibrary, gdextension};
use godot_bevy_test::prelude::*;

// Declare the test runner class for Godot
godot_bevy_test::declare_test_runner!();

// Include your test modules
mod player_tests;
mod combat_tests;

#[gdextension(entry_symbol = my_game_tests)]
unsafe impl ExtensionLibrary for IntegrationTests {}
```

### 3. Write Tests

```rust
// my-game-tests/src/player_tests.rs
use bevy::prelude::*;
use godot_bevy_test::prelude::*;
use my_game::Player;

#[itest(async)]
fn test_player_movement(ctx: &TestContext) -> godot::task::TaskHandle {
    godot::task::spawn(async move {
        let mut app = TestApp::new(&ctx, |app| {
            app.add_plugins(my_game::PlayerPlugin);
        }).await;

        // Spawn a player
        app.with_world_mut(|world| {
            world.spawn((Player::default(), Transform::default()));
        });

        // Run a few frames
        for _ in 0..5 {
            app.update().await;
        }

        // Verify player moved
        let pos = app.with_world(|world| {
            world.query::<&Transform>()
                .iter(world)
                .next()
                .unwrap()
                .translation
        });
        
        assert!(pos.x > 0.0, "Player should have moved");
    })
}
```

### 4. Set Up Godot Test Project

Create a minimal Godot project for running tests:

```
my-game-tests/
├── godot/
│   ├── project.godot
│   └── my-game-tests.gdextension
```

The `.gdextension` file should point to your test library:

```ini
[configuration]
entry_symbol = "my_game_tests"
compatibility_minimum = 4.3

[libraries]
linux.debug.x86_64 = "res://../target/debug/libmy_game_tests.so"
# ... other platforms
```

### 5. Run Tests

```bash
cd my-game-tests
cargo build
godot4 --headless --path godot --quit-after 5000
```

## API Reference

### Test Macros

```rust
#[itest]                    // Sync test
#[itest(async)]             // Async test (most common)
#[itest(skip)]              // Skip this test
#[itest(focus)]             // Only run focused tests
#[itest(async, skip)]       // Combine attributes
```

### TestApp

The main testing interface:

```rust
// Create with custom setup
let mut app = TestApp::new(&ctx, |app| {
    app.add_plugins(MyPlugin);
}).await;

// Step one frame
app.update().await;

// Access the world
app.with_world(|world| { /* read-only */ });
app.with_world_mut(|world| { /* read-write */ });

// Convenience methods
let transform = app.get_single::<Transform>();
let entity = app.single_entity_with::<Player>();

// Cleanup (automatic on drop)
app.cleanup();
```

### Frame Helpers

```rust
await_frame().await;        // Wait for next frame
await_frames(5).await;      // Wait for N frames
```

### bevy_app_test! Macro

For quick test setup with a counter:

```rust
#[itest(async)]
fn test_systems_run(ctx: &TestContext) -> godot::task::TaskHandle {
    bevy_app_test!(ctx, counter, |app| {
        app.add_systems(Update, move |c: Res<MyCounter>| {
            counter.increment();
        });
    }, async {
        await_frames(5).await;
        assert!(counter.get() >= 4);
    })
}
```

### Benchmarks

```rust
#[bench]
fn my_benchmark() -> i32 {
    // Code to benchmark - must return a value
    expensive_operation();
    42
}

#[bench(repeat = 50)]       // Custom iteration count
fn expensive_benchmark() -> i32 {
    very_expensive_operation();
    42
}
```

## Custom Test Runner Name

If you need a custom class name:

```rust
godot_bevy_test::declare_test_runner!(MyCustomTestRunner);

#[gdextension(entry_symbol = my_tests)]
unsafe impl ExtensionLibrary for MyCustomTestRunner {}
```

Then update `addons/godot-bevy/test/TestRunner.gd`:

```gdscript
@export var test_class_name: String = "MyCustomTestRunner"
```

## License

MIT OR Apache-2.0
