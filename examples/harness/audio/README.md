# Platformer audio capture

`cues` records the platformer's jump and gem through its existing `AudioChannel` API. It stays on the shipped main menu, waits for both streams and menu connections, disables ordinary music/SFX observers, and fixes both cue gains to 0.8. Main-menu gameplay systems remain inactive. This scenario does not traverse into a level.

| Action | Frame | Wall time from recording start |
| --- | --- | --- |
| Jump | 30 | 500 ms |
| Gem | 90 | 1500 ms |
| Stop both channels | 100 | 1666.67 ms |
| Final drain | 210 | 3500 ms |

The gem source lasts about 249 ms, so stop interrupts it. The reviewed recording must include that truncated cue. Jump is compared inside 450–900 ms and gem inside 1450–1950 ms. Post-stop silence covers 2700–3400 ms.

`extensions.audio` declares the cue order, frames, windows, reference, mixer rate and thresholds. `schema/validate.py` reads `schema/extension.json`, rejects unknown fields and weakened budgets, and checks schedule/window relationships. It does not require the reference to exist during manifest validation. A missing reference fails the audio verdict after recording, leaving a candidate WAV for review.

## Run

Use a GUI-capable host with Godot 4.6.2 and a real audio driver. The manifest declares the mixer rate it was recorded at (`cues.json` says 44100, the Godot default; macOS CoreAudio follows the output device and ignores `audio/driver/mix_rate`). The hook fails when the live mixer differs from the declared rate and rejects Dummy. It records the selected driver and output-device name every frame.

From the repository root, prepare and enter through the shared launcher:

```bash
devenv shell -- cargo build -p platformer-2d-example --features capture-audio
devenv shell -- cargo run -p xtask -- gdextension --manifest-path examples/platformer-2d/rust/Cargo.toml
devenv shell -- godot --headless --path examples/platformer-2d/godot --editor --import
devenv shell -- env GODOT_BEVY_CAPTURE=cues cargo run -p platformer-2d-example --features capture-audio
```

Headless is only for import. The capture driver launches windowed Godot with real-time pacing. `capture-audio` enables `capture`, whose dependency recipe enables the hook and compiles this adapter. Both feature names work for this isolated audio implementation. Without `GODOT_BEVY_CAPTURE=cues`, the adapter and its observer suppression are inactive. Existing itest selection takes precedence.

For a named evidence run after preparation:

```bash
devenv shell -- python3 examples/harness/capture.py --example platformer-2d --check-library
devenv shell -- python3 examples/harness/capture.py --example platformer-2d --scenario cues --run audio-candidate-1 --timeout 120
```

Use a fresh run ID each time. Until a reference has been approved, the second command must exit 1 and print `reviewed reference WAV is missing`. The error is also in `audio/audio-verdict.json`. The core owns process cleanup, the final instruction/ACK handshake and exit codes.

## Record and approve a reference

No reference WAV ships with this implementation. The adapter never generates or replaces a reference. A maintainer must:

1. Run `audio-candidate-1` above on the intended display/audio host. Retain the whole evidence leaf at `target/example-evidence/audio-candidate-1/platformer-2d/cues/`.
2. Inspect `verdict.json`, `audio/audio-verdict.json`, the per-frame audio facts and stderr. Require all 210 drains, the declared mixer rate, a real driver, no discarded samples, no timing errors, and a complete core verdict with ACK. The missing-reference failure is expected. Resolve every other failure before approval.
3. Listen to `audio/capture.wav`: one jump, one interrupted gem, then silence. Review its waveform for cue timing, both channels, clipping and post-stop silence. The file must be stereo PCM16 at the declared mixer rate. Do not approve silence, a wrong cue or an incomplete recording just to establish a baseline.
4. Record the maintainer's approval, source run ID, Godot version, driver/device and WAV checksum in the review. Only then copy the candidate into the reference path:

   ```bash
   mkdir -p examples/harness/audio/references/platformer-2d
   cp -n target/example-evidence/audio-candidate-1/platformer-2d/cues/audio/capture.wav examples/harness/audio/references/platformer-2d/cues.wav
   shasum -a 256 examples/harness/audio/references/platformer-2d/cues.wav
   ```

5. Run three fresh captures. Require exit 0, a complete acknowledged core verdict, and `capabilities.audio == "pass"` on each. Review reference replacements by the same procedure; no command automatically updates them.

```bash
devenv shell -- python3 examples/harness/capture.py --example platformer-2d --scenario cues --run audio-repeat-1 --timeout 120
devenv shell -- python3 examples/harness/capture.py --example platformer-2d --scenario cues --run audio-repeat-2 --timeout 120
devenv shell -- python3 examples/harness/capture.py --example platformer-2d --scenario cues --run audio-repeat-3 --timeout 120
```

Mixer capture does not establish that speakers work. Listening acknowledgement belongs to the physical-device adapter and is not inferred from this verdict.

## Capture and comparison

The core calls `verdict.py` only after the last facts. Live pacing and draining therefore run in the Rust adapter; `audio_capture.py`, imported by `verdict.py`, consumes the resulting facts and raw samples. There is no second launcher or caller-invoked helper.

The manifest uses `pacing: "realtime"`, which removes `--fixed-fps`. The hook caps Godot at 60 process passes per second and waits for absolute monotonic deadlines in `before_frame(n)` before queuing that frame's cue. Python requires each deadline and drain to fall within 50 ms of `n / 60` and requires the sample clock to track wall time within 50 ms. Late runs fail; recordings are never stretched or resampled. The core still pins `Time<Virtual>` with `bevy_step_ns`. `Time<Fixed>` advances from Godot's 60 Hz physics delta, and can have zero or several ticks in a real-time process pass.

Reset queues stop/gain/pitch/panning commands, installs a one-second Master-bus capture buffer and sets a pending flag. A `PostUpdate` system clears the flag after the audio plugin's `Update` drain. `is_settled` stays false until then, and verifies both assets are loaded `AudioStream`s before any cue can be queued. Reset starts no cue. At frame 1 the adapter discards warm-up silence once, starts the wall clock, then continuously consumes the capture buffer through frame 210.

Each `PostUpdate` drain writes little-endian stereo float32 samples to `audio/mixer.f32`, flushes them, and emits one `CAPTURE_EXT adapter=audio frame=<n> <json>` line. Facts name the frame, contiguous sample offset/count, discarded-frame counter, actual mixer rate, driver/device, command and drain times, cue/stop command and final-drain flag. The core saves them as `frame-NNNNNN.audio.json`. The final raw bytes and audio fact are emitted in pass N before the hook's late `_process`, so Python can consume them immediately.

Godot's [AudioEffectCapture](https://docs.godotengine.org/en/4.6/classes/class_audioeffectcapture.html) supplies stereo sample frames and a dropped-frame counter. The adapter requires an initially empty Master effect chain, sets Master to 0 dB and unmutes it. `get_driver_name` uses a dynamic call because it is newer than the generated API 4.2 surface; runtime capture remains pinned to Godot 4.6.2.

Python verifies every drain, creates `audio/capture.wav` even when the reference is absent, and reads only `examples/harness/audio/references/<example>/<scenario>.wav` as the oracle. It compares each cue independently using normalized stereo cross-correlation and an FFT implemented with the standard library. Every integer sample lag within ±50 ms is considered. Both channels share one lag; the full comparison window remains the same length. Correlation must be at least 0.98 and aligned RMS level must be within 1 dB. A silent reference or muted cue fails. There is no loud-cue averaging that can hide a missing quiet cue.

Clipping, unexpected sound outside the cue windows plus alignment margins, incomplete recordings, missing silence samples and discarded samples fail. Post-stop silence uses peak amplitude strictly below −60 dBFS, which also rejects a brief click. A zero peak is represented as `post_stop_peak: 0` with a null dBFS value. Raw float samples are judged before PCM16 evidence conversion can clip them.

`audio/audio-verdict.json` records metrics, errors, the reference path and required audio artifacts. `verdict()` returns exactly `{"audio": "pass"}` or `{"audio": "fail"}`. It never sets another capability. Physical input, synthetic input, browser, 3D rendering and window presentation remain `untested`. The mandatory core checkpoints only check a visible menu root and a non-white corner; their RGB tolerance accepts any colour. They are not a platformer rendering qualification.

## Caller checks

The required signal tests and initial schema tests were written before implementation. Evidence checks were added during source review. None have been run in the adapter worktree.

```bash
python3 -m unittest discover -s examples/harness/audio/tests -v
python3 examples/harness/capture_schema.py examples/platformer-2d/capture/cues.json
devenv shell -- cargo check -p platformer-2d-example --features capture-audio
devenv shell -- cargo check -p platformer-2d-example --features capture
devenv shell -- cargo check -p platformer-2d-example
devenv shell -- cargo check -p godot-bevy --features api-4-2
devenv shell -- cargo check -p godot-bevy --no-default-features --features api-4-2
devenv shell -- cargo fmt --check
devenv shell -- cargo clippy -p platformer-2d-example --features capture-audio -- -D warnings
```

The first Cargo resolution will add the platformer's optional `serde` and `serde_json` dependency edges to `Cargo.lock`; both packages already exist in the workspace. The adapter's file scope excludes that root file. The caller should retain those lockfile changes during integration.
