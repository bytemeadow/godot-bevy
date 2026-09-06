---
name: editor-probe
description: Verify editor-facing Godot classes in a real editor with property assertions, scene save/reload, Inspector screenshots, and optional help text checks
---

# Editor probe

Use this after changing a class, exported field, Inspector hint, or property description that
designers see in the editor. A temporary `@tool` plugin instantiates the class, checks its
property metadata and default, edits a value, saves and reloads a scene, and captures the
Inspector. Runtime itests do not cover the editor UI.

## Prepare the project

Use Godot 4.6 with a display. On the hub, use a session attached to the GUI, not a bare SSH
shell. Run commands through `devenv shell --`. Check `df -h "/Volumes/DS Vault"` before builds.
The hub's `target/` is a shared symlink. Never run `cargo clean`.

Build the classes into the project's GDExtension before probing. For the platformer:

```bash
devenv shell -- cargo run --features itest --manifest-path examples/platformer-2d/rust/Cargo.toml
```

For itest, the runner builds the library, generates `itest/godot/itest.gdextension`, imports the
project, and runs the selected test:

```bash
devenv shell -- ./itest/run-tests.sh --filter inspector_metadata_roundtrip
```

It builds with `--features test-frame-signal,autosync-tests`. The itest crate already enables
`godot-bevy/register-docs`. A manual build alone does not generate its `.gdextension`.

Help assertions require `godot-bevy/register-docs` compiled into the extension. Enable that
feature when preparing other crates whose probes check descriptions. Build and import
perf-test with `devenv shell -- cargo run --manifest-path examples/perf-test/rust/Cargo.toml`,
then close the running example before probing.

## Run one probe

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
  "expect": {
    "type": "FLOAT",
    "hint": "NONE",
    "hint_string": "",
    "default": 1.5
  },
  "shots": "/tmp/editor-probe/JumpBoostCarrier"
}
```

`class`, `property`, `value`, and `shots` are required. `root` defaults to `Node2D` and
`node_name` to `Probe`. Use an absolute `shots` path. Values and defaults must be representable
in JSON, such as numbers, strings, and booleans.

All `expect` keys are optional. The plugin finds `property` in `get_property_list()` and checks
`type`, `hint`, and `hint_string` against that entry. Use Variant.Type names such as `FLOAT`
or `STRING`, and PropertyHint names such as `RANGE` or `ENUM`, without `TYPE_` or
`PROPERTY_HINT_` prefixes. `default` is read from the fresh node before editing. Missing keys
are not checked. Every mismatch appears in the probe's verdict line.

To check help text, add `"description": "Primary kind description"` inside `expect`.
The plugin calls
`EditorInterface.get_script_editor().goto_help("class_property:<class>:<property>")`, waits,
and checks that the visible help label contains the description. This traversal is pinned to
Godot 4.6: a visible `EditorHelp` below `ScriptEditor`, with a direct visible `RichTextLabel`
child. It fails clearly if the version differs or the label is missing.
The structure comes from [Godot 4.6's EditorHelp constructor](https://github.com/godotengine/godot/blob/4.6-stable/editor/doc/editor_help.cpp).

## Run a manifest

A manifest is a JSON list of probes. Each probe has the same fields as above plus `project`,
a path relative to the repo root, such as `examples/platformer-2d/godot` or `itest/godot`.
Give each probe its own `shots` directory.

```bash
devenv shell -- .claude/skills/editor-probe/scripts/run.sh --manifest .claude/skills/editor-probe/manifests/platformer-2d.json
devenv shell -- .claude/skills/editor-probe/scripts/run.sh --manifest .claude/skills/editor-probe/manifests/itest.json
devenv shell -- .claude/skills/editor-probe/scripts/run.sh --manifest .claude/skills/editor-probe/manifests/perf-test.json
```

The driver groups probes by project in first-seen order. It launches one editor per project,
runs that project's probes inside the plugin, then cleans up before launching the next editor.
A failed assertion does not stop the remaining probes. Each probe prints one
`EDITOR_PROBE verdict=...` line with its class, property, and mismatches. Any failure makes the
driver exit non-zero.

The checked-in manifests cover:

- `platformer-2d.json`: Player2D, Gem2D, Door2D, JumpBoostCarrier.
- `perf-test.json`: ParticleRain.
- `itest.json`: AutoSyncPlayerNode, InspectorPrimaryNode, InspectorTupleNode,
  InspectorNativeNode, TestMovementComponent, AttachCarrier.

Gem2D and ParticleRain have no exported fields of their own. Their probes edit the inherited
`editor_description`. The itest manifest checks the RANGE hint on `speed`, ENUM hints, and
the help description for `InspectorPrimaryNode.label`.

## Results and cleanup

Read the PNGs and inspect the Inspector panel. Each probe captures `1-default.png`,
`2-edited.png`, and `3-reloaded.png`. Description probes also capture `4-help.png`.
Captures use the editor viewport. `RenderingServer.force_draw()` is required because an idle
editor does not redraw; waiting on `frame_post_draw` can hang.

Exit codes: 0 all probes passed, 2 configuration or driver error, 3 missing class,
4 assertion or capture failure, 5 timeout or incomplete editor run. Across projects the
highest failure code wins. `EDITOR_PROBE_TIMEOUT` sets seconds per project, default 240.
`EDITOR_PROBE_LOG` sets the combined editor log, default `/tmp/editor-probe.log`.

The temporary scene is `res://scenes/editor_probe.tscn`. Cleanup restores `project.godot`
from its backup and deletes the temporary plugin, scene, and scene's `.uid`. The driver
refuses to overwrite those temporary paths if they already exist.

Godot rewrites `*.import` files on editor startup. Cleanup checks `git status` under the
project and runs `git checkout --` only for modified imports that were clean before the run.
It restores pre-existing import edits from separate backups. Other project edits survive.
After a run started from a clean project, verify `git status --short -- <project dir>` is empty.

Godot can crash during GDExtension shutdown after printing results. Complete probe verdicts
remain authoritative in that case. Missing verdicts or a timeout fail the run.
