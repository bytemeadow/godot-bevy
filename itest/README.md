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
./itest/run-tests.sh --filter test_exactly_one_clear
./itest/run-tests.sh --filter transform_,scene_tree_ --repeat 3
./itest/run-tests.sh --timeout-frames 600 --json target/itest.json
```

Filters are comma-separated, case-sensitive substrings. Empty filters and filters
that select no tests exit 2 instead of producing a false-green run. Focused tests
intersect with the filter; set `ITEST_DENY_FOCUS=1` to reject any focused registry
before execution.

`--repeat` runs every selected test in test-major order and reports a mix of passing and
failing attempts as `flaky`. `--timeout-frames` defaults to 600 per attempt. Failed,
flaky, and timed-out tests exit 1; configuration and harness failures exit 2.

`--json` writes the versioned report atomically and checkpoints it after each
logical test. The same final JSON appears between `===ITEST_JSON_START===` and
`===ITEST_JSON_END===` in stdout. A checkpoint left by a watchdog has
`"complete": false`.

The harness contracts can be exercised independently:

```bash
./itest/verify-harness.sh repeat|panic|config|focus
```

Each verification tier ships the same kind of self-check, proving the tooling
itself fails closed rather than reporting false green:

| Script | Modes |
|--------|-------|
| `verify-harness.sh` | `repeat`, `panic`, `config`, `focus` |
| `verify-profiling.sh` | `schemas`, `tools`, `contract`, `tracy-live`, `fail-closed`, `compare`, `compare-live`, `native-live`, `workflow` |
| `verify-qualification.sh` | `contract`, `mutants`, `doctests`, `assertions`, `faults`, `workflow` |
| `verify-coverage.sh` | `contract`, `tools`, `flush`, `pipeline`, `reports`, `diff`, `godot-live`, `fail-closed-live`, `workflow`, `all-offline` |

Modes ending in `-live` need Godot and a full build; the rest are offline and
run in ordinary CI.

## Profiling

Tier-2 profiles are optimized, symbolized diagnostics and are not benchmark
results. Tracy profiles exact benchmark spans; native profiles use Samply over
the whole Godot process, including startup and teardown. Samply records CPU
samples, not allocation counts or bytes.

```bash
./itest/run-profile.sh --bench transform_sync_bevy_to_godot_3d
./itest/run-profile.sh --native --bench transform_sync_bevy_to_godot_3d --seconds 5
./itest/compare-profiles.py path/to/a/spans.json path/to/b/spans.json
PROFILE_ROUNDS=3 ./itest/compare-profiles.sh main --bench transform_sync_bevy_to_godot_3d
PROFILE_ROUNDS=3 ./itest/compare-profiles.sh --self --bench transform_sync_bevy_to_godot_3d
```

The Python comparison is descriptive. The shell comparison alternates at least
three process captures per side and reports standard-error-based noise. Artifacts
are written under `target/profiles/`.

## Coverage

Tier-4 merges unit-test, proc-macro construction, and real-Godot Rust coverage.
It is reach evidence, not assertion quality, branch proof, GDScript coverage, or
benchmark evidence.

```bash
devenv shell -- coverage
devenv shell -- coverage diff --base main
devenv shell -- coverage clean
```

The scope and exclusions live in `coverage/scope-v1.toml`. Diff mode runs three
separate Godot processes and requires every compiler region touching a changed
line to be reached by unit/build evidence or by all three runs. There are no
percentage gates. Reports live under `target/coverage/runs/`; successful runs
prune raw profiles and merged profdata unless `--keep-raw` is supplied, while
failed runs retain them. `coverage clean` removes only the coverage build and run
trees. Coverage forces sccache recache so proc-macro construction cannot be
replaced by a compiler-cache hit.

The manual Linux workflow has a 90-minute cold-run budget; actual phase timings
and disk use are recorded in `coverage-v1.json`. Use the rustc-sysroot LLVM tools,
the Cargo JSON object paths, and the test-process sentinel. Import profiles are
diagnostic only, absent source mappings are not zero coverage, and instrumented
timings must never be compared with Tier-2 or benchmark results.

## How It Works

1. Sync and async tests are collected into one globally ordered run
2. `app.update().await` waits for a Godot frame signal
3. During await, Godot's main loop progresses
4. Godot calls `BevyApp::process()`, which runs the Main suffix (`Update`/`PostUpdate`/`Last`) + `clear_trackers`
5. Test resumes after frame completes

This ensures we're testing **real integration**, not mocked behavior.

## Current Tests

| Module | Covers |
|--------|--------|
| `real_frame_tests.rs` | Update/FixedUpdate schedules, frame pacing |
| `scene_tree_tests.rs` | Entity creation/cleanup, renames, reparenting, NodeEntityIndex |
| `scene_tree_watcher_init_tests.rs` | Watcher initialization, no duplicate watchers |
| `transform_sync_tests.rs` | OneWay/TwoWay/disabled sync modes |
| `collision_tests.rs` | Collision state tracking, started/ended observers |
| `signal_tests.rs` | Signal connection and dispatch to observers |
| `input_tests.rs` | Keyboard/action/mouse events, Bevy ButtonInput bridge |

Run `./run-tests.sh` for the authoritative list and count.

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
