#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

if [[ $# -ne 1 ]]; then
    echo "usage: $0 schemas|tools|contract|tracy-live|fail-closed" >&2
    exit 2
fi

case $1 in
    schemas)
        exec python3 "$SCRIPT_DIR/test_profiling.py" schemas
        ;;
    tools)
        exec python3 "$SCRIPT_DIR/test_profiling.py" tools
        ;;
    contract)
        exec python3 "$SCRIPT_DIR/test_profiling.py" contract
        ;;
    tracy-live)
        log="$(mktemp "${TMPDIR:-/tmp}/godot-bevy-profile-live.XXXXXX")"
        trap 'rm -f "$log"' EXIT
        "$SCRIPT_DIR/run-profile.sh" --bench transform_sync_bevy_to_godot_3d | tee "$log"
        spans_path="$(sed -n 's/^Profile complete: //p' "$log" | tail -1)"
        if [[ -z $spans_path ]]; then
            echo "profile completion path missing" >&2
            exit 2
        fi
        if [[ $spans_path != /* ]]; then
            spans_path="$SCRIPT_DIR/../$spans_path"
        fi
        python3 "$SCRIPT_DIR/test_profiling.py" live "$spans_path"
        ;;
    fail-closed)
        exec python3 "$SCRIPT_DIR/test_profiling.py" fail-closed
        ;;
    *)
        echo "usage: $0 schemas|tools|contract|tracy-live|fail-closed" >&2
        exit 2
        ;;
esac
