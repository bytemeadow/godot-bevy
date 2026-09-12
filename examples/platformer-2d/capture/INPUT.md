# Input capture

Build and prepare the platformer from the repository root:

```bash
devenv shell -- cargo build -p platformer-2d-example --features capture,capture-input
devenv shell -- cargo run -p xtask -- gdextension --manifest-path examples/platformer-2d/rust/Cargo.toml
devenv shell -- godot --headless --path examples/platformer-2d/godot --editor --import
devenv shell -- python3 itest/capture/capture.py --example platformer-2d --check-library
```

Run the device session in a GUI-capable session with the Godot window focused:

```bash
devenv shell -- python3 itest/capture/capture.py --example platformer-2d --scenario devices --run input-devices-1 --timeout 180
```

Follow the prompts in the window. Hold each control until the release prompt appears. The session asks for a mouse press and release on Start, keyboard movement and jump, both horizontal stick directions and neutral returns, and a controller jump. Each step allows five seconds. The 4,800 frames provide a nominal 80-second budget, including ten seconds beyond the sum of the deadlines. The adapter caps rendering at 60 FPS; the manifest uses realtime pacing. A slow machine can take longer, so the driver timeout is 180 seconds.

The saved menu remains the current scene throughout capture. The adapter loads the shipped first level as its child during settling, verifies its player and sprite are ready, and connects the shipped Start button. The physical click reveals that level and enables the existing player systems. These input scenarios use their own setup branch; the ordinary game’s scene replacement, menu navigation and audio playback are outside their verdict.

The Rust recorder timestamps each prompt at its first `frame_post_draw` and each raw event at `_input`, using nanoseconds elapsed from one monotonic `Instant`. It records matching and unrelated events, Godot device IDs, controller names/GUIDs, and `GodotActions` snapshots in `PostUpdate`. Keyboard and mouse identity is the device slot Godot exposes; it does not distinguish multiple keyboards that the OS combines into one slot. Evidence is written before the core’s final facts. Adapter Python runs during validation and after capture, with no Python prompt loop.

[`device-input.json`](device-input.json) is the expected sequence. The physical verdict checks it against the manifest, then requires each event to arrive within its prompt’s deadline and the matching action edge to be observed by the success processing pass. Processing an on-time arrival on the next frame does not make it late. Releases and stick neutral returns must follow the corresponding press or direction on the same device. A neutral action snapshot alone cannot satisfy a requested neutral event. A device kind that produces no event during the run is recorded in `missing_devices` and makes `physical_input` `untested`. Incomplete evidence fails. The core’s exit code alone does not qualify hardware; inspect `verdict.json`’s `capabilities.physical_input`.

Run the saved keyboard trace separately:

```bash
devenv shell -- python3 itest/capture/capture.py --example platformer-2d --scenario input-replay --run input-replay-1 --timeout 60
```

`input-replay.json` injects paired press/release events through `Input.parse_input_event` in `before_frame`. Movement and jump use `physical_keycode`; Enter uses the project’s logical `ui_accept` binding. The adapter checks each binding against the live `InputMap`. The verdict requires exactly one matching raw event and the matching `GodotActions` edge between the injection frame and two frames later, inclusive. This declared buffering allowance still needs caller runtime verification. Replay sets only `synthetic_input`; `physical_input` remains `untested`.

The `capture` feature enables the hook and includes `capture-input`. `capture-input` alone cannot enable the hook marker. Both scenarios reject a manifest containing another extension, so the physical scenario cannot enable replay. The golden is read from this directory and is never replaced by capture.

The standard evidence leaves are `target/example-evidence/<run>/platformer-2d/<scenario>/`. Each requires every `frame-NNNNNN.physical_input.json` or `frame-NNNNNN.synthetic_input.json` from 1 through N, plus the [core evidence](../../../itest/capture/README.md). The committed manifests retain PNGs at frames 0 and N and check the prompt overlay structurally. They do not compare images.

Run the adapter tests and schema validation:

```bash
python3 -m unittest discover -s itest/capture/physical_input/tests -v
python3 -m unittest discover -s itest/capture/synthetic_input/tests -v
python3 itest/capture/capture_schema.py examples/platformer-2d/capture/devices.json
python3 itest/capture/capture_schema.py examples/platformer-2d/capture/input-replay.json
```
