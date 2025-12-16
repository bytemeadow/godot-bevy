# Web Demo

This example demonstrates **godot-bevy running in a web browser** via WebAssembly. It shows rotating sprites controlled by Bevy's ECS while rendered by Godot.

> **Note:** Web support in godot-rust is experimental. See the [godot-rust web export documentation](https://godot-rust.github.io/book/toolchain/export-web.html) for the latest information.

## Prerequisites

### Using devenv (Recommended)

If you're using the project's devenv setup, everything is pre-configured:

```bash
# From the project root
devenv shell

# Build for web
cd examples/web-demo
./build-web.sh release
```

### Manual Setup

#### 1. Rust Nightly Toolchain

```bash
rustup toolchain install nightly
rustup component add rust-src --toolchain nightly
```

#### 2. Emscripten SDK (3.1.73 for Godot 4.3+)

```bash
# Clone emsdk
git clone https://github.com/emscripten-core/emsdk.git
cd emsdk

# Install and activate (3.1.73 is compatible with godot-rust)
./emsdk install 3.1.73
./emsdk activate 3.1.73

# Add to your shell (run this before building)
source ./emsdk_env.sh
```

> **Note:** Emscripten 4.x has compatibility issues with godot-rust. Use 3.1.73.

### 3. Godot 4.3+

GDExtension web support requires Godot 4.3 or later. Download web export templates from the Godot editor:

1. Open Godot
2. Go to **Editor > Manage Export Templates...**
3. Download templates for your Godot version

## Building

### Native Build (for testing)

```bash
# From this directory
cd rust
cargo build

# Run with Godot
cargo run
```

### Web Build

```bash
# Make sure Emscripten is in your PATH
source /path/to/emsdk/emsdk_env.sh

# Build for web (creates both threaded and non-threaded WASM)
./build-web.sh release
```

This creates:
- `rust/target/wasm32-unknown-emscripten/release/rust_web_demo.threads.wasm` (threaded)
- `rust/target/wasm32-unknown-emscripten/release/rust_web_demo.wasm` (non-threaded)

## Exporting from Godot

1. Open the Godot project: `godot/project.godot`
2. Go to **Project > Export...**
3. Select the **Web** preset (already configured)
4. Ensure **Extensions Support** is enabled
5. For threaded builds, enable **Thread Support**
6. Click **Export Project** and choose an output directory

## Running in Browser

### Local Testing

The exported files need to be served with proper HTTP headers. Godot 4.1.3+ includes a built-in web server:

1. In Godot, click the **Remote Debug** button (globe icon) in the top-right
2. Select **Run in Browser**

Or use the included server script:

```bash
# From the export directory
python3 /path/to/examples/web-demo/serve.py 8000 ./export

# Then open http://localhost:8000 in your browser
```

### Browser Requirements

| Browser | Support | Notes |
|---------|---------|-------|
| Chrome/Edge 113+ | Full | WebGPU available |
| Firefox 117+ | With Godot 4.3+ | Requires cross-origin isolation |
| Safari | Limited | WebGPU not yet supported |

### Cross-Origin Isolation

**Threaded builds** require Cross-Origin Isolation headers for `SharedArrayBuffer` support:

```
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp
```

Without these headers, use the **non-threaded build** (disable Thread Support in Godot export).

## Known Limitations

1. **No gamepad input via gilrs** - Use Godot's `Input` singleton instead
2. **No bevy_log** - Use `godot_print!()` or browser console for logging
3. **Large file sizes** - WASM binaries can be 10-30MB
4. **Experimental** - godot-rust web support is still experimental
5. **Firefox quirks** - Some GDExtension features may be limited

## Troubleshooting

### "Emscripten (emcc) not found"

Make sure you've sourced the Emscripten environment:

```bash
source /path/to/emsdk/emsdk_env.sh
```

### Build fails with tracing-wasm errors

Ensure you're building with the `web` or `web-nothreads` feature, which disables `bevy_log`:

```bash
cargo +nightly build --features web -Zbuild-std --target wasm32-unknown-emscripten
```

### "SharedArrayBuffer is not defined"

Your server isn't sending Cross-Origin Isolation headers. Either:
- Use Godot's built-in web server (Remote Debug > Run in Browser)
- Add the headers to your web server
- Use the non-threaded build

### Blank screen in Firefox

Firefox requires Godot 4.3+ for GDExtension support on web. Check your Godot version.

## Architecture Notes

This example uses several godot-bevy features:

- **GodotTransformSyncPlugin** - Synchronizes Bevy Transform components with Godot node positions
- **Sprite2DMarker** - Automatically marks Sprite2D nodes as Bevy entities
- **GodotNodeHandle** - Provides access to Godot nodes from Bevy systems

The web build works by:
1. Compiling Rust to WebAssembly using the `wasm32-unknown-emscripten` target
2. Loading the WASM as a Godot GDExtension
3. Running Bevy's ECS within Godot's frame loop

## References

- [godot-rust Web Export Guide](https://godot-rust.github.io/book/toolchain/export-web.html)
- [Godot Web Export Documentation](https://docs.godotengine.org/en/stable/tutorials/export/exporting_for_web.html)
- [Bevy WASM Guide](https://bevy-cheatbook.github.io/platforms/wasm.html)
