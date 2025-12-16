#!/bin/bash
#
# Build script for web (WASM) exports
#
# Prerequisites (choose one):
#   A) Using devenv (recommended):
#      - Just run `devenv shell` - everything is set up automatically
#      - Use $CARGO_NIGHTLY for the nightly toolchain
#
#   B) Using rustup:
#      - Rust nightly: rustup toolchain install nightly
#      - rust-src: rustup component add rust-src --toolchain nightly
#      - Emscripten SDK 3.1.74+ (for Godot 4.3+)
#
# Usage:
#   ./build-web.sh           # Build both threaded and non-threaded (release)
#   ./build-web.sh debug     # Build both threaded and non-threaded (debug)
#   ./build-web.sh release   # Build both threaded and non-threaded (release)

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RUST_DIR="$SCRIPT_DIR/rust"

# Parse arguments
BUILD_MODE="${1:-release}"
if [[ "$BUILD_MODE" != "debug" && "$BUILD_MODE" != "release" ]]; then
    echo "Usage: $0 [debug|release]"
    exit 1
fi

RELEASE_FLAG=""
if [[ "$BUILD_MODE" == "release" ]]; then
    RELEASE_FLAG="--release"
fi

# Determine which cargo to use
# - If CARGO_NIGHTLY is set (devenv), use it directly
# - Otherwise, try cargo +nightly (rustup)
if [[ -n "$CARGO_NIGHTLY" ]]; then
    CARGO_CMD="$CARGO_NIGHTLY"
    echo "Using nightly cargo from devenv: $CARGO_CMD"
    # Disable sccache for web builds - it interferes with nightly rustc
    unset RUSTC_WRAPPER
    echo "Disabled RUSTC_WRAPPER (sccache) for nightly build"
    # Set RUSTC to nightly rustc (cargo uses RUSTC env var)
    if [[ -n "$RUSTC_NIGHTLY" ]]; then
        export RUSTC="$RUSTC_NIGHTLY"
        echo "Using nightly rustc: $RUSTC"
    fi
else
    CARGO_CMD="cargo +nightly"
    echo "Using rustup nightly: $CARGO_CMD"
fi

echo "=== Building Web Demo for $BUILD_MODE ==="
echo ""

cd "$RUST_DIR"

# Check for Emscripten
if ! command -v emcc &> /dev/null; then
    echo "ERROR: Emscripten (emcc) not found in PATH"
    echo ""
    echo "If using devenv, run: devenv shell"
    echo ""
    echo "Otherwise, install Emscripten SDK:"
    echo "  git clone https://github.com/emscripten-core/emsdk.git"
    echo "  cd emsdk"
    echo "  ./emsdk install 3.1.74"
    echo "  ./emsdk activate 3.1.74"
    echo "  source ./emsdk_env.sh"
    exit 1
fi

EMCC_VERSION=$(emcc --version | head -n1)
echo "Using Emscripten: $EMCC_VERSION"
echo ""

# Set writable Emscripten cache directory (required for Nix-provided emscripten)
# The Nix store is read-only, so we need a writable cache location
if [[ -z "$EM_CACHE" ]]; then
    export EM_CACHE="$SCRIPT_DIR/.em_cache"
    mkdir -p "$EM_CACHE"
    echo "Using Emscripten cache: $EM_CACHE"
    echo ""
fi

# Generate extension_api.json if it doesn't exist (needed for api-custom-json)
API_JSON="$SCRIPT_DIR/../../extension_api.json"
if [[ ! -f "$API_JSON" ]]; then
    echo "Generating extension_api.json from Godot..."
    (cd "$SCRIPT_DIR/../.." && godot --headless --dump-extension-api 2>/dev/null)
fi

if [[ -f "$API_JSON" ]]; then
    export GODOT4_GDEXTENSION_JSON="$API_JSON"
    echo "Using API JSON: $GODOT4_GDEXTENSION_JSON"
else
    echo "WARNING: extension_api.json not found - build may fail"
fi
echo ""

# Build threaded version (for browsers with SharedArrayBuffer support)
echo "=== Building THREADED version ==="
# Note: -O0 skips wasm-opt post-processing to avoid binaryen version mismatches
RUSTFLAGS="-C link-args=-pthread -C target-feature=+atomics -C link-args=-sSIDE_MODULE=2 -C link-args=-O0 -Zlink-native-libraries=no -Cllvm-args=-enable-emscripten-cxx-exceptions=0" \
    $CARGO_CMD build --no-default-features --features web -Zbuild-std --target wasm32-unknown-emscripten $RELEASE_FLAG

# Rename to .threads.wasm
WASM_DIR="target/wasm32-unknown-emscripten/$BUILD_MODE"
if [[ -f "$WASM_DIR/rust_web_demo.wasm" ]]; then
    mv "$WASM_DIR/rust_web_demo.wasm" "$WASM_DIR/rust_web_demo.threads.wasm"
    echo "Created: $WASM_DIR/rust_web_demo.threads.wasm"
fi

echo ""

# Build non-threaded version (broader browser compatibility)
echo "=== Building NON-THREADED version ==="
# Note: -O0 skips wasm-opt post-processing to avoid binaryen version mismatches
RUSTFLAGS="-C link-args=-sSIDE_MODULE=2 -C link-args=-O0 -Zlink-native-libraries=no -Cllvm-args=-enable-emscripten-cxx-exceptions=0" \
    $CARGO_CMD build --no-default-features --features web-nothreads -Zbuild-std --target wasm32-unknown-emscripten $RELEASE_FLAG

echo "Created: $WASM_DIR/rust_web_demo.wasm"

echo ""
echo "=== Web build complete! ==="
echo ""
echo "Output files:"
echo "  Threaded:     $WASM_DIR/rust_web_demo.threads.wasm"
echo "  Non-threaded: $WASM_DIR/rust_web_demo.wasm"
echo ""
echo "Next steps:"
echo "  1. Open Godot and load the project from: $SCRIPT_DIR/godot/"
echo "  2. Go to Project > Export..."
echo "  3. Select the 'Web' preset"
echo "  4. Click 'Export Project' and choose an output directory"
echo "  5. Serve the exported files with a web server that sets proper headers"
echo ""
echo "For threaded builds, you need Cross-Origin Isolation headers:"
echo "  Cross-Origin-Opener-Policy: same-origin"
echo "  Cross-Origin-Embedder-Policy: require-corp"
