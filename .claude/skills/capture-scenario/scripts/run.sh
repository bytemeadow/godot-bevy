#!/bin/bash
# usage: run.sh <example> <cargo-features> <scenario>...
# Builds the example with its capture feature(s), writes the GDExtension descriptor, imports the
# project once, checks the built library, runs each scenario through the driver and prints one
# summary line per scenario. Exit 2 if any scenario could not run; the scenario outcomes are
# printed for the caller to judge, since a negative scenario is expected to exit 1.
set -uo pipefail
if [ $# -lt 3 ]; then
  sed -n 2,6p "$0" >&2
  exit 2
fi
example="$1"; features="$2"; shift 2
repo="$(cd "$(dirname "$0")/../../../.." && pwd)"
cd "$repo"
run="${CAPTURE_RUN:-$(date -u +%Y%m%dT%H%M%SZ)}"
timeout="${CAPTURE_TIMEOUT:-120}"
manifest="examples/$example/rust/Cargo.toml"
project="examples/$example/godot"
godot="${GODOT4_BIN:-${GODOT:-godot}}"

cargo build --manifest-path "$manifest" --features "$features" || exit 2
cargo run -p xtask -- gdextension --manifest-path "$manifest" >/dev/null || exit 2
if [ ! -d "$project/.godot/imported" ]; then
  # A cold headless import can crash at cleanup after writing .godot (godot#111645).
  (cd "$project" && "$godot" --headless --import >/dev/null 2>&1) 2>/dev/null || true
  [ -d "$project/.godot/imported" ] || { echo "import failed for $project" >&2; exit 2; }
fi
python3 itest/capture/capture.py --example "$example" --scenario "$1" --check-library >/dev/null || exit 2

worst=0
for scenario in "$@"; do
  python3 itest/capture/capture.py --example "$example" --scenario "$scenario" --run "$run-$scenario" --timeout "$timeout" >/dev/null 2>&1
  code=$?
  [ "$code" -gt "$worst" ] && worst=$code
  leaf="target/example-evidence/$run-$scenario/$example/$scenario"
  python3 - "$scenario" "$code" "$leaf" <<'PY'
import json, sys
from pathlib import Path
scenario, code, leaf = sys.argv[1], int(sys.argv[2]), Path(sys.argv[3])
verdict = leaf / "verdict.json"
if not verdict.is_file():
    print(f"CAPTURE {scenario} exit={code} verdict=missing leaf={leaf}")
    sys.exit()
v = json.loads(verdict.read_text())
caps = {k: c for k, c in v.get("capabilities", {}).items() if c != "untested"}
diffs = [(d.get("frame"), d.get("path")) for d in v.get("differences", [])]
errors = v.get("errors") or []
print(f"CAPTURE {scenario} exit={code} outcome={v.get('outcome')} complete={v.get('complete')} ack={v.get('acknowledged')} capabilities={caps} differences={diffs[:8]} errors={errors[:3]} leaf={leaf}")
PY
done
churn="$(git status --short -- "$project" | grep -E '\.import$|project\.godot$' || true)"
if [ -n "$churn" ]; then
  echo "Godot rewrote tracked project files; discard unless you changed them:"
  echo "$churn"
fi
[ "$worst" -ge 2 ] && exit 2
exit 0
