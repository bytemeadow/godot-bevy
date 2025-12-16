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
#   ./build-web.sh           # Build both versions (release)
#   ./build-web.sh debug     # Build both versions (debug)
#   ./build-web.sh --serve   # Build and serve non-threaded locally
#   ./build-web.sh --serve --threaded  # Build and serve threaded version

set -e

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
RUST_DIR="$SCRIPT_DIR/rust"

# Parse arguments
BUILD_MODE="release"
SERVE=false
THREADED=false
for arg in "$@"; do
    case $arg in
        debug)
            BUILD_MODE="debug"
            ;;
        release)
            BUILD_MODE="release"
            ;;
        --serve)
            SERVE=true
            ;;
        --threaded)
            THREADED=true
            ;;
    esac
done

RELEASE_FLAG=""
if [[ "$BUILD_MODE" == "release" ]]; then
    RELEASE_FLAG="--release"
fi

# Determine which cargo to use
if [[ -n "$CARGO_NIGHTLY" ]]; then
    CARGO_CMD="$CARGO_NIGHTLY"
    echo "Using nightly cargo from devenv: $CARGO_CMD"
    unset RUSTC_WRAPPER
    echo "Disabled RUSTC_WRAPPER (sccache) for nightly build"
    if [[ -n "$RUSTC_NIGHTLY" ]]; then
        export RUSTC="$RUSTC_NIGHTLY"
        echo "Using nightly rustc: $RUSTC"
    fi
else
    CARGO_CMD="cargo +nightly"
    echo "Using rustup nightly: $CARGO_CMD"
fi

echo "=== Building Simple Node2D Movement for web ($BUILD_MODE) ==="
echo ""

cd "$RUST_DIR"

# Check for Emscripten
if ! command -v emcc &> /dev/null; then
    echo "ERROR: Emscripten (emcc) not found in PATH"
    echo ""
    echo "If using devenv, run: devenv shell"
    exit 1
fi

EMCC_VERSION=$(emcc --version | head -n1)
echo "Using Emscripten: $EMCC_VERSION"
echo ""

# Set writable Emscripten cache directory
if [[ -z "$EM_CACHE" ]]; then
    export EM_CACHE="$SCRIPT_DIR/.em_cache"
    mkdir -p "$EM_CACHE"
    echo "Using Emscripten cache: $EM_CACHE"
    echo ""
fi

# Set up bindgen to use Emscripten's sysroot
if [[ -n "$EMSDK" ]]; then
    EMSCRIPTEN_SYSROOT="$EMSDK/upstream/emscripten/cache/sysroot"
    if [[ -d "$EMSCRIPTEN_SYSROOT" ]]; then
        export BINDGEN_EXTRA_CLANG_ARGS="--sysroot=$EMSCRIPTEN_SYSROOT"
        echo "Using Emscripten sysroot for bindgen: $EMSCRIPTEN_SYSROOT"
        echo ""
    fi
fi

# Generate extension_api.json if it doesn't exist
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

# Workspace target directory (use absolute path since we cd later)
# SCRIPT_DIR is examples/simple-node2d-movement, so go up 2 levels to workspace root
WORKSPACE_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
WASM_DIR="$WORKSPACE_ROOT/target/wasm32-unknown-emscripten/$BUILD_MODE"

# Build non-threaded version (default, broader compatibility)
echo "=== Building NON-THREADED version ==="
RUSTFLAGS="-C link-args=-sSIDE_MODULE=2 -C link-args=-O0 -Zlink-native-libraries=no -Cllvm-args=-enable-emscripten-cxx-exceptions=0" \
    $CARGO_CMD build --no-default-features --features web-nothreads -Zbuild-std --target wasm32-unknown-emscripten $RELEASE_FLAG

if [[ -f "$WASM_DIR/rust_simple_node2d_movement.wasm" ]]; then
    echo "Created: $WASM_DIR/rust_simple_node2d_movement.wasm"
else
    echo "ERROR: Non-threaded build failed"
    exit 1
fi

# Only build threaded version if requested or doing a full build
if [[ "$THREADED" == true ]] || [[ "$SERVE" == false ]]; then
    echo ""
    echo "=== Building THREADED version ==="

    # Use a separate target directory for threaded builds to avoid cargo caching issues
    # (RUSTFLAGS changes don't trigger rebuilds with the same target dir)
    THREADED_TARGET_DIR="$WORKSPACE_ROOT/target-threaded"
    THREADED_WASM_DIR="$THREADED_TARGET_DIR/wasm32-unknown-emscripten/$BUILD_MODE"

    CARGO_TARGET_DIR="$THREADED_TARGET_DIR" \
    RUSTFLAGS="-C link-args=-pthread -C target-feature=+atomics -C link-args=-sSIDE_MODULE=2 -C link-args=-O0 -Zlink-native-libraries=no -Cllvm-args=-enable-emscripten-cxx-exceptions=0" \
        $CARGO_CMD build --no-default-features --features web -Zbuild-std --target wasm32-unknown-emscripten $RELEASE_FLAG

    # Copy to main target dir with .threads.wasm suffix
    if [[ -f "$THREADED_WASM_DIR/rust_simple_node2d_movement.wasm" ]]; then
        cp "$THREADED_WASM_DIR/rust_simple_node2d_movement.wasm" "$WASM_DIR/rust_simple_node2d_movement.threads.wasm"
        echo "Created: $WASM_DIR/rust_simple_node2d_movement.threads.wasm"
    else
        echo "WARNING: Threaded build failed - .wasm not found in $THREADED_WASM_DIR"
    fi
fi

echo ""
echo "=== Web build complete! ==="
echo ""
echo "Output files:"
echo "  Threaded:     $WASM_DIR/rust_simple_node2d_movement.threads.wasm"
echo "  Non-threaded: $WASM_DIR/rust_simple_node2d_movement.wasm"

if [[ "$SERVE" == true ]]; then
    echo ""
    echo "=== Exporting and serving ==="
    cd "$SCRIPT_DIR/godot"

    EXPORT_DIR="$SCRIPT_DIR/web-export"
    mkdir -p "$EXPORT_DIR"

    if [[ "$BUILD_MODE" == "release" ]]; then
        EXPORT_FLAG="--export-release"
    else
        EXPORT_FLAG="--export-debug"
    fi

    # Select preset based on threading mode
    if [[ "$THREADED" == true ]]; then
        PRESET="Web Threaded"
    else
        PRESET="Web"
    fi

    echo "Exporting with: godot --headless $EXPORT_FLAG $PRESET"
    godot --headless $EXPORT_FLAG "$PRESET" "$EXPORT_DIR/index.html" 2>&1 || {
        echo "Note: Export may have warnings, checking if files exist..."
    }

    if [[ -f "$EXPORT_DIR/index.html" ]]; then
        echo ""
        echo "Starting server with COOP/COEP headers at http://localhost:8060"
        echo "Press Ctrl+C to stop"
        cd "$EXPORT_DIR"
        python3 "$SCRIPT_DIR/../web-demo/serve.py" 8060
    else
        echo "Export failed - index.html not found"
        exit 1
    fi
fi
