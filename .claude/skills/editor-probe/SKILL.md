---
description: Verify an Inspector-facing change in the real Godot editor - instantiate a GDExtension class in a scene, edit an exported property, save, reload, and capture editor screenshots, all scripted
---

# Editor probe

The itest suite runs in a real headless Godot, so it covers runtime behaviour. It cannot show
that a class appears in the editor, that its `#[export]` fields render in the Inspector, or that
an edited value survives save and reload. This skill does that with a temporary `@tool`
`EditorPlugin`, driven end to end with no clicking, and captures the editor's own viewport as
PNGs (no OS screenshot permission needed).

Use it after adding or changing a `GodotClass` that designers touch in the editor: an
`AttachableComponent` carrier, a `#[derive(GodotNode)]` class with exported fields, a new
Inspector hint. It proves the class registers, the property shows with its default, an edit
sticks, and the saved `.tscn` carries the value.

## Run

The class must already be compiled into the project's GDExtension; build it the way the project
normally is built (for an example, `cargo run --features itest --manifest-path
examples/<name>/rust/Cargo.toml` builds and imports it). Then:

```bash
devenv shell -- .claude/skills/editor-probe/scripts/run.sh examples/platformer-2d/godot probe.json
```

`probe.json`:

```json
{
  "class": "JumpBoostCarrier",
  "node_name": "JumpBoost",
  "root": "Node2D",
  "property": "multiplier",
  "value": 3.0,
  "shots": "/tmp/editor-probe/shots"
}
```

The driver enables a temporary plugin in `project.godot`, launches `godot --editor` on the
project, waits for it to finish, and prints the probe's output. Exit codes: 0 the reloaded
value matched, 3 the class does not exist in the editor, 4 the value did not survive reload,
5 the editor did not finish within the timeout. It restores `project.godot`, removes the plugin
and the probe scene, and reverts `.import` files the editor rewrote, so `git status` is clean
afterwards. Screenshots land in `shots/` as `1-default.png`, `2-edited.png`, `3-reloaded.png`;
read them and look at the Inspector panel.

## What it does inside the editor

Instantiates `root` with one child of `class`, saves it as `res://scenes/editor_probe.tscn`,
opens it, selects the child so the Inspector shows it, captures, sets `property` to `value`,
captures, saves, reloads the scene from disk, reads the property back, captures, and quits with
the verdict.

## Traps

- The editor needs a display. On a headless box there is nothing to capture; on the hub run it
  from a session attached to the GUI (the tmux session), not a bare SSH shell.
- The editor only redraws on input. Captures use `RenderingServer.force_draw()`; waiting on
  `frame_post_draw` hangs forever.
- `EditorScript` cannot be run from the command line; that is why this is a plugin toggled in
  `project.godot`.
- Godot rewrites `.import` files on every editor start. The driver reverts them; if you adapt
  it, keep that step or you will commit noise.
- The Godot import step can crash on exit with a GDExtension loaded (godotengine/godot#111645).
  The driver uses the verdict the probe prints, so a crash after it does not turn a pass into a failure.
