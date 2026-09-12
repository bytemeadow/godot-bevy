# Example capture

Capture runs a prepared example in windowed Godot 4.6.2 and checks structural JSON at declared frames. The scenario manifest is the oracle. PNGs are evidence; there are no rendering goldens or image comparisons. Behavioural assertions still belong in itest.

`GODOT_BEVY_CAPTURE=<scenario>` selects capture in the shared launcher. The example's optional `capture` Cargo feature compiles its adapter. Without the environment selector, the example runs normally. The existing `itest` launcher takes precedence. Before dispatching, the launcher calls the driver's `--check-library` mode, which loads the actual platform/architecture `.debug` cdylib from `rust.gdextension` and calls the hook's exported C symbol `godot_bevy_capture_version() -> u32`. It must return 2. A missing symbol, missing library or wrong version exits 2 with an instruction to build with `--features capture`. Direct driver runs perform the same check. A capture-enabled launcher with a stale plain cdylib fails this check.

Transport strings, environment names, filenames, versions, clock constants and exits live in [`protocol.json`](../../godot-bevy-test/src/capture/protocol.json). Rust embeds it; Python loads it. [`schema/capture-v2.json`](schema/capture-v2.json) defines the core manifest. [`capture_schema.py`](capture_schema.py) applies its rules and the cross-field checks below. The core Python tools use only the standard library. Version 1 is rejected with: `capture version 1 is unsupported; set version to 2 and add extensions: {} and pacing: "fixed"`.

| Manifest key | Contract |
| --- | --- |
| `version` | Integer 2. All root keys in this table are required. |
| `example`, `scenario` | Lowercase letters, digits, `_`, `-`; first character is a letter or digit. Default file: `examples/<example>/capture/<scenario>.json`. |
| `scene` | Initial launch scene, a whitespace-free `res://...tscn` path without traversal. An adapter may traverse to another scene during warm-up. READY names the final scene. |
| `pacing` | Exactly `"fixed"` or `"realtime"`. Audio scenarios use `"realtime"`. |
| `extensions` | Object keyed by adapter name, using the same name syntax. `{}` is valid. Values are opaque JSON, including scalars, arrays and null. Reserved names: `rendering_2d`, `facts`, `diff`, `request`, `png`, because the core's per-frame files use those stems. Adapter names are global across examples and `itest/capture/<name>/` is shared. Naming an adapter after a baseline capability (`audio`, `physical_input`, `synthetic_input`, `browser`, `rendering_3d`, `window_presentation`) is how it sets that capability's verdict entry; any other name gets its own entry and leaves the baseline `untested`. |
| `frames` | Integer N in `[1,1000000]`. Frame 0 is reset state; 1 through N count subsequent process passes. Fixed pacing requires one physics tick per pass; realtime permits zero or several. |
| `seed` | Integer in `[0,4294967295]`. The core seeds Godot immediately before reset; adapters reset their own RNGs from this value. |
| `viewport` | `[width,height]`, each integer in `[1,8192]`, in image pixels. The root window uses viewport scaling at this size. |
| `clocks` | Exactly `{"physics_hz":60,"bevy_step_ns":16666667}`. These are protocol constants, not freely selectable rates. |
| `settle` | Exactly `{"stable_frames":K,"nodes":[...],"classes":[...]}`. K is in `[2,600]`; both lists are nonempty and unique. Class names use letters, digits and `_`, with a letter or `_` first. |
| `checkpoints` | At least two entries, each exactly `frame`, `nodes`, `regions`. Frames are strictly increasing, start at 0 and end at N. |
| checkpoint `nodes` | Nonempty list of `{"path":...,"position":[x,y],"tolerance":t,"visible":bool}`. Paths are unique within the checkpoint. |
| checkpoint `regions` | Nonempty list of `{"name":...,"rect":[x,y,w,h],"non_blank":{...},"dominant_colour":{...}}`. Names are unique within the checkpoint and use the adapter-name syntax. |

Unknown keys outside extension values fail, including unknown keys in nested core objects. Numbers must be finite; booleans are not numbers. Duplicate JSON keys fail. Node paths are relative to the current scene; `.` names its root. Absolute paths, `..` components, backslashes, whitespace and property subnames are forbidden. Positions are local `Node2D` or `Control` positions. Each axis uses an inclusive, nonnegative tolerance. Visibility is `CanvasItem.is_visible_in_tree()`. Missing or unsupported nodes produce null facts and fail comparison.

Region rectangles use integer viewport pixels with the origin at the top left. Width and height are positive, the whole rectangle fits inside the viewport, and sampling covers `[x,x+w)` and `[y,y+h)`. Regions may overlap. `non_blank` is exactly `{"background":[r,g,b],"tolerance":t,"min_fraction":f}`. RGB and tolerance are integers in `[0,255]`; `0 < f <= 1`. A pixel is non-blank when any RGB channel differs from the background by more than t. Its fraction must be at least f. Alpha is ignored.

`dominant_colour` is exactly `{"rgb":[r,g,b],"tolerance":t}`, with RGB and tolerance integers in `[0,255]`. The hook rounds clamped image channels to RGB8, counts exact triples, and picks the most frequent triple; ties choose the lexicographically smallest triple. Each channel uses inclusive tolerance. Expected position, visibility, minimum fraction and dominant RGB are used only by the comparator. There is no whole-image aggregation or PNG pixel oracle. Adapters must set any scenario-specific clear colour; the movement adapter sets black during reset.

For each `extensions.<name>`, supply these adapter-owned files:

| File under `itest/capture/<name>/` | Python interface |
| --- | --- |
| `schema/extension.json` | The named adapter's schema for its extension value. The core requires the file and leaves its interpretation to the adapter validator. |
| `schema/validate.py` | Required `validate(value) -> list[str]`. Read and enforce your schema here. Return `[]` on success and explanatory strings on failure. |
| `verdict.py` | Optional `verdict(value, output: pathlib.Path) -> dict[str,str]`. Read evidence from the full leaf and return exactly `{name: "pass"}`, `{name: "fail"}` or `{name: "untested"}`. |

`validate.py` and `verdict.py` run through `runpy.run_path`: `__name__` is not `"__main__"`, the adapter directory is not on `sys.path`, and each file is executed afresh on every call, so validation runs at least twice per run. Keep them self-contained, or insert `Path(__file__).parent` into `sys.path` yourself before importing a sibling helper. The standard library only.

The driver validates core fields, then invokes each named validator on a copy of the raw extension value. A missing schema or validator, rejected value, invalid return type or validator exception rejects the run with exit 2 before launch. Validators must be deterministic and free of side effects; validation may run more than once. The core never applies its structural expectation schema to an extension value. Keep all extension-specific constraints, including cue schedules and device rates, in that adapter's schema and validator.

The verdict step runs once after the final core FACTS and before the instruction is written. Nothing of the adapter's runs between `before_frame(N)` and that FACTS line, and the driver reads the evidence leaf the moment the line is accepted, so evidence for frame N must be finalised from a Bevy `Update` or `PostUpdate` system during pass N, before the hook's late `_process`. It receives a copy of its own extension value and the evidence leaf. It may inspect its own extra files and raw extension facts. A missing verdict step leaves that adapter `untested`; a bad return or exception fails the run with exit 1. It can set exactly its own capability entry. It cannot return another adapter's entry or `rendering_2d`. Extra required artifacts and their validation belong in this verdict function; a missing artifact must produce that adapter's failure. The core does not infer an adapter pass from a rendering pass.

Add the hook through the consumer's optional dependency and feature:

```toml
[dependencies]
godot-bevy-test = { workspace = true, optional = true }

[features]
capture = ["dep:godot-bevy-test", "godot-bevy-test/capture"]
```

The public Rust interface in `godot_bevy_test::capture` is:

```rust
pub fn is_enabled() -> bool;
pub fn install(app: &mut App, adapter: CaptureAdapter);
pub enum Phase { WarmUp, Running }
pub fn phase() -> Phase;
pub fn evidence_dir() -> Option<std::path::PathBuf>;
pub extern "C" fn godot_bevy_capture_version() -> u32;

pub struct CaptureAdapter {
    pub name: &'static str,
    pub extension: fn(&mut World, &CaptureManifest, &serde_json::Value) -> Result<(), String>,
    pub scenarios: &'static [&'static str],
    pub is_settled: fn(&mut World, &CaptureManifest) -> Result<bool, String>,
    pub reset: fn(&mut World, &CaptureManifest) -> Result<(), String>,
    pub before_frame: fn(&mut World, &CaptureManifest, u32) -> Result<(), String>,
}
```

`App` and `World` are Bevy types. `CaptureAdapter` is `Clone + Copy`; `Phase` is `Clone + Copy + Debug + PartialEq + Eq`. Store persistent callback state in the world. An adapter without extension data can use `extension: |_, _, _| Ok(())`; one without cues can use `before_frame: |_, _, _| Ok(())`. Gate the adapter module with `#[cfg(feature = "capture")]` and call `install` from the existing app builder. `install` is inert without the environment selector. Multiple named adapters can be installed; there is one internal hook and every installed adapter must support the chosen scenario. Names must be unique. Every extension key must have a matching installed adapter.

`extension` receives exactly the `serde_json::Value` at `manifest.extensions[adapter.name]`, once before the first settling callback. It is not called when that key is absent. Extension callbacks run in name order; settling, reset and frame callbacks run in installation order. Each callback has access to `capture::evidence_dir()`, the existing absolute `<leaf>/<adapter.name>/` directory. Outside callbacks this returns `None`. Cache the path in a world resource if a scenario system needs it. Adapter files stay inside that subdirectory; only the core writes its root-level transport files.

The manifest model is public and derives `Clone` and `Deserialize`; nested structs reject unknown fields:

```text
CaptureManifest {
  version: u32, example: String, scenario: String, scene: String,
  pacing: Pacing, extensions: BTreeMap<String, serde_json::Value>,
  frames: u32, seed: u64, viewport: [u32; 2],
  clocks: Clocks, settle: Settle, checkpoints: Vec<Checkpoint>
}
Pacing { Fixed, Realtime } // JSON: "fixed", "realtime"
Clocks { physics_hz: u32, bevy_step_ns: u64 }
Settle { stable_frames: u32, nodes: Vec<String>, classes: Vec<String> }
Checkpoint { frame: u32, nodes: Vec<NodeExpectation>, regions: Vec<RegionExpectation> }
NodeExpectation { path: String, position: [f64; 2], tolerance: f64, visible: bool }
RegionExpectation { name: String, rect: [u32; 4], non_blank: NonBlank, dominant_colour: DominantColour }
NonBlank { background: [u8; 3], tolerance: u8, min_fraction: f64 }
DominantColour { rgb: [u8; 3], tolerance: u8 }
```

`Pacing` also derives `Copy`, `Debug`, `PartialEq` and `Eq`. The driver performs full schema validation before the hook reads its snapshot. Calling the hook directly does not replace that validation.

Callbacks run on Godot's main thread between schedule invocations, while `BevyAppSingleton` is mutably bound. They have exclusive access to the existing `World`. Anything that reaches `BevyAppSingleton` from a callback is undefined: obtaining another binding, or calling GDScript or emitting a signal whose handler calls `BevyAppSingleton.send_event` (Dodge's `hud.gd` has exactly that shape). Signal connections and input events are channel-backed and safe to trigger from a callback, which is how menu traversal during warm-up is done. Finish synchronously, release Godot borrows before returning and use `Err(String)` for failures. For handles, use `SystemState<(..., GodotAccess)>`; release its parameters and call `SystemState::apply(world)` if it created deferred commands. Commands queued directly on the world need `world.flush()`. Neither operation drains the audio plugin's separate command queue.

`capture::phase()` is `WarmUp` until reset, post-reset settling and the reset draw have completed. The core does not run adapter scenario motion during warm-up: adapters must gate every Bevy system that mutates scenario state with a run condition. Setup/loading systems remain active. The movement adapter does this:

```rust
app.add_systems(
    FixedUpdate,
    orbit_system.run_if(|| capture::phase() == capture::Phase::Running),
);
```

Apply equivalent gating to scenario motion in GDScript through the adapter's own script state. `before_frame` is not called during warm-up. Phase becomes `Running` at READY and returns to `WarmUp` after frame N or a hook failure, so gated systems also stop while realtime shutdown awaits the instruction.

Every warm-up frame, starting with the hook's first late `_process`, calls every adapter's `is_settled`, even if the current scene is missing, the initial scene has changed or required nodes are absent. Only after all adapters return true does the core check the current scene, its readiness, every declared ready node and every registered class. The scene must be a saved `res://...tscn` scene. These conditions must hold for K consecutive frames. False conditions or a change of current scene identity reset the count. Scene changes before READY are permitted; `scene` names the starting scene, not a required final path. Use the real menu/button/action during setup. The adapter must report false until traversal, assets, textures, spawning and signal connections are complete. A sleep alone is not readiness.

Once K frames settle, the core seeds Godot, resets Bevy clocks, then calls each adapter's `reset` once. The adapter restores its timers, Rust RNGs, transforms, visibility and input state. Godot-visible transforms at frame 0 must already match Bevy; waiting for the next `FixedLast` is insufficient. Immediately after each reset returns, the core sets `TransformSyncMetadata.shadow = *Transform` for every synced entity. This reconciles the bridge's third transform copy and prevents a post-reset write from being skipped because it equals an old warm-up value. The core preserves `written_once`.

The core calls `is_settled` again immediately after reset. Audio reset may leave queued audio commands, and the audio plugin's queue is not public, so use this recipe: `reset` inserts a flag resource, a `PostUpdate` system removes it (the plugin drains in `Update`), and `is_settled` returns false while the flag exists. A `Play` re-queued because its asset was not ready is invisible to that recipe. Reset must not start audio or motion meant to begin at frame 0, because post-reset settle frames run with physics and audio live. Reset itself must return synchronously. If post-reset readiness is false, the core stays in warm-up, lets normal setup/audio draining run, and requires K consecutive ready frames again without calling reset twice. The adapter keeps reset scenario state stable while this finishes. Immediately before frame 0, the core zeros clocks and reseeds shadows again to discard post-reset warm-up time. A scene change or missing readiness between that point and the first natural draw restarts settling. READY is emitted only after the reset state is drawn.

`bevy_step_ns` pins `Time<Virtual>`; `FixedUpdate` motion advances on `Time<Fixed>`, which the fixed driver sets from Godot's physics delta ([fixed_schedule.rs](../../godot-bevy/src/plugins/fixed_schedule.rs)). In both pacing modes, both clocks are pinned; they are not the same clock. Virtual advances 16,666,667 ns per process pass. Godot's 60 Hz physics delta, passed through its single-precision callback value, becomes 16,666,668 ns per fixed tick. `Time<Fixed>` starts with that timestep, and the hook checks it and its elapsed time. Realtime physics tick count need not equal the process frame index.

Clock reset clears `Time<Real>`, `Time<Virtual>`, generic `Time` and `Time<Fixed>`, primes Real with a zero update so the first manual delta is full, and sets `TimeUpdateStrategy::ManualDuration(16666667 ns)`. Godot physics is set to 60 Hz with time scale 1 and interpolation disabled in Bevy Startup. This does not undo a camera's mode choice made earlier in `_ready`; an adapter that depends on that mode must configure it. Keep the tree unpaused and do not change the clocks. Runtime capture requires Godot 4.6.2; the Rust module keeps generated API 4.2 compatibility by using dynamic calls for newer Godot facilities.

The driver always passes `--windowed --render-thread safe --resolution <W>x<H> --scene <scene>`. Fixed pacing also passes `--fixed-fps 60`. Realtime drops `--fixed-fps` and uses Godot's normal wall-time pacing. Both modes keep the Bevy clocks above. Realtime does not promise an exact wall-clock duration for a numbered frame or qualify a mixer/device; the adapter must record and judge those facts.

In fixed mode, `SceneTree.physics_frame` calls `before_frame(world, manifest, n)` before tick n's node `_physics_process` callbacks and Bevy prefix/fixed schedules. The callback sees the preceding process frame's Bevy time. Put motion that must reach Godot in frame n in `FixedUpdate`, before transform sync's `FixedLast`; an `Update` transform write reaches the following fixed tick instead.

In realtime mode, `SceneTree.process_frame` calls `before_frame` once before process pass n, even if that pass has zero or multiple preceding physics ticks. Its prefix may already have run during physics, so use n as the scenario index instead of inferring it from the callback's clock value. Physics still advances `Time<Fixed>` on every Godot physics tick. Cues scheduled in `before_frame(n)` take effect in frame n's `_process`; the audio plugin drains commands in Bevy `Update`. The hook does not promise that synthetic input becomes visible in tick n: input adapters must establish and assert the normal input buffering boundary themselves.

The hook uses late `_process` priority `i32::MAX`; scenario nodes must have lower priority. It advances the frame, checks the fixed tick count in fixed mode, and completes capture at the natural `frame_post_draw`. A missing draw before the next process callback fails. At frame 0 it emits and flushes once:

```text
CAPTURE_READY scenario=<scenario> scene=<actual-final-scene> frame=0
```

The core locks the current scene identity at READY. Any subsequent removal or replacement, including a reload of the same scene file, fails. The session accepts the final READY scene path and requires that same path in all FACTS. Scene-relative node assertions therefore describe the final scene. A scene name alone does not prove that menu traversal succeeded; the adapter's predicate and evidence must establish that.

For fixed pacing, the driver publishes all checkpoint requests after matching READY. For realtime, it publishes them before starting Godot, so they already exist before READY. Each `frame-NNNNNN.request.json` contains exactly `{"version":2,"frame":n}`. Fixed checkpoint waits hold the frame until the request exists or the timeout expires. Realtime reads once and fails immediately if the request is missing; it never spins on a missing checkpoint request.

After the natural draw, queued redraw/texture work is reflected in the renderer. At each checkpoint the hook disconnects its own rendering callback, observes a separate `frame_post_draw` latch, calls `RenderingServer.force_draw`, requires synchronous completion and reads `root.get_texture().get_image()`. Safe rendering is pinned for this barrier. It saves the PNG atomically before emitting:

```text
CAPTURE_FACTS frame=<n> <single-line JSON>
```

Keep rendering callbacks observational because a forced evidence draw emits extra rendering signals. Use the supplied frame index to drive a script's capture counter; an adapter relying on `Engine.get_frames_drawn()` must establish its own offset and verify that mapping.

The core facts object is:

```json
{
  "version": 2,
  "scenario": "orbit",
  "scene": "res://main.tscn",
  "frame": 1,
  "viewport": [960, 640],
  "physics_ticks": 1,
  "virtual_elapsed_ns": 16666667,
  "fixed_step_ns": 16666668,
  "fixed_elapsed_ns": 16666668,
  "nodes": [{"path":"Icon","position":[99.98611,1.66659],"visible":true}],
  "regions": [{"name":"other-icon","rect":[804,314,4,16],"non_blank_fraction":1.0,"dominant_colour":[54,61,82]}],
  "configuration": {"godot":"4.6.2.stable.official","os":"macOS","renderer":"gl_compatibility","gpu":"reported adapter name"}
}
```

This illustrates the format; positions, regions and configuration are measured. FACTS arrive once per checkpoint, in manifest order, after READY. Malformed, missing, duplicate, out-of-order or non-finite facts fail. The driver checks version, frame, scenario, READY scene, viewport, virtual elapsed time, fixed timestep and fixed elapsed time. `physics_ticks` is a nonnegative integer, zero at frame 0; in fixed mode it must equal n. Fixed elapsed is `physics_ticks * 16666668`. Node and region identities must match the requested sets exactly. Configuration is retained as evidence without a capability verdict. PNG validation checks dimensions, chunks, CRCs, decompression and row filters without comparing pixel content to expectations. Missing or invalid PNG evidence makes the run incomplete.

Adapters may emit and flush:

```text
CAPTURE_EXT adapter=<name> frame=<n> <single-line JSON>
```

The name must be present in `extensions` and n must be in `[0,N]`. There may be at most one such line for each adapter/frame pair. Frame 0 extension evidence may arrive during reset before READY. Emit all extension evidence before the final core FACTS; later extension lines fail. The session validates JSON syntax and records the JSON payload verbatim, including its whitespace, plus a newline, into `frame-NNNNNN.<name>.json`. The complete original line remains verbatim in `stdout.log`. The core does not interpret the payload or apply its comparator to it. No EXT line is required by the core; the adapter verdict must enforce any required frames or content. `CaptureSession.accept` matches protocol prefixes as whole whitespace-delimited tokens. A line such as `CAPTURE_FACTS_AUDIO ...` is ordinary output and is ignored by the core parser.

At N, the driver runs extension verdict steps and atomically writes only this exit instruction:

```json
{"version":2,"scenario":"orbit","exit_code":0}
```

The filename is `instruction.json`. Its only legal exit codes are 0 and 1. The hook validates its three keys, version and scenario, reads the code, emits and flushes `CAPTURE_ACK exit=<0|1>`, then quits Godot with that code. Fixed mode waits at N; realtime polls once per subsequent process pass without blocking and without further scenario callbacks. An early, duplicate or wrong ACK fails. The driver requires the ACK after the instruction and the matching process exit. A process that emits its final FACTS and exits 0 without reading the instruction fails. `verdict.json` is written only after shutdown/cleanup, so a partial run cannot leave a provisional pass report.

Evidence is allocated at `target/example-evidence/<run>/<example>/<scenario>/`. A leaf must be new. The default run ID is UTC `YYYYMMDDTHHMMSSZ-<8 lowercase hex>`; custom IDs start with a letter/digit and contain letters, digits, `_` or `-`. Frame numbers use a minimum width of six decimal digits, without truncation.

| File | Writer and contents |
| --- | --- |
| `manifest.json` | Driver; full validated manifest, including all opaque extension values. |
| `driver.pid`, `godot.pid` | Driver; decimal PIDs plus newline. Godot's PID is also its POSIX process-group ID. |
| `stdout.log`, `stderr.log` | Driver; engine output, retained on failure. |
| `frame-NNNNNN.request.json` | Driver; v2 request, after READY in fixed mode and before process launch in realtime. |
| `frame-NNNNNN.png` | Hook; viewport evidence before FACTS. |
| `frame-NNNNNN.facts.json` | Driver; FACTS JSON, pretty-printed with a trailing newline. |
| `frame-NNNNNN.diff.json` | Driver; array of `{path,expected,actual}`, `[]` on a match. |
| `frame-NNNNNN.<name>.json` | Driver; uninterpreted EXT JSON payload, preserving whitespace. |
| `<name>/...` | Named adapter; extra files such as WAVs or traces. |
| `instruction.json` | Driver; version, scenario and exit code for the hook. It is not a verdict. |
| `verdict.json` | Driver; terminal report after ACK/exit checking and cleanup. |

Core JSON publication uses `<filename>.tmp` followed by rename; PNG uses `frame-NNNNNN.png.tmp`. A crash may leave these temporary files. EXT payloads and PID files are written directly. The terminal verdict lists `version`, `example`, `scenario`, `scene` (READY's path, or null), `outcome`, `complete`, `exit_code`, `required_artifacts`, `missing_artifacts`, `checkpoints`, `differences`, `errors`, `capabilities`, `acknowledged` and `command`. Aggregate differences add `frame`. Required artifacts include the manifest, PID pair, logs, requests, PNG/facts/diff files, received EXT files, instruction and verdict. The verdict does not require itself to preexist before publication. Adapter subdirectory requirements belong to its verdict step.

`complete` requires READY, every checkpoint, all required evidence, ACK and no transport/process/artifact errors. A complete comparison mismatch can have `complete:true`. An adapter verdict of `fail` makes the overall run fail; absent verdicts remain `untested`. `capabilities.rendering_2d` records core rendering/evidence status. The baseline entries `audio`, `physical_input`, `synthetic_input`, `browser`, `rendering_3d`, `window_presentation` start `untested`. Every name in `extensions` has an entry initially `untested`, and only its own verdict function can update it. A core rendering pass does not qualify audio, input devices, browsers, desktop-window presentation or 3D rendering.

The driver creates a separate POSIX process group for Godot; Windows uses a new process group and `taskkill /T`. SIGINT/SIGTERM start cleanup; subsequent signals are ignored through terminal verdict publication. Cleanup sends TERM, allows two seconds, escalates to KILL, and allows five seconds to reap. Cleanup errors are retained and cannot turn a timeout into exit 2. Logs and known artifacts remain on failure. SIGKILL cannot execute driver cleanup: the PID pair lets the caller locate an orphan. Before manually signalling an old PID/group, inspect it to avoid PID-reuse mistakes. No automatic parent-death watchdog is promised.

| Exit | Meaning |
| --- | --- |
| 0 | Core rendering checks passed, no adapter reported failure, evidence is complete, ACK and process exit match. Untested capabilities remain untested. |
| 1 | Mismatch, timeout, interruption, incomplete/invalid evidence, hook/transport/adapter-verdict error, cleanup failure or missing/wrong ACK/exit. |
| 2 | Invalid invocation/manifest/adapter validation, allocation failure, unprepared project, `GODOT_BEVY_ITEST` conflict, missing executable/library/capture marker or other launch configuration error. |

Manifest/CLI rejection occurs before evidence allocation. Direct `run_capture` also rejects invalid manifests with 2 before constructing a session. Configuration errors found after session creation retain an `outcome:"error"`, `complete:false`, exit-2 terminal verdict; launched-run failures use `outcome:"fail"`. The hook accepts only 0/1 instructions. In-process errors emit `CAPTURE_ERROR <JSON string>` and quit 1.

The driver sets `GODOT_BEVY_CAPTURE` (scenario), `GODOT_BEVY_CAPTURE_MANIFEST` (absolute snapshot path), `GODOT_BEVY_CAPTURE_OUTPUT` (existing absolute evidence leaf), and `GODOT_BEVY_CAPTURE_TIMEOUT` (positive wall-clock seconds). Consumers use the hook interfaces instead of another environment parser. Keep `GODOT_BEVY_ITEST` unset. Godot discovery is `--godot`, then `GODOT4_BIN`, then `GODOT`, then `godot` on PATH. `--timeout` defaults to 60 seconds, range `(0,3600]`, covering engine startup, settling, capture and handshake; bounded cleanup grace may extend it. Godot's descriptor and `.godot` imports must already exist. Capture builds/imports nothing. The `.debug` descriptor entry is selected even after a release build; prepare that entry for the library profile you intend to capture.

Reusable Python interfaces are `load_manifest(Path) -> dict`, `validate_manifest(document) -> list[str]`, `compare_checkpoint(manifest, checkpoint, facts, *, scene=None) -> list[dict]`, `frame_file(frame, kind) -> str`, `write_json(path, value)`, and `CaptureSession(manifest, output)`. `kind` is `request`, `png`, `facts` or `diff`. `compare_checkpoint` defaults scene to the launch scene; a session supplies its READY scene. `CaptureSession.accept(line)` processes output. `comparison_verdict()` describes current comparison evidence without certifying a handshake; `instruction()` evaluates adapters and selects the expected exit; `verdict()` includes ACK and transport-file requirements. `run_capture(manifest, output, godot, timeout) -> int` owns one prepared process. Call it on Python's main thread with a validated manifest and fresh absolute leaf because it handles signals.

The movement consumer uses fixed pacing and `{}` extensions. Its predicate requires six Orbiters with nonempty textures plus the declared classes and nodes. Reset sets all six Bevy/Godot positions to authored initial position plus `(100,0)`, visibility true, clear colour black and the next angle to one `Time<Fixed>` step. Motion accumulates that f32 step in `FixedUpdate`; it is approximately `(100 cos(n/60),100 sin(n/60))`, and `OtherIcon` adds `(500,1)`. Core shadow reseeding makes tick 1 visible. Normal mode keeps `Update` motion.

`orbit.json` checks 0/1/60/120. Frame 1 expects Icon `[99.98611,1.66659]` and OtherIcon `[599.98611,2.66659]` within 0.001 per axis, so frame-0 state cannot pass. `orbit-empty-region.json` checks 0/1, with exactly one intended difference: `regions.empty-corner.non_blank_fraction` at frame 0. Its frame-1 node and region assertions must pass. `orbit-frames-0-1-2.json` keeps scenario name `orbit` and checks frames 0/1/2, including the same frame-1 assertion; pass it explicitly with `--manifest`.

Caller commands from the repository root (these are not run by the author of round 2):

```bash
python3 -m unittest discover -s itest/capture/tests -v
devenv shell -- cargo test -p godot-bevy-test --features capture
devenv shell -- ci-lint
devenv shell -- cargo clippy -p godot-bevy-test -p simple-node2d-movement-example --all-targets --features godot-bevy-test/capture,simple-node2d-movement-example/capture -- -D warnings
devenv shell -- cargo check -p godot-bevy --no-default-features --features api-4-2
devenv shell -- cargo check -p godot-bevy-test --features capture,godot-bevy/api-4-2
python3 itest/capture/capture_schema.py examples/simple-node2d-movement/capture/orbit.json
python3 itest/capture/capture_schema.py examples/simple-node2d-movement/capture/orbit-empty-region.json
python3 itest/capture/capture_schema.py examples/simple-node2d-movement/capture/orbit-frames-0-1-2.json
```

Prepare the default-API debug extension and import assets before display runs:

```bash
devenv shell -- cargo build -p simple-node2d-movement-example --features capture
devenv shell -- cargo run -p xtask -- gdextension --manifest-path examples/simple-node2d-movement/rust/Cargo.toml
devenv shell -- godot --headless --path examples/simple-node2d-movement/godot --editor --import
devenv shell -- python3 itest/capture/capture.py --example simple-node2d-movement --check-library
```

Run the following in the host's GUI-capable session. Each positive must return 0 with a complete verdict and ACK. The negative must return 1, still complete, with exactly the one frame-0 empty-corner difference. The probe must return 0 and match frame 1 itself; there is no frame offset allowance. Use fresh `--run` IDs when repeating this batch.

```bash
devenv shell -- python3 itest/capture/capture.py --example simple-node2d-movement --scenario orbit --run core-round2-repeat-1
devenv shell -- python3 itest/capture/capture.py --example simple-node2d-movement --scenario orbit --run core-round2-repeat-2
devenv shell -- python3 itest/capture/capture.py --example simple-node2d-movement --scenario orbit --run core-round2-repeat-3
devenv shell -- python3 itest/capture/capture.py --example simple-node2d-movement --scenario orbit-empty-region --run core-round2-negative
devenv shell -- python3 itest/capture/capture.py --example simple-node2d-movement --scenario orbit --manifest examples/simple-node2d-movement/capture/orbit-frames-0-1-2.json --run core-round2-early
```

To enter through the shared launcher after preparation:

```bash
devenv shell -- env GODOT_BEVY_CAPTURE=orbit cargo run -p simple-node2d-movement-example --features capture
```

After the display batch, inspect the retained verdicts and compare structural facts across the three repeats with this command. It makes no PNG comparison:

```bash
python3 - <<'PY'
import json
from pathlib import Path

root = Path('target/example-evidence')
repeats = [root / f'core-round2-repeat-{n}/simple-node2d-movement/orbit' for n in (1, 2, 3)]
for leaf in repeats:
    verdict = json.loads((leaf / 'verdict.json').read_text())
    assert verdict['exit_code'] == 0 and verdict['complete'] and verdict['acknowledged'], verdict
for frame in (0, 1, 60, 120):
    points = [json.loads((leaf / f'frame-{frame:06d}.facts.json').read_text()) for leaf in repeats]
    for point in points:
        point.pop('configuration', None)
    assert points[0] == points[1] == points[2], (frame, points)
negative = json.loads((root / 'core-round2-negative/simple-node2d-movement/orbit-empty-region/verdict.json').read_text())
assert negative['exit_code'] == 1 and negative['complete'] and negative['acknowledged'], negative
assert [(d['frame'], d['path']) for d in negative['differences']] == [(0, 'regions.empty-corner.non_blank_fraction')], negative
probe = root / 'core-round2-early/simple-node2d-movement/orbit'
verdict = json.loads((probe / 'verdict.json').read_text())
assert verdict['exit_code'] == 0 and verdict['complete'] and verdict['acknowledged'], verdict
for frame in (0, 1, 2):
    assert json.loads((probe / f'frame-{frame:06d}.diff.json').read_text()) == [], frame
point = json.loads((probe / 'frame-000001.facts.json').read_text())
icon = next(node for node in point['nodes'] if node['path'] == 'Icon')
assert all(abs(a - b) <= 0.001 for a, b in zip(icon['position'], [99.98611, 1.66659])), icon
print('Three repeats match; negative has one difference; frames 0/1/2 match without an offset.')
PY
```
