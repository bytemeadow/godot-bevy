#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

if [[ $# -ne 1 ]]; then
    echo "usage: $0 contract|tools|flush|pipeline|reports|diff|godot-live|fail-closed-live|workflow|all-offline" >&2
    exit 2
fi

case $1 in
    contract|tools|flush|pipeline|reports|diff|godot-live|fail-closed-live|workflow|all-offline)
        exec python3 "$SCRIPT_DIR/test_coverage.py" "$1"
        ;;
    *)
        echo "usage: $0 contract|tools|flush|pipeline|reports|diff|godot-live|fail-closed-live|workflow|all-offline" >&2
        exit 2
        ;;
esac
