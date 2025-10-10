#!/bin/bash
set -e

# Run godot-bevy benchmarks

# Color codes
CYAN='\033[0;36m'
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
NC='\033[0m' # No Color

echo -e "${CYAN}Building godot-bevy-itest...${NC}"

# Build the Rust library in release mode for accurate benchmarks
cd "$(dirname "$0")/rust"
cargo build --release

cd ..

# Check for GODOT4_BIN environment variable
if [ -z "$GODOT4_BIN" ]; then
    # Try common locations for Godot binary
    if command -v godot4 &> /dev/null; then
        GODOT4_BIN="godot4"
    elif command -v godot &> /dev/null; then
        GODOT4_BIN="godot"
    elif [ -f "/Applications/Godot.app/Contents/MacOS/Godot" ]; then
        GODOT4_BIN="/Applications/Godot.app/Contents/MacOS/Godot"
    elif [ -f "$HOME/Library/Application Support/gdenv/bin/godot" ]; then
        GODOT4_BIN="$HOME/Library/Application Support/gdenv/bin/godot"
    else
        echo -e "${RED}Error: Could not find Godot binary${NC}"
        echo "Please set GODOT4_BIN environment variable to point to your Godot 4 executable"
        echo "Example: export GODOT4_BIN=/path/to/godot"
        exit 1
    fi
fi

echo -e "${CYAN}Using Godot binary: $GODOT4_BIN${NC}"

# Check if debug build
if cargo metadata --format-version=1 2>/dev/null | grep -q '"profile":"dev"'; then
    echo -e "${YELLOW}Warning: Running with debug build. Use --release for accurate benchmarks.${NC}"
fi

# Run benchmarks in headless mode with BenchRunner scene
echo -e "${CYAN}Running benchmarks...${NC}"
"$GODOT4_BIN" --headless --path godot BenchRunner.tscn --quit-after 30000

echo -e "${GREEN}Benchmarks complete!${NC}"
