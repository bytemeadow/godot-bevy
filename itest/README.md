# godot-bevy Integration Tests

Integration tests that use **real Godot runtime** with **Bevy-style testing patterns**.

## Writing Tests

Tests use `TestApp` for frame-by-frame control, inspired by Bevy's testing patterns:

```rust
#[itest(async)]
fn test_transform_sync(ctx: &TestContext) -> TaskHandle {
    godot::task::spawn(async move {
        // Create test app (just like Bevy!)
        let mut app = TestApp::new(ctx, |app| {
            app.add_plugins(GodotTransformSyncPlugin::default());

            app.add_systems(Startup, |mut commands: Commands| {
                commands.spawn((Transform::default(),));
            });
        }).await;

        // Frame 1: Initial state
        app.update().await;

        let entity = app.single_entity_with::<Transform>();
        let x = app.with_world(|world| {
            world.get::<Transform>(entity).unwrap().translation.x
        });
        assert_eq!(x, 0.0);

        // Frame 2: Modify and verify
        app.with_world_mut(|world| {
            world.get_mut::<Transform>(entity).unwrap().translation.x = 5.0;
        });
        app.update().await;

        // Cleanup
        app.cleanup();
    })
}
```

**Key benefits:**
- ✅ **Explicit frame control** - `app.update().await` steps one frame
- ✅ **Direct ECS access** - Query/modify world anytime
- ✅ **Bevy-idiomatic** - Familiar to Bevy developers
- ✅ **Real Godot integration** - Backed by actual Godot frames

## TestApp API

### Setup
```rust
let mut app = TestApp::new(ctx, |app| {
    app.add_plugins(MyPlugin);
    // GodotBaseCorePlugin is automatically added
}).await;
```

### Frame stepping
```rust
app.update().await; // Wait for one Godot frame
```

### World access
```rust
// Read
let value = app.with_world(|world| {
    world.get::<Component>(entity).unwrap().value
});

// Write
app.with_world_mut(|world| {
    world.get_mut::<Component>(entity).unwrap().value = 42;
});

// Helpers
let entity = app.single_entity_with::<Transform>();
```

### Cleanup
```rust
// IMPORTANT: Call before freeing Godot nodes
app.cleanup();
node.queue_free();
```

## Running Tests

```bash
./itest/run-tests.sh
```

## How It Works

1. Tests are async Godot tasks (`godot::task::spawn`)
2. `app.update().await` waits for a Godot frame signal
3. During await, Godot's main loop progresses
4. Godot calls `BevyApp::process()`, which runs Bevy's `app.update()`
5. Test resumes after frame completes

This ensures we're testing **real integration**, not mocked behavior.

## Current Tests

**real_frame_tests.rs** - Frame progression (4 tests)
- Update/PhysicsUpdate schedules
- Entity persistence
- Frame pacing

**scene_tree_tests.rs** - Scene tree integration (6 tests)
- Node added creates entity
- SceneTreeEvent::NodeAdded event
- Node removal cleanup
- Node renamed event
- ProtectedNodeEntity prevents despawn
- GodotNodeHandle validity

**transform_sync_tests.rs** - Transform synchronization (4 tests)
- OneWay mode (Bevy→Godot)
- TwoWay mode (Godot→Bevy and bidirectional)
- Disabled mode

**Total: 14 tests, all passing ✅**

## Architecture: How Tests Work with Production Code

### Unified Addon Architecture

The test Godot project **symlinks the entire addon directory**, ensuring tests use the exact same code as production:

```bash
itest/godot/addons/godot-bevy -> ../../../addons/godot-bevy
```

This means:
- ✅ All addon files are available in tests (scripts, scenes, resources)
- ✅ No code duplication or drift between test and production
- ✅ Changes to the addon are immediately reflected in tests
- ✅ Users can reference the addon in their test projects the same way

The main addon's `OptimizedSceneTreeWatcher.gd` auto-detects its environment:

**Production path** (autoload singleton):
```gdscript
/root/BevyAppSingleton/SceneTreeWatcher
```

**Test path** (sibling node):
```gdscript
get_parent().get_node("SceneTreeWatcher")
```

### Test Infrastructure for Library Users

`TestApp` provides a Bevy-style testing API for writing integration tests:

1. **Adds `GodotCorePlugins` automatically** - provides scene tree integration
2. **Waits for frames** using async/await for real Godot frame progression
3. **Provides `with_world()` / `with_world_mut()`** for ECS access
4. **Automatic cleanup** when dropped

This infrastructure is **reusable** - users of godot-bevy can write their own tests following the same patterns shown in this directory.
