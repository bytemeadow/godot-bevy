#!/bin/bash
set -e

# Run godot-bevy integration tests
# Based on gdext's run-itest.sh

# Color codes
CYAN='\033[0;36m'
GREEN='\033[0;32m'
RED='\033[0;31m'
NC='\033[0m' # No Color

echo -e "${CYAN}Building godot-bevy-itest...${NC}"

# Build the Rust library
cd "$(dirname "$0")/rust"
cargo build

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

# Clean up any previous exit code file
rm -f /tmp/godot_test_exit_code

# Run tests in headless mode
echo -e "${CYAN}Running integration tests...${NC}"
"$GODOT4_BIN" --headless --path godot --quit-after 5000

# Read the exit code from the file written by tests
if [ -f /tmp/godot_test_exit_code ]; then
    EXIT_CODE=$(cat /tmp/godot_test_exit_code)
    rm -f /tmp/godot_test_exit_code
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
