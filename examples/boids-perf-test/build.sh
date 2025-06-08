#!/bin/bash

# Boids Performance Benchmark Build Script
# Builds the Rust extension and prepares the Godot project

set -e

echo "🚀 Building Boids Performance Benchmark"
echo "========================================"

# Check if we're in the right directory
if [ ! -f "rust/Cargo.toml" ]; then
    echo "❌ Error: Must run from the boids-perf-test directory"
    echo "   Expected to find rust/Cargo.toml"
    exit 1
fi

# Check for Rust toolchain
if ! command -v cargo &> /dev/null; then
    echo "❌ Error: Rust toolchain not found"
    echo "   Please install Rust from: https://rustup.rs/"
    exit 1
fi

# Check for Godot (optional, for auto-run)
GODOT_BINARY=""
if command -v godot &> /dev/null; then
    GODOT_BINARY="godot"
elif command -v godot4 &> /dev/null; then
    GODOT_BINARY="godot4"
elif [ -f "/Applications/Godot.app/Contents/MacOS/Godot" ]; then
    GODOT_BINARY="/Applications/Godot.app/Contents/MacOS/Godot"
elif [ -f "/usr/local/bin/godot" ]; then
    GODOT_BINARY="/usr/local/bin/godot"
fi

echo "📦 Building Rust extension..."
cd rust

# Build in release mode for accurate performance testing
echo "   Building in release mode for optimal performance..."
cargo build --release

if [ $? -ne 0 ]; then
    echo "❌ Rust build failed"
    exit 1
fi

echo "✅ Rust extension built successfully"

cd ..

# Verify the built library exists
if [ "$(uname)" == "Darwin" ]; then
    LIB_PATH="rust/target/release/libboids_benchmark.dylib"
elif [ "$(expr substr $(uname -s) 1 5)" == "Linux" ]; then
    LIB_PATH="rust/target/release/libboids_benchmark.so"
else
    LIB_PATH="rust/target/release/boids_benchmark.dll"
fi

if [ ! -f "$LIB_PATH" ]; then
    echo "❌ Built library not found at: $LIB_PATH"
    exit 1
fi

echo "📋 Verifying Godot project structure..."

# Check that required files exist
REQUIRED_FILES=(
    "godot/project.godot"
    "godot/rust.gdextension"
    "godot/scenes/main.tscn"
    "godot/scenes/bevy_app_singleton.tscn"
    "godot/scripts/main.gd"
    "godot/scripts/godot_boids.gd"
)

for file in "${REQUIRED_FILES[@]}"; do
    if [ ! -f "$file" ]; then
        echo "❌ Missing required file: $file"
        exit 1
    fi
done

echo "✅ All required files found"

echo ""
echo "🎉 Build completed successfully!"
echo ""
echo "📊 Performance Benchmark Ready"
echo ""
echo "To run the benchmark:"
echo ""
if [ -n "$GODOT_BINARY" ]; then
    echo "1. Automatic (using detected Godot):"
    echo "   $GODOT_BINARY --path godot"
    echo ""
fi
echo "2. Manual:"
echo "   - Open 'godot/project.godot' in Godot Editor"
echo "   - Run the main scene"
echo ""
echo "3. Command line (if supported):"
echo "   cargo run"
echo ""
echo "📈 Benchmark Features:"
echo "   ✓ Switch between Godot (GDScript) and godot-bevy (Rust + ECS)"
echo "   ✓ Adjust boid count from 50 to 2000+"
echo "   ✓ Real-time FPS monitoring and comparison"
echo "   ✓ Performance metrics tracking"
echo ""
echo "🎯 Expected Results:"
echo "   - Similar performance with < 500 boids"
echo "   - 2-5x better performance with godot-bevy at 1000+ boids"
echo "   - Rust's compiled nature shines with computational workloads"
echo ""

# Option to auto-run if Godot is available
if [ -n "$GODOT_BINARY" ] && [ "$1" == "--run" ]; then
    echo "🚀 Auto-running benchmark..."
    cd godot
    exec "$GODOT_BINARY" --path .
fi