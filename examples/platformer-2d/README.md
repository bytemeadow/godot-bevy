# Platformer 2D Example

This platformer uses scenes configured in the Godot editor. Rust components define exported properties and tag nodes for Bevy systems. The game includes player movement, collisions, level changes, and audio.

## Running this example

From the repository root, with Godot on your `PATH`:

```bash
cargo run --manifest-path examples/platformer-2d/rust/Cargo.toml
```

The launcher builds the library, generates its GDExtension descriptor, and opens Godot.

This example enables `godot-bevy/register-docs` for property help in the editor. That feature requires Godot 4.3 or later. The editor-probe help check uses Godot 4.6.

## Copying

Template Code: Brett Chalupa (CC0, dedicated to public domain)
Sprites: 1-Bit Platformer Pack by Kenney (CC0)
Music: Ted Kerr (CC-BY 4.0) - 8-Bit Theme & 8-Bit Quirky Waltz
Sound Effects: jsfxr
