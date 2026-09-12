# Examples

The [examples directory](https://github.com/bytemeadow/godot-bevy/tree/main/examples) contains four learning projects and a performance diagnostic. Run an example from the repository root:

```bash
cargo run --manifest-path examples/simple-node2d-movement/rust/Cargo.toml
```

Replace `simple-node2d-movement` with the directory name below. The launcher builds the Rust library, generates its GDExtension descriptor, and opens Godot. Put the Godot binary on your `PATH`.

## Learning projects

| Project | Lesson |
| --- | --- |
| [simple-node2d-movement](https://github.com/bytemeadow/godot-bevy/tree/main/examples/simple-node2d-movement) | Move a sprite with Bevy systems and transform sync. |
| [two-way-sync-demo](https://github.com/bytemeadow/godot-bevy/tree/main/examples/two-way-sync-demo) | Let GDScript and Bevy write different axes of the same transform. |
| [dodge-the-creeps-2d](https://github.com/bytemeadow/godot-bevy/tree/main/examples/dodge-the-creeps-2d) | Organize a game with states, signals, and a menu setting sent through the event bridge. |
| [platformer-2d](https://github.com/bytemeadow/godot-bevy/tree/main/examples/platformer-2d) | Build gameplay around editor-authored nodes, exported components, collisions, and audio. |

These projects run natively. The platformer enables editor property documentation, which requires Godot 4.3 or later. The simple movement web build is blocked on [#268](https://github.com/bytemeadow/godot-bevy/issues/268); its CI job remains disabled.

## Diagnostic tool

[perf-test](https://github.com/bytemeadow/godot-bevy/tree/main/examples/perf-test) compares rendered particle load in GDScript and godot-bevy. Use it to investigate performance on your machine. It is separate from the learning sequence, and its FPS readings are diagnostic.
