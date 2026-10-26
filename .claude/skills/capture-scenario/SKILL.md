---
name: capture-scenario
description: Run an example's capture scenarios in a real windowed Godot and read their verdicts, or add a scenario or adapter to the capture harness
---

# Capture scenario

Use this when a change affects what a running example actually renders, plays or reads from
input: transform sync, visibility, scene traversal, audio cues, replayed input. The harness runs
the example in windowed Godot 4.6.2, resets the scenario to a seeded state and compares node
facts and viewport region statistics against a manifest at declared frames. The manifest is the
oracle; PNGs are written as evidence and never compared. Behavioural assertions that do not need
real frames still belong in itest. The full contract is `itest/capture/README.md`.

## Prepare

Use a display. On the hub, a session attached to the GUI, not a bare SSH shell. Run commands
through `devenv shell --`. The hub's `target/` is a shared symlink; never run `cargo clean`.

Every example has a `capture` Cargo feature. The platformer also has `capture-audio` and
`capture-input`, which imply `capture`; build it with both to serve all of its scenarios from
one library. The workspace clippy never enables these features, so lint the crate you touched
with them:

```bash
devenv shell -- cargo clippy -p platformer-2d-example --all-targets --features capture-audio,capture-input -- -D warnings
```

## Run

```bash
devenv shell -- .claude/skills/capture-scenario/scripts/run.sh dodge-the-creeps-2d capture title-and-first-creeps title-unseeded
devenv shell -- .claude/skills/capture-scenario/scripts/run.sh platformer-2d capture-audio,capture-input title level level-wrong-spawn input-replay cues
```

The script builds, writes the `.gdextension`, imports the project once, checks the built
library exports the capture hook and runs each scenario. It prints one `CAPTURE <scenario>`
line with the exit code, outcome, capabilities, differences and the evidence leaf. Exit codes
from the driver: 0 passed, 1 a checked assertion failed, 2 the run could not happen (wrong
manifest, stale library, timeout, crash). Every example ships a negative scenario that must
exit 1 on exactly one named field, so judge each line against what the scenario is for.

`CAPTURE_RUN` names the run; the default is a UTC timestamp. The evidence directory refuses
to overwrite, so reruns need a new run name. `CAPTURE_TIMEOUT` is seconds per scenario,
default 120.

Scenarios and their expected outcomes:

| Example | Passes | Fails on one field |
| --- | --- | --- |
| simple-node2d-movement | `orbit` | `orbit-empty-region` |
| two-way-sync-demo | `orbit-split` | `orbit-split-hidden` |
| dodge-the-creeps-2d | `title-and-first-creeps` | `title-unseeded` |
| perf-test | `rain-2000` | `rain-2000-unspawned` |
| platformer-2d | `title`, `level`, `input-replay` | `level-wrong-spawn` |
| platformer-2d | `cues` (audio) | fails until a reviewed reference WAV exists |
| platformer-2d | `devices` (physical input) | needs a person and a controller; `untested` otherwise |

## Read the results

Evidence lives under `target/example-evidence/<run>/<example>/<scenario>/`: `verdict.json`,
per-checkpoint `frame-NNNNNN.{facts,diff,request}.json` and `.png`, `stdout.log`,
`stderr.log`, and adapter subdirectories such as `audio/capture.wav`. Read the PNG of a
checkpoint when a difference is surprising; it shows what the facts describe. A verdict with
`complete: false` or an `errors` entry is a broken run, not a failed assertion.

Realtime scenarios (audio) fail their pacing budget when the machine is loaded, for example
during another session's builds. Rerun on an idle machine before reading anything into it.

Godot rewrites tracked `*.import` files on import. Discard that churn with
`git checkout -- <project>` unless you changed those files. Keep new `*.uid` files for scripts
you added; the repo tracks them.

## Add a scenario

A scenario is `examples/<example>/capture/<scenario>.json` validated by
`python3 itest/capture/capture_schema.py <manifest>`. Copy a neighbour, keep `version: 2`,
`extensions: {}` and `pacing: "fixed"` unless the adapter needs real time, and put the
scenario name in the adapter's `scenarios` list. Add the matching negative: the same manifest
with one expectation deliberately wrong, so the comparator's exact difference is on record.
Node positions are local `Node2D` positions with an inclusive tolerance; regions are viewport
rectangles with a `non_blank` minimum fraction and a `dominant_colour`.

## Add an adapter

An adapter is a Rust module behind the example's `capture` feature that calls
`godot_bevy_test::capture::install` with a `CaptureAdapter`: readiness, reset, per-frame
cues. Gate every system that moves scenario state on
`capture::phase() == Phase::Running`; reset must leave Godot-visible transforms already
matching Bevy. When several adapters live in one crate, each must install only when the
selected scenario is its own, because the core rejects a run where an installed adapter does
not list the scenario. An adapter that sets a capability verdict (`audio`, `physical_input`,
`synthetic_input`, `browser`) owns `itest/capture/<name>/` with `schema/extension.json`,
`schema/validate.py`, `verdict.py` and its unit tests; CI runs those tests from the lint job.

The entity viewer's platformer scenarios add `level-speed-control` and `level-speed-edit`
(expected pass), plus `level-speed-edit-wrong-position` (expected failure on frame-30 player x).
See `itest/capture/debugger/README.md` for the shared initial velocity, endpoint acknowledgement
and predicted resting positions. Include all three in a run to compare the same motion with
and without a speed edit.
