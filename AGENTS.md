# AGENTS.md

Instructions for coding agents. `CLAUDE.md` loads this file via `@AGENTS.md`.

## Working Principles

- **Aim for simplicity.** Prefer the smallest change that solves the problem. Avoid speculative abstraction, extra layers, and configuration that isn't needed yet. Match the style, naming, and comment density of the surrounding code rather than introducing new patterns.
- **Run a deslop pass after work.** Once a change is functionally complete, review the diff and strip the slop: redundant or obvious comments (especially session-narrative like "now that…"/"previously…"), dead code, unused imports, leftover scaffolding, over-engineered helpers, and verbose phrasing. The repo's `deslop` skill (`.claude/skills/deslop`) does exactly this -- run it (or do the equivalent by hand) before considering work done.

## What This Is

`godot-bevy` is a Rust library that runs Bevy's ECS inside Godot 4 -- Godot for scenes and authoring, Bevy for game logic.

## Commands

Everything runs inside `devenv shell -- <cmd>` (direnv usually activates the environment for you). The devenv scripts:

| Script | What it does |
|--------|--------------|
| `ci-lint` | what CI enforces: `cargo fmt --check` + `clippy -D warnings` |
| `itest` | integration tests, natively (needs local Godot) |
| `bench` | benchmarks, natively (needs local Godot) |
| `ci-test` / `ci-itest` / `ci-benches` | replay the CI workflows via act -- needs Docker, slow |
| `book` / `book-serve` | build / live-serve the mdbook in `book/` |

`cargo fmt` is enforced by a pre-commit hook; clippy runs in CI (`ci.yml`: fmt, clippy, unit tests on three OSes, integration tests). Example builds and Godot exports run in `examples.yml`.

Never use `--all-features`: the `api-4-2`..`api-4-5` features are mutually exclusive gdext API-level selectors, and `experimental-wasm` conflicts with `experimental-threads`.

## Workspace Map

| Path | What lives there |
|------|------------------|
| `godot-bevy/` | the library -- `src/interop/` (`GodotNodeHandle`, `GodotResourceHandle`, generated markers/signal names), `src/plugins/` (transforms, scene_tree, audio, input, signals, event_bridge, …) |
| `godot-bevy-macros/` | the derives: `#[bevy_app]`, `#[derive(GodotNode)]`, `#[derive(BevyComponents)]`, `NodeTreeView` |
| `godot-bevy-test/` + `-macros/` | published test harness: `TestApp`, `#[itest]`, `#[bench]` |
| `itest/` | integration tests + benchmarks (`rust/src/*_tests.rs`, `benchmarks.rs`) |
| `godot_bevy_codegen/` | Python generator for per-Godot-version files (see Codegen) |
| `godot_extension_api/` | dumped Godot API JSONs (4.2–4.6) that feed codegen |
| `addons/godot-bevy/` | GDScript runtime addon (watchers, bulk ops), symlinked into `itest/godot` |
| `examples/` | example games, each `rust/` crate + `godot/` project; the `BevyAppSingleton` autoload is the ECS entry point |
| `book/` | mdbook docs, published per-version by `book.yml` |

Versions: workspace 0.11.0, Bevy 0.19 (the lib depends on `bevy_*` sub-crates; examples and itest use the umbrella crate), gdext (`godot`) 0.5.

## Architecture

**BevyApp** (`godot-bevy/src/app.rs`) is the Godot node that hosts the Bevy `App` and drives Bevy's standard `Main` schedule across Godot's two frame callbacks:

- the prefix (`First`, `PreUpdate`, `StateTransition`) and `FixedMain` run during `_physics_process()` (fixed physics clock, default 60Hz)
- the suffix (`Update`, `PostUpdate`, `Last`) runs during `_process()` at display framerate

There is no `PhysicsUpdate` schedule -- fixed-rate logic goes in `FixedUpdate`.

**Plugins are opt-in.** `GodotPlugin` (added by `#[bevy_app]`) includes only `GodotCorePlugins` = scene tree + base setup. Assets are NOT in core. Add the plugins you need, or `GodotDefaultPlugins` for everything: `GodotAssetsPlugin`, `GodotTransformSyncPlugin`, `GodotAudioPlugin`, `GodotCollisionsPlugin`, `BevyInputBridgePlugin` (auto-includes `GodotInputEventPlugin`), `GodotPackedScenePlugin`, `GodotDebuggerPlugin`. `GodotSignalsPlugin::<T>` is registered per event type. The authoritative list is `godot-bevy/src/plugins/mod.rs`.

Where things live:

- transform sync: `src/plugins/transforms/` -- reads Godot in `FixedFirst`, writes back in `FixedLast`, shadow-based change filtering
- scene-tree entity lifecycle (spawning, reparenting, autosync): `src/plugins/scene_tree/`
- `#[derive(GodotNode)]` (component-first, generates the Godot class) and `#[derive(BevyComponents)]` (Godot-first, annotates your own `GodotClass`) share the `#[gdbevy(...)]` attribute grammar; component-first field exports need the explicit `export` directive. Macros in `godot-bevy-macros/`, registration in `src/plugins/scene_tree/autosync.rs`.

## Events

- Happens once or rarely (signals, collisions, game events) → `Event` + observers (`On<T>`). Connect Godot signals with `GodotSignalsPlugin::<T>` and `GodotSignals<T>`.
- Happens every frame or in batches (input, scene-tree updates) → `Message` + `MessageReader<T>`.
- Firing typed Bevy events from GDScript or non-ECS Rust: the event bridge (`src/plugins/event_bridge.rs`) -- register with `app.add_godot_event::<T>()`, send via `GodotEventSender`; events drain in `First` inside `EventBridgeSet::Drain` and reach `On<T>` observers. Mirrors `signals.rs`.

## Testing

- Unit tests: `cargo test`.
- Integration tests run in a real Godot: `devenv shell -- itest` (or `cd itest && ./run-tests.sh`). They build with `--features test-frame-signal,autosync-tests`, need Godot (`GODOT4_BIN` or on PATH), and run headless with `--fixed-fps 60` so physics steps exactly once per render frame.
- Tests live in `itest/rust/src/*_tests.rs` using `TestApp` + `#[itest]`/`#[itest(async)]`; add new files as `mod` in `itest/rust/src/lib.rs`. Read `itest/README.md` before writing one.
- There is no name filter -- to run a single test, tag it `#[itest(focus)]` and rerun; `#[itest(skip)]` skips one.
- In async tests, `app.physics_update().await` guarantees a physics tick. Don't write exact single-frame assertions -- frame boundaries have ±1-frame slop.

## Benchmarks

```bash
cd itest
./run-benches.sh                      # build + run
./run-benches.sh --filter transform_  # subset while iterating
./compare-benches.sh                  # working tree vs main -- the only valid comparison
```

Never compare two standalone runs -- process-to-process noise often exceeds 10% at µs scale. `compare-benches.sh` interleaves runs against a base branch and reports each benchmark's own noise; only trust deltas above the `Noise` column. CI runs the same comparison on every PR.

Benchmark real godot-bevy systems, not raw FFI (that just measures gdext). Run the schedules the plugin actually registers into -- transform sync is `FixedFirst`/`FixedLast` -- and give hand-built `App`s `app.insert_non_send_resource(GodotMainThread)`. Full recipe and suite: `itest/BENCHMARKING.md`. Size-suffixed variants (`_5000`) exist to catch super-linear scaling -- compare against the default size.

## Codegen

Per-Godot-version files are generated by `uv run python -m godot_bevy_codegen` from `godot_extension_api/*.json`: `src/interop/node_markers/`, `src/interop/signal_names/`, `src/plugins/scene_tree/node_type_checking/`, their dispatcher files, and the GDScript watcher versions in `addons/`. Regenerate when adding a Godot API version; never edit generated files by hand.

## Examples

```bash
cargo run --manifest-path examples/{example}/rust/Cargo.toml   # builds, then launches Godot
```

Gotcha: the examples with a static `rust.gdextension` (perf-test, platformer-2d, simple-node2d-movement) list both `.debug` and `.release` dylib paths, and `godot --path` always loads the `.debug` entry -- after a `--release` build, copy the release dylib over the debug path (or edit the `.gdextension`) or you're measuring debug perf. This matters most for perf-test.

## Performance Rules

- Bulk data between Rust and GDScript (>10 elements): use PackedArrays -- one FFI call instead of N per-element Variant conversions. Reference implementation: the transform sync systems.
- Bulk GDScript helpers (`OptimizedBulkOperations`) are `#[cfg(debug_assertions)]`-gated -- in release builds, direct FFI is faster.
- Don't replace the GDScript `OptimizedSceneTreeWatcher` with pure-FFI scene tree analysis -- measured ~2x slower even in release (decision record in `itest/BENCHMARKING.md`).
