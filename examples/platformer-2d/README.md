# Platformer 2D Example

This platformer uses scenes configured in the Godot editor. Rust components define exported properties and tag nodes for Bevy systems. The game includes player movement, collisions, level changes, and audio.

## Running this example

From the repository root, with Godot on your `PATH`:

```bash
cargo run --manifest-path examples/platformer-2d/rust/Cargo.toml
```

The launcher builds the library, generates its GDExtension descriptor, and opens Godot.

This example enables `godot-bevy/register-docs` for property help in the editor. That feature requires Godot 4.3 or later. The editor-probe help check uses Godot 4.6.

The [capture harness](../../itest/capture/README.md) provides two rendering scenarios with the optional `capture` feature: `title` checks the authored title and credits at frames 0/30; `level` focuses Start and injects its bound Enter key during warm-up, then checks the player at spawn at frame 0 and at its analytical resting position at frames 30/120. Level capture names the player `Player2D` and disables camera drag and smoothing so the sprite region stays fixed. `level-wrong-spawn` must exit 1 with only `nodes.Player2D.position` differing at frame 0. All three use empty extensions and retain PNGs as evidence. After the harness's build, descriptor and import preparation, enter with `devenv shell -- env GODOT_BEVY_CAPTURE=title cargo run -p platformer-2d-example --features capture` (substitute either level scenario as needed). Audio and input capabilities remain untested.

## Copying

Template Code: Brett Chalupa (CC0, dedicated to public domain)
Sprites: 1-Bit Platformer Pack by Kenney (CC0)
Music: Ted Kerr (CC-BY 4.0) - 8-Bit Theme & 8-Bit Quirky Waltz
Sound Effects: jsfxr
