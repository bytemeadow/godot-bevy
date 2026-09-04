# Integration Testing

Integration tests run game code in Godot with real frame progression. Use them when a system depends on the scene tree, Godot nodes, or Godot's frame loop. Pure Rust logic is still a good fit for unit tests.

## Project layout

Tests live in the game crate and run in its Godot project:

```text
my-game/
├── godot/
│   ├── addons/godot-bevy/
│   ├── project.godot
│   └── .godot/extension_list.cfg
└── rust/
    ├── Cargo.toml
    ├── run_godot.rs
    └── src/
        ├── lib.rs
        └── itests.rs
```

Install or symlink the addon into `godot/addons/godot-bevy`, and configure its `BevyAppSingleton` autoload. The generated GDExtension must be imported once so Godot records it in `.godot/extension_list.cfg`.

## Setup

Add the optional test dependency and enable the frame signal used by the harness:

```toml
godot-bevy-test = { version = "0.11", optional = true }
```

```toml
itest = ["dep:godot-bevy-test", "godot-bevy-test/test-frame-signal"]
```

Register the runner in the game library. Keep `#[bevy_app] fn build_app` as the only GDExtension entry point.

```rust
{{#include ../../../examples/platformer-2d/rust/src/lib.rs:itest}}
```

Set up `run_godot.rs` as shown in [Cargo Run Godot](../getting-started/gdenv.md). Its `itest` path starts the runner scene and sets `GODOT_BEVY_ITEST=1` for the child Godot process.

`#[bevy_app]` leaves `build_app` as a normal function, so `TestApp::new(&ctx, build_app)` works. Most tests should add only the plugins they cover.

## Writing tests

Use `#[itest]` on an async function with an owned `TestContext`. `TestApp` initializes the autoload, waits for the initial scene-tree population, and gives the test explicit frame control.

```rust
{{#include ../../../examples/platformer-2d/rust/src/itests.rs:gem_collected}}
```

Use `with_world` for read-only access. Use `with_world_mut` for mutations and queries, since creating a Bevy query needs mutable world access.

```rust
{{#include ../../../examples/platformer-2d/rust/src/itests.rs:with_world_mut_query}}
```

`#[itest(async)] fn test(ctx: &TestContext) -> godot::task::TaskHandle` remains available when a test returns an explicitly spawned task. Async functions use an owned context, because a reference cannot outlive the spawned task.

## Running tests

Run the game crate's runner:

```bash
cargo run --features itest
```

After adding or changing the extension, import the project once. Then a direct invocation is:

```bash
godot --headless --path godot --import
GODOT_BEVY_ITEST=1 godot --headless --fixed-fps 60 --path godot --scene res://addons/godot-bevy/test/TestRunner.tscn --quit-after 10000
```

`ITEST_FILTER` selects comma-separated, case-sensitive name substrings. `ITEST_REPEAT` repeats selected tests. `ITEST_JSON_PATH` writes a report. `#[itest(skip)]` reports a skipped test, while `#[itest(focus)]` selects focused tests; set `ITEST_DENY_FOCUS=1` in CI to reject focus mode. The full configuration table is in the [godot-bevy-test README](https://github.com/bytemeadow/godot-bevy/tree/main/godot-bevy-test#runner-configuration).

## Troubleshooting

### BevyApp defined multiple times

The game and test dependencies resolved different copies of `godot-bevy`. Run `cargo tree -d`, then align their sources and versions. A second test crate that links the game as an rlib is unsupported because it creates duplicate GDExtension entry symbols.

### IntegrationTests class not found

Godot did not load the extension. Run `--import` once and check that `.godot/extension_list.cfg` lists the generated GDExtension.

### Must be run in headless mode

The test runner only runs headlessly. Pass `--headless` when starting Godot.

### Game code runs before the first test

Without `GODOT_BEVY_ITEST`, the game autoload boots for a frame or two before the runner. Its startup logs and any nodes it adds under root appear before `Run godot-bevy integration tests` and leak into every test's scene scan. Set `GODOT_BEVY_ITEST=1` in the process that launches Godot.

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

Build the Rust crate with `--release`, then launch the benchmark runner:

```bash
godot --headless --path godot --scene res://addons/godot-bevy/test/BenchRunner.tscn --quit-after 30000
```
