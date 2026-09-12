# Browser qualification preparation

Browser qualification is blocked on [godot-bevy#268](https://github.com/bytemeadow/godot-bevy/issues/268). The CI condition stays exactly:

```yaml
if: false && contains(needs.changes.outputs.examples, 'simple-node2d-movement')
```

Before re-enabling that job, both the `Web` export (`web-nothreads`) and `Web Threaded` export (`web`) must load in Chromium, report readiness, and show visible orbit motion over 60 Godot process frames. Compiling both WASM libraries is insufficient. The job must run those browser checks before its gate is removed. This preparation has no web execution evidence.

`verdict(value, output)` always returns exactly `{"browser":"untested"}`, even if diagnostics detect motion. No manifest currently adds `extensions.browser` or installs a Rust browser adapter. The reserved extension schema accepts only `{}`. Adding it to today's movement manifest would require a matching installed adapter and is not part of this preparation. The existing native `capture` feature already enables `godot-bevy-test/capture` as required by the library check.

## Checks without a WASM build

Run from the repository root:

```bash
python3 -m unittest discover -s itest/capture/browser/tests -v
node --check itest/capture/browser/browser.mjs
bash -n examples/simple-node2d-movement/build-web.sh
```

The Python tests use only the standard library. They exercise HTTP GET, HEAD and error responses for both header modes, WASM MIME type, synthetic translated and identical images, strict threshold boundaries, PNG decoding and the unconditional untested verdict. These tests were written before the implementation; neither a red nor a green run was executed by the author.

## Prepare exports after #268 clears

Use the repository toolchain, Godot 4.6.2 and matching web export templates with extension support. Run this complete sequence from the repository root. The descriptor generator writes native entries only, so the Python block adds explicit `threads` and `nothreads` web entries to that freshly generated, ignored descriptor. Both export modes use the release WASM libraries built here. Godot uses feature tags to select a GDExtension library. [Godot feature tags](https://docs.godotengine.org/en/4.6/tutorials/export/feature_tags.html)

```bash
set -e
devenv shell -- cargo build -p simple-node2d-movement-example --features capture
devenv shell -- cargo run -p xtask -- gdextension --manifest-path examples/simple-node2d-movement/rust/Cargo.toml
devenv shell -- examples/simple-node2d-movement/build-web.sh release
python3 - <<'PY'
from pathlib import Path

descriptor = Path('examples/simple-node2d-movement/godot/rust.gdextension')
with descriptor.open('a') as stream:
    for mode in ('debug', 'release'):
        for feature, suffix in (('nothreads', ''), ('threads', '.threads')):
            library = f'simple_node2d_movement_example{suffix}.wasm'
            stream.write(f'\nweb.{mode}.wasm32.{feature} = "res://../../../target/wasm32-unknown-emscripten/release/{library}"\n')
PY
mkdir -p target/browser-exports/web-nothreads target/browser-exports/web
devenv shell -- godot --headless --path examples/simple-node2d-movement/godot --editor --import
devenv shell -- godot --headless --path examples/simple-node2d-movement/godot --export-release "Web" "$(pwd)/target/browser-exports/web-nothreads/index.html"
devenv shell -- godot --headless --path examples/simple-node2d-movement/godot --export-release "Web Threaded" "$(pwd)/target/browser-exports/web/index.html"
```

`build-web.sh` builds both feature variants, including when its legacy `--serve` switch is present. The rejected `-Zemscripten-wasm-eh=false` flag was already absent. For qualification, use the server below: the legacy `--serve` branch still points at the absent `examples/web-demo/serve.py`, outside the requested build-script changes.

## Serve and probe each export

Start these in two terminals and retain their logs if diagnosing an HTTP failure:

```bash
python3 itest/capture/browser/serve.py target/browser-exports/web-nothreads --variant web-nothreads --port 8060
```

```bash
python3 itest/capture/browser/serve.py target/browser-exports/web --variant web --port 8061
```

The threaded server sends `Cross-Origin-Opener-Policy: same-origin` and `Cross-Origin-Embedder-Policy: require-corp` on every response. The non-threaded server omits both. Both disable caching and serve WASM as `application/wasm`. The server binds to loopback by default; use localhost HTTP or a suitable HTTPS origin for browser isolation. These are the headers prescribed by [Godot's web serving documentation](https://docs.godotengine.org/en/4.6/tutorials/export/exporting_for_web.html#serving-the-files).

With a Node-resolvable `playwright` or `playwright-core` package and its Chromium executable already provisioned:

```bash
node itest/capture/browser/browser.mjs --url http://127.0.0.1:8060 --variant web-nothreads --output target/example-evidence/browser-nothreads-1/simple-node2d-movement/browser-orbit
node itest/capture/browser/browser.mjs --url http://127.0.0.1:8061 --variant web --output target/example-evidence/browser-threaded-1/simple-node2d-movement/browser-orbit
```

Each output leaf must be new. No dependencies or browsers are downloaded. Missing Playwright returns 2 with a pointer to the manual path. A missing Chromium executable, startup error, timeout, wrong headers/variant, missing READY, missing screenshots or insufficient motion returns nonzero. `--timeout` is the whole probe budget in seconds (default 60, maximum 3600); browser cleanup may extend it. SIGINT/SIGTERM and failures close the owned Chromium and retain available evidence. Stop each separately started server with Ctrl+C when finished.

The driver checks the preset tag, actual Godot thread support and `crossOriginIsolated`, then waits for exactly this console line:

```text
CAPTURE_READY scenario=browser-orbit scene=res://browser/orbit.tscn frame=0
```

Both web presets select a [custom HTML shell](https://docs.godotengine.org/en/4.6/tutorials/platform/web/customizing_html5_shell.html). Only `?capture-browser=1` launches the inherited `browser/orbit.tscn`, which adds a small GDScript probe to the existing lesson scene. Normal launches keep the authored main scene. The shim requires six ready, visible sprites with textures and the registered `BevyApp` class for three consecutive drawn process passes, then forwards READY to `console.log` through `JavaScriptBridge`.

The shim disables physics interpolation and holds the scene after that draw. The driver captures the canvas, calls `window.browserCapture.advance()`, and captures it again when the shim holds after exactly 60 further Godot process passes. Paused redraws do not advance the counter. Screenshot calls do not determine the frame interval. Frame 0 is a stable browser sampling origin; this probe does not run the native reset/clock/FACTS/ACK transport or claim its deterministic rendering guarantees.

The driver uses [Playwright canvas element screenshots](https://playwright.dev/docs/screenshots#element-screenshot), at a 960×640 viewport and device scale 1. It invokes `verdict.py` to decode those PNGs and measure the fraction of pixels whose RGB difference exceeds 8 in at least one channel. The fraction must be strictly above 0.001 by default (more than 0.1% of pixels); `--threshold` changes that diagnostic budget. Alpha is ignored. Dimensions must match. The stdlib decoder accepts non-interlaced RGB8/RGBA8 screenshots and rejects malformed or unsupported input.

This temporal motion diagnostic compares two samples from the same run. It has no golden image, does not change the native structural rendering oracle and never sets `rendering_2d` or `browser` to pass. The threshold remains provisional until verified with actual exports and deliberate stopped-motion failures.

Files stay under `<leaf>/browser/`: `console.log`, `responses.json`, `frame-000000.png`, `frame-000060.png`, `motion.json` and `probe.json`. Failures retain whatever exists. `probe.json` lists the required artifacts, observed environment, diagnostic checks and errors. Exit 0 means those diagnostics completed; its top-level `browser` remains `untested`. No root-level core verdict, facts or transport file is written.

## Manual path when Playwright or Chromium automation is unavailable

1. Serve each export as above. Open its URL with `?capture-browser=1` in Chromium. Use a fresh browser context, with no service worker, and a 960×640 viewport at device scale 1.
2. Save the console log and network headers. Confirm the exact READY line above, the preset's header mode, `crossOriginIsolated`, and `window.browserCapture.threaded` (`false` for port 8060, `true` for port 8061).
3. Confirm `window.browserCapture.ready === true`, `.paused === true`, `.frame === 0` and `.scene === "res://browser/orbit.tscn"`. In DevTools' Elements panel select `#canvas` and use **Capture node screenshot**. Save it as `frame-000000.png` under a fresh evidence leaf's `browser/` directory.
4. In the console call `window.browserCapture.advance()`. Wait until `.paused === true` and `.frame === 60`; capture the same canvas as `frame-000060.png`. Record these states and the Chromium/Godot versions in that directory. A missing READY or checkpoint is incomplete evidence.
5. Measure the screenshots with the same function used by automation:

```bash
python3 itest/capture/browser/verdict.py /absolute/leaf/browser/frame-000000.png /absolute/leaf/browser/frame-000060.png > /absolute/leaf/browser/motion.json
```

That command returns 0 only when the pixel threshold is exceeded, 1 for insufficient motion or invalid evidence, and always emits `"browser":"untested"`. Repeat for the other export. Keep both records for review; manual observations do not re-enable CI or grant browser qualification.
