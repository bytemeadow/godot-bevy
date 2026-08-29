#!/bin/bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

if [[ $# -ne 1 ]]; then
    echo "usage: $0 contract|mutants|faults|workflow" >&2
    exit 2
fi

case $1 in
    contract|mutants|faults|workflow)
        exec python3 "$SCRIPT_DIR/test_qualification.py" "$1"
        ;;
    *)
        echo "usage: $0 contract|mutants|faults|workflow" >&2
        exit 2
        ;;
esac
