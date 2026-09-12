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
