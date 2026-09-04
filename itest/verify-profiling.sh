#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

if [[ $# -ne 1 ]]; then
    echo "usage: $0 schemas|fixtures|tools|contract|tracy-live|fail-closed|compare|compare-live|native-live|workflow" >&2
    exit 2
fi

case $1 in
    schemas)
        exec python3 "$SCRIPT_DIR/test_profiling.py" schemas
        ;;
    fixtures)
        exec python3 "$SCRIPT_DIR/test_profiling.py" fixtures
        ;;
    tools)
        exec python3 "$SCRIPT_DIR/test_profiling.py" tools
        ;;
    contract)
        exec python3 "$SCRIPT_DIR/test_profiling.py" contract
        ;;
    compare)
        exec python3 "$SCRIPT_DIR/test_profiling.py" compare
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
    compare-live)
        log="$(mktemp "${TMPDIR:-/tmp}/godot-bevy-profile-compare.XXXXXX")"
        trap 'rm -f "$log"' EXIT
        "$SCRIPT_DIR/compare-profiles.sh" --self --bench transform_sync_bevy_to_godot_3d | tee "$log"
        comparison_path="$(sed -n 's/^Comparison complete: //p' "$log" | tail -1)"
        if [[ -z $comparison_path ]]; then
            echo "comparison completion path missing" >&2
            exit 2
        fi
        if [[ $comparison_path != /* ]]; then
            comparison_path="$SCRIPT_DIR/../$comparison_path"
        fi
        python3 "$SCRIPT_DIR/test_profiling.py" compare-live "$comparison_path"
        ;;
    native-live)
        log="$(mktemp "${TMPDIR:-/tmp}/godot-bevy-profile-native.XXXXXX")"
        trap 'rm -f "$log"' EXIT
        "$SCRIPT_DIR/run-profile.sh" --native --bench transform_sync_bevy_to_godot_3d | tee "$log"
        summary_path="$(sed -n 's/^Native profile complete: //p' "$log" | tail -1)"
        if [[ -z $summary_path ]]; then
            echo "native profile completion path missing" >&2
            exit 2
        fi
        if [[ $summary_path != /* ]]; then
            summary_path="$SCRIPT_DIR/../$summary_path"
        fi
        python3 "$SCRIPT_DIR/test_profiling.py" native-live "$summary_path"
        ;;
    workflow)
        exec python3 "$SCRIPT_DIR/test_profiling.py" workflow
        ;;
    fail-closed)
        exec python3 "$SCRIPT_DIR/test_profiling.py" fail-closed
        ;;
    *)
        echo "usage: $0 schemas|fixtures|tools|contract|tracy-live|fail-closed|compare|compare-live|native-live|workflow" >&2
        exit 2
        ;;
esac
