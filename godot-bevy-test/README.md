# godot-bevy-test

`godot-bevy-test` runs integration tests inside Godot with real frame progression.

## Setup

Keep tests in the game crate and gate them behind an `itest` feature. The game keeps its existing `cdylib` and `#[bevy_app]` entry point.

```toml
[dependencies]
bevy = { version = "0.19", default-features = false }
godot = "0.5"
godot-bevy = "0.11"
godot-bevy-test = { version = "0.11", optional = true }

[features]
itest = ["dep:godot-bevy-test", "godot-bevy-test/test-frame-signal"]
```

Register the runner in the same library as the game extension:

```rust
#[cfg(feature = "itest")]
mod itests;

#[cfg(feature = "itest")]
godot_bevy_test::declare_test_runner!();
```

Use the game’s Godot project. It needs the `godot-bevy` addon and the `BevyAppSingleton` autoload. Launch `res://addons/godot-bevy/test/TestRunner.tscn` with `--scene`; do not copy the runner scene into another project. Set `GODOT_BEVY_ITEST=1` before starting Godot so the autoload does not boot the game before the runner initializes it.

## Writing tests

The usual form is an async function with an owned `TestContext`:

```rust
use bevy::prelude::*;
use godot_bevy_test::prelude::*;

#[itest]
async fn player_spawns(ctx: TestContext) {
    let mut app = TestApp::new(&ctx, |app| {
        app.add_plugins(PlayerPlugin);
    })
    .await;

    app.with_world_mut(|world| {
        let mut players = world.query::<&Player>();
        assert_eq!(players.iter(world).count(), 0);
    });

    app.cleanup().await;
}
```

Use `with_world` for read-only access. Queries require `with_world_mut`, because constructing a Bevy query mutates world-local query state.

`#[itest(async)] fn test(ctx: &TestContext) -> godot::task::TaskHandle` is still supported when a test must return an explicitly spawned task. Async functions cannot take `&TestContext` because that reference cannot outlive the wrapper.

`#[itest(skip)]` leaves a test in the report without running it. `#[itest(focus)]` runs only focused tests. `ITEST_DENY_FOCUS=1` rejects focused registrations in CI.

## Running tests

Run the game crate’s runner:

```bash
cargo run --features itest
```

For a direct launch, import once after adding or changing the extension, then run:

```bash
godot --headless --path godot --import
GODOT_BEVY_ITEST=1 godot --headless --fixed-fps 60 --path godot --scene res://addons/godot-bevy/test/TestRunner.tscn --quit-after 10000
```

## Runner configuration

| Variable | Meaning |
| --- | --- |
| `ITEST_FILTER` | Comma-separated, case-sensitive name substrings |
| `ITEST_REPEAT` | Positive attempt count per selected test; default `1` |
| `ITEST_TIMEOUT_FRAMES` | Positive frame limit per attempt; default `600` |
| `ITEST_JSON_PATH` | Optional path for the v1 JSON report |
| `ITEST_DENY_FOCUS` | `1`/`true` rejects focused runs; default false |
| `ITEST_BUILD_PROFILE` | `debug` or `release` report metadata |

Filters are trimmed and empty terms are discarded. Empty selections are configuration errors. Skip affects execution after selection, so selected skipped tests remain visible in the report.

## TestApp

`TestApp::new(&ctx, build_app)` can boot the full game because `#[bevy_app]` leaves `build_app` as a normal function. Prefer adding the specific plugins a test needs. `update().await` advances one frame, and `physics_update().await` guarantees a physics tick. Call `cleanup().await` before freeing Godot nodes used by the test.

## Benchmarks

`#[bench]` runs a function repeatedly and requires a return value so its work is not optimized away:

```rust
#[bench]
fn name() -> i32 {
    42
}

#[bench(repeat = 50)]
fn repeated_name() -> i32 {
    42
}
```

Build the Rust crate with `--release`, then launch:

```bash
godot --headless --path godot --scene res://addons/godot-bevy/test/BenchRunner.tscn --quit-after 30000
```

## Troubleshooting

`BevyApp` defined multiple times means the game and test dependencies resolved different copies of `godot-bevy`. Run `cargo tree -d` and align their sources and versions. `IntegrationTests class not found` usually means Godot has not imported the extension or `.godot/extension_list.cfg` does not contain it; run the one-time `--import` command. A separate test crate that links the game as an rlib is unsupported because it creates duplicate GDExtension entry symbols.

### Game code runs before the first test

Without `GODOT_BEVY_ITEST`, the game autoload boots for a frame or two before the runner. Its startup logs and any nodes it adds under root appear before `Run godot-bevy integration tests` and leak into every test's scene scan. Set `GODOT_BEVY_ITEST=1` in the process that launches Godot.

## License

MIT OR Apache-2.0
