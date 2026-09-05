#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
for argument in "$@"; do
    if [[ $argument == --native ]]; then
        exec python3 "$SCRIPT_DIR/profile_native.py" "$@"
    fi
done
exec python3 "$SCRIPT_DIR/profile_orchestrator.py" "$@"
