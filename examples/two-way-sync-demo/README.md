# Two-way transform sync

GDScript changes a quad's x position in `godot/quad.gd`. Bevy changes its y position in `rust/src/lib.rs`. Together they move the quad around a circle.

The app enables `TransformSyncMode::TwoWay`. Godot and Bevy can write different translation axes without overwriting each other's changes. If both write the same axis, Bevy wins. A Godot write in `_process` reaches Bevy's physics-phase read on the next frame.

See [Transform Sync Modes](https://bytemeadow.github.io/godot-bevy-book?page=transforms/sync-modes.html) for the read and write schedule.

## Running this example

From the repository root, with Godot on your `PATH`:

```bash
cargo run --manifest-path examples/two-way-sync-demo/rust/Cargo.toml
```

The launcher builds the library, generates its GDExtension descriptor, and opens Godot.

The [capture harness](../harness/README.md) provides `orbit-split` (120 frames, checkpoints 0/30/60/90/120) and `orbit-split-hidden` (a deliberate frame-0 visibility failure). From the repository root, prepare and run with `devenv shell -- bash -c 'cargo build -p two-way-sync-example --features capture && cargo run -p xtask -- gdextension --manifest-path examples/two-way-sync-demo/rust/Cargo.toml && godot --headless --path examples/two-way-sync-demo/godot --editor --import && GODOT_BEVY_CAPTURE=orbit-split cargo run -p two-way-sync-example --features capture'`; substitute `orbit-split-hidden` for the negative run. Capture inherits the demo scene and supplies the reset-relative `before_frame` index to GDScript's x writer and Bevy's y writer, which runs in `FixedUpdate` before `FixedLast`. The quad follows `(100 sin(n/50), 100 cos(n/50))`, independent of drawn-frame count. `bevy_step_ns` pins `Time<Virtual>`; `FixedUpdate` motion advances on `Time<Fixed>`, which the fixed driver sets from Godot's physics delta. Both clocks are pinned; they are not the same clock, and this trajectory uses the explicit frame index.
