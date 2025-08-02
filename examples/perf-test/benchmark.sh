#!/usr/bin/env bash

# Boids Performance Benchmark Runner
# This script runs automated benchmarks comparing Godot and Bevy implementations

# bash strict mode, http://redsymbol.net/articles/unofficial-bash-strict-mode/
set -euo pipefail
IFS=$'\n\t'

# Default values
GODOT_EXECUTABLE="godot"
IMPLEMENTATION="godot"
BOID_COUNT=1000
DURATION=10
OUTPUT_DIR="$(readlink -f .)/benchmark_results"
TIMESTAMP=$(date +"%Y%m%d_%H%M%S")

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --godot)
            GODOT_EXECUTABLE="$2"
            shift 2
            ;;
        --implementation|-i)
            IMPLEMENTATION="$2"
            shift 2
            ;;
        --boids|-b)
            BOID_COUNT="$2"
            shift 2
            ;;
        --duration|-d)
            DURATION="$2"
            shift 2
            ;;
        --output|-o)
            OUTPUT_DIR="$2"
            shift 2
            ;;
        --help|-h)
            echo "Boids Performance Benchmark Runner"
            echo ""
            echo "Usage: $0 [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --godot PATH          Path to Godot executable (default: godot)"
            echo "  -i, --implementation  Implementation to test: godot or bevy (default: godot)"
            echo "  -b, --boids          Number of boids (default: 1000)"
            echo "  -d, --duration       Test duration in seconds (default: 10)"
            echo "  -o, --output         Output directory for results (default: benchmark_results)"
            echo "  -h, --help           Show this help message"
            echo ""
            echo "Examples:"
            echo "  # Test Godot implementation with 5000 boids"
            echo "  $0 -i godot -b 5000"
            echo ""
            echo "  # Test Bevy implementation for 30 seconds"
            echo "  $0 -i bevy -d 30"
            echo ""
            echo "  # Run both implementations and compare"
            echo "  $0 -i godot -b 2000 && $0 -i bevy -b 2000"
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            echo "Use --help for usage information"
            exit 1
            ;;
    esac
done

# Create output directory if it doesn't exist
mkdir -p "$OUTPUT_DIR"

# Construct output filename
OUTPUT_FILE="$OUTPUT_DIR/benchmark_${IMPLEMENTATION}_${BOID_COUNT}boids_${TIMESTAMP}.json"

echo "🎮 Boids Performance Benchmark"
echo "==============================="
echo "Implementation: $IMPLEMENTATION"
echo "Boid Count: $BOID_COUNT"
echo "Duration: $DURATION seconds"
echo "Output: $OUTPUT_FILE"
echo ""

# First ensure the Rust library is built
echo "🔨 Building Rust library..."
cd rust
# for unknown reasons, both the debug and release dynamic libraries must be present;
# otherwise, when we attempt to run the exported godot binary, we'll see spurious
# warning messages despite the benchmark actually running fine; hence, we just
# build both:
cargo build
cargo build --release
cd ..

echo "🔨 Exporting a godot release build..."
EXPORT_DIR="$OUTPUT_DIR/export"
BENCHMARK_BINARY="$EXPORT_DIR/boids"
mkdir -p "$EXPORT_DIR"
cd godot
PLATFORM=""
if [ "$(uname)" == "Linux" ]; then
  PLATFORM="Linux/X11"
elif [ "$(uname)" == "Darwin" ]; then
  PLATFORM="macOS"
else
  PLATFORM="Windows Desktop"
fi
godot --headless --export-release "$PLATFORM" "$BENCHMARK_BINARY" project.godot
cd ..

# If we're using a nix development environment, wrap executable in a file hierarchy standard (FHS) env
[[ $(type -P "steam-run") ]] && FHS_BINARY="steam-run" || FHS_BINARY=""

# Run the benchmark
echo "🚀 Starting benchmark..."
$FHS_BINARY "$BENCHMARK_BINARY" --headless \
    --implementation="$IMPLEMENTATION" \
    --boid-count="$BOID_COUNT" \
    --duration="$DURATION" \
    --output="$OUTPUT_FILE"

echo ""
echo "✅ Benchmark complete!"
echo "Results saved to: $OUTPUT_FILE"

# Display the results
if [ -f "$OUTPUT_FILE" ]; then
    echo ""
    echo "📊 Results:"
    jq '.' "$OUTPUT_FILE"
fi
