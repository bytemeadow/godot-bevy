#!/bin/bash
set -e

# Run godot-bevy integration tests
# Based on gdext's run-itest.sh

# Color codes
CYAN='\033[0;36m'
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

# Parse arguments
BUILD_TYPE="debug"
CARGO_BUILD_FLAGS=""

while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            BUILD_TYPE="release"
            CARGO_BUILD_FLAGS="--release"
            shift
            ;;
        *)
            shift
            ;;
    esac
done

echo -e "${CYAN}Building godot-bevy-itest ($BUILD_TYPE)...${NC}"

# Build the Rust library
cd "$(dirname "$0")/rust"
cargo build $CARGO_BUILD_FLAGS

cd ..

# Generate .gdextension file with correct library paths for the build type
cat > godot/itest.gdextension << EOF
[configuration]
entry_symbol = "godot_bevy_itest"
compatibility_minimum = 4.2

[libraries]
linux.debug.x86_64 = "res://../../target/${BUILD_TYPE}/libgodot_bevy_itest.so"
linux.release.x86_64 = "res://../../target/${BUILD_TYPE}/libgodot_bevy_itest.so"
windows.debug.x86_64 = "res://../../target/${BUILD_TYPE}/godot_bevy_itest.dll"
windows.release.x86_64 = "res://../../target/${BUILD_TYPE}/godot_bevy_itest.dll"
macos.debug = "res://../../target/${BUILD_TYPE}/libgodot_bevy_itest.dylib"
macos.release = "res://../../target/${BUILD_TYPE}/libgodot_bevy_itest.dylib"
macos.debug.arm64 = "res://../../target/${BUILD_TYPE}/libgodot_bevy_itest.dylib"
macos.release.arm64 = "res://../../target/${BUILD_TYPE}/libgodot_bevy_itest.dylib"
EOF

echo -e "${CYAN}Generated itest.gdextension for ${BUILD_TYPE} build${NC}"

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

# Cross-platform temp directory and exit code file
# Use GODOT_TEST_EXIT_CODE_PATH if set, otherwise use system temp dir
if [ -z "$GODOT_TEST_EXIT_CODE_PATH" ]; then
    if [ -n "$TMPDIR" ]; then
        EXIT_CODE_FILE="$TMPDIR/godot_test_exit_code"
    elif [ -n "$TEMP" ]; then
        # Windows compatibility
        EXIT_CODE_FILE="$TEMP/godot_test_exit_code"
    else
        EXIT_CODE_FILE="/tmp/godot_test_exit_code"
    fi
else
    EXIT_CODE_FILE="$GODOT_TEST_EXIT_CODE_PATH"
fi

# Export for Rust code to use the same path
export GODOT_TEST_EXIT_CODE_PATH="$EXIT_CODE_FILE"

# Clean up any previous exit code file
rm -f "$EXIT_CODE_FILE"

# Run tests in headless mode
echo -e "${CYAN}Running integration tests...${NC}"
"$GODOT4_BIN" --headless --path godot --quit-after 5000

# Read the exit code from the file written by tests
if [ -f "$EXIT_CODE_FILE" ]; then
    EXIT_CODE=$(cat "$EXIT_CODE_FILE")
    rm -f "$EXIT_CODE_FILE"
else
    # If no file exists, assume failure
    EXIT_CODE=1
fi

if [ $EXIT_CODE -eq 0 ]; then
    # Success message already printed by tests
    true
else
    echo -e "${RED}Tests failed with exit code $EXIT_CODE${NC}"
fi

exit $EXIT_CODE
