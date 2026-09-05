#!/bin/bash
set -euo pipefail

CYAN='\033[0;36m'
RED='\033[0;31m'
NC='\033[0m'

fail_config() {
    echo -e "${RED}Error: $*${NC}" >&2
    exit 2
}

require_value() {
    if [[ $# -lt 2 || ${2:-} == --* ]]; then
        fail_config "$1 requires a value"
    fi
}

is_positive_u32() {
    local value=$1
    [[ $value =~ ^[0-9]+$ ]] || return 1

    while [[ ${#value} -gt 1 && ${value:0:1} == 0 ]]; do
        value=${value:1}
    done

    [[ $value != 0 ]] || return 1
    if [[ ${#value} -gt 10 ]]; then
        return 1
    fi
    if [[ ${#value} -eq 10 && $value > 4294967295 ]]; then
        return 1
    fi
    return 0
}

CALLER_DIR=$PWD
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
GODOT_PROJECT_DIR="$SCRIPT_DIR/godot"
BUILD_TYPE=debug
RELEASE_FLAG=()
HARNESS_FEATURE=

REPORT_PATH=${ITEST_JSON_PATH:-}
REPORT_REQUESTED=false
if [[ ${ITEST_JSON_PATH+x} ]]; then
    REPORT_REQUESTED=true
fi

ARGS=("$@")
INDEX=0
while [[ $INDEX -lt ${#ARGS[@]} ]]; do
    NEXT_INDEX=$((INDEX + 1))
    case ${ARGS[$INDEX]} in
        --filter|--repeat|--timeout-frames)
            if [[ $NEXT_INDEX -lt ${#ARGS[@]} ]] && [[ ${ARGS[$NEXT_INDEX]} != --* ]]; then
                INDEX=$((INDEX + 2))
            else
                INDEX=$((INDEX + 1))
            fi
            ;;
        --json)
            if [[ $NEXT_INDEX -lt ${#ARGS[@]} ]] && [[ ${ARGS[$NEXT_INDEX]} != --* ]]; then
                REPORT_PATH=${ARGS[$NEXT_INDEX]}
                REPORT_REQUESTED=true
                INDEX=$((INDEX + 2))
            else
                INDEX=$((INDEX + 1))
            fi
            ;;
        *)
            INDEX=$((INDEX + 1))
            ;;
    esac
done

if [[ $REPORT_REQUESTED == true ]]; then
    [[ -n $REPORT_PATH ]] || fail_config "ITEST_JSON_PATH must not be empty"
    if [[ $REPORT_PATH != /* ]]; then
        REPORT_PATH="$CALLER_DIR/$REPORT_PATH"
    fi
    export ITEST_JSON_PATH=$REPORT_PATH
    rm -f -- "$ITEST_JSON_PATH" || fail_config "could not remove stale report: $ITEST_JSON_PATH"
fi

while [[ $# -gt 0 ]]; do
    case $1 in
        --release)
            BUILD_TYPE=release
            RELEASE_FLAG=(--release)
            shift
            ;;
        --filter)
            require_value "$@"
            export ITEST_FILTER=$2
            shift 2
            ;;
        --repeat)
            require_value "$@"
            is_positive_u32 "$2" || fail_config "--repeat must be a positive 32-bit integer"
            export ITEST_REPEAT=$2
            shift 2
            ;;
        --timeout-frames)
            require_value "$@"
            is_positive_u32 "$2" || fail_config "--timeout-frames must be a positive 32-bit integer"
            export ITEST_TIMEOUT_FRAMES=$2
            shift 2
            ;;
        --json)
            require_value "$@"
            shift 2
            ;;
        --harness-probes)
            HARNESS_FEATURE=harness-probes
            shift
            ;;
        --harness-focus-probe)
            HARNESS_FEATURE=harness-focus-probe
            shift
            ;;
        *)
            fail_config "unknown argument: $1"
            ;;
    esac
done

if [[ ${ITEST_REPEAT+x} ]]; then
    is_positive_u32 "$ITEST_REPEAT" || fail_config "ITEST_REPEAT must be a positive 32-bit integer"
else
    export ITEST_REPEAT=1
fi

if [[ ${ITEST_TIMEOUT_FRAMES+x} ]]; then
    is_positive_u32 "$ITEST_TIMEOUT_FRAMES" || fail_config "ITEST_TIMEOUT_FRAMES must be a positive 32-bit integer"
else
    export ITEST_TIMEOUT_FRAMES=600
fi

export ITEST_BUILD_PROFILE=$BUILD_TYPE

FEATURES=test-frame-signal,autosync-tests
if [[ -n $HARNESS_FEATURE ]]; then
    FEATURES="$FEATURES,$HARNESS_FEATURE"
fi

echo -e "${CYAN}Building godot-bevy-itest ($BUILD_TYPE)...${NC}"
cd "$SCRIPT_DIR/rust"
cargo build ${RELEASE_FLAG[@]+"${RELEASE_FLAG[@]}"} --features "$FEATURES"
cd "$SCRIPT_DIR"

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

if [[ -z ${GODOT4_BIN:-} ]]; then
    if command -v godot4 &> /dev/null; then
        GODOT4_BIN=godot4
    elif command -v godot &> /dev/null; then
        GODOT4_BIN=godot
    elif [[ -f /Applications/Godot.app/Contents/MacOS/Godot ]]; then
        GODOT4_BIN=/Applications/Godot.app/Contents/MacOS/Godot
    elif [[ -f "${HOME:-}/Library/Application Support/gdenv/bin/godot" ]]; then
        GODOT4_BIN="${HOME:-}/Library/Application Support/gdenv/bin/godot"
    else
        fail_config "could not find Godot 4; set GODOT4_BIN"
    fi
fi

echo -e "${CYAN}Using Godot binary: $GODOT4_BIN${NC}"

mkdir -p "$GODOT_PROJECT_DIR/.godot"
echo "res://itest.gdextension" > "$GODOT_PROJECT_DIR/.godot/extension_list.cfg"

echo -e "${CYAN}Importing Godot project...${NC}"
"$GODOT4_BIN" --headless --path "$GODOT_PROJECT_DIR" --import --quit || true

if [[ -n ${GODOT_TEST_EXIT_CODE_PATH:-} ]]; then
    EXIT_CODE_FILE=$GODOT_TEST_EXIT_CODE_PATH
    if [[ $EXIT_CODE_FILE != /* ]]; then
        EXIT_CODE_FILE="$CALLER_DIR/$EXIT_CODE_FILE"
    fi
elif [[ -n ${TMPDIR:-} ]]; then
    EXIT_CODE_FILE="${TMPDIR%/}/godot_test_exit_code_$$"
elif [[ -n ${TEMP:-} ]]; then
    EXIT_CODE_FILE="${TEMP%/}/godot_test_exit_code_$$"
else
    EXIT_CODE_FILE="/tmp/godot_test_exit_code_$$"
fi
export GODOT_TEST_EXIT_CODE_PATH=$EXIT_CODE_FILE
rm -f -- "$EXIT_CODE_FILE" || fail_config "could not remove stale exit file: $EXIT_CODE_FILE"

# --fixed-fps 60 pins Godot's frame delta so physics steps exactly once per render
# frame and the whole schedule is reproducible (deterministic itests). This is
# test-determinism only -- do NOT add it to run-benches.sh/compare-benches.sh, which
# are synchronous and Instant-timed and gain nothing from it.
# --quit-after is a gross watchdog behind the per-attempt ITEST_TIMEOUT_FRAMES limit.
QUIT_AFTER=$((5000 * 10#$ITEST_REPEAT))
echo -e "${CYAN}Running integration tests...${NC}"
set +e
"$GODOT4_BIN" --headless --fixed-fps 60 --path "$GODOT_PROJECT_DIR" --quit-after "$QUIT_AFTER"
GODOT_STATUS=$?
set -e

if [[ -f $EXIT_CODE_FILE ]]; then
    EXIT_CODE=$(<"$EXIT_CODE_FILE")
    rm -f -- "$EXIT_CODE_FILE" || true
    [[ $EXIT_CODE =~ ^[012]$ ]] || EXIT_CODE=2
else
    EXIT_CODE=2
fi

if [[ $GODOT_STATUS -ne 0 ]]; then
    echo -e "${RED}Godot exited with status $GODOT_STATUS${NC}" >&2
    exit "$GODOT_STATUS"
fi

if [[ $EXIT_CODE -ne 0 ]]; then
    echo -e "${RED}Tests exited with status $EXIT_CODE${NC}" >&2
fi
exit "$EXIT_CODE"
