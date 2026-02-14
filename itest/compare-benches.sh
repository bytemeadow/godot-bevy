#!/bin/bash
set -e

# Compare benchmarks between the current branch and a base branch (default: main).
# Uses a git worktree so the working tree stays untouched.
#
# Usage:
#   ./compare-benches.sh              # compare against main
#   ./compare-benches.sh other-branch # compare against other-branch

CYAN='\033[0;36m'
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
BOLD='\033[1m'
NC='\033[0m'

BASE_REF="${1:-main}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WORKTREE_DIR="$REPO_ROOT/.bench-baseline"
RESULTS_DIR="$SCRIPT_DIR/.bench-results"

mkdir -p "$RESULTS_DIR"

# ── Resolve Godot binary ────────────────────────────────────────────
resolve_godot() {
    if [ -n "$GODOT4_BIN" ]; then return; fi
    if command -v godot4 &>/dev/null; then GODOT4_BIN="godot4"; return; fi
    if command -v godot  &>/dev/null; then GODOT4_BIN="godot";  return; fi
    if [ -f "/Applications/Godot.app/Contents/MacOS/Godot" ]; then
        GODOT4_BIN="/Applications/Godot.app/Contents/MacOS/Godot"; return
    fi
    if [ -f "$HOME/Library/Application Support/gdenv/bin/godot" ]; then
        GODOT4_BIN="$HOME/Library/Application Support/gdenv/bin/godot"; return
    fi
    echo -e "${RED}Error: Could not find Godot binary${NC}"
    echo "Set GODOT4_BIN to your Godot 4 executable."
    exit 1
}

# ── Run benchmarks for a given itest directory ──────────────────────
run_benchmarks() {
    local itest_dir="$1"
    local json_out="$2"
    local label="$3"
    local godot_dir="$itest_dir/godot"

    # Point .gdextension at the shared target dir
    cat > "$godot_dir/itest.gdextension" << EOF
[configuration]
entry_symbol = "godot_bevy_itest"
compatibility_minimum = 4.2

[libraries]
linux.debug.x86_64 = "$REPO_ROOT/target/release/libgodot_bevy_itest.so"
linux.release.x86_64 = "$REPO_ROOT/target/release/libgodot_bevy_itest.so"
windows.debug.x86_64 = "$REPO_ROOT/target/release/godot_bevy_itest.dll"
windows.release.x86_64 = "$REPO_ROOT/target/release/godot_bevy_itest.dll"
macos.debug = "$REPO_ROOT/target/release/libgodot_bevy_itest.dylib"
macos.release = "$REPO_ROOT/target/release/libgodot_bevy_itest.dylib"
macos.debug.arm64 = "$REPO_ROOT/target/release/libgodot_bevy_itest.dylib"
macos.release.arm64 = "$REPO_ROOT/target/release/libgodot_bevy_itest.dylib"
EOF

    mkdir -p "$godot_dir/.godot"
    echo "res://itest.gdextension" > "$godot_dir/.godot/extension_list.cfg"

    "$GODOT4_BIN" --headless --path "$godot_dir" --import --quit 2>/dev/null || true

    BENCHMARK_JSON=1 BENCHMARK_JSON_PATH="$json_out" \
        "$GODOT4_BIN" --headless --path "$godot_dir" \
        addons/godot-bevy/test/BenchRunner.tscn --quit-after 30000 \
        2>&1 | tee "$RESULTS_DIR/${label}-output.txt"

    # Fall back to parsing stdout markers if the file wasn't written directly
    if [ ! -f "$json_out" ]; then
        sed -n '/===BENCHMARK_JSON_START===/,/===BENCHMARK_JSON_END===/p' \
            "$RESULTS_DIR/${label}-output.txt" | sed '1d;$d' > "$json_out"
    fi

    if ! python3 -c "import json,sys; json.load(open(sys.argv[1]))" "$json_out" 2>/dev/null; then
        echo -e "${RED}Error: Invalid JSON in $json_out${NC}"
        exit 1
    fi
}

# ── Cleanup on exit ─────────────────────────────────────────────────
cleanup() {
    if [ -d "$WORKTREE_DIR" ]; then
        echo -e "${CYAN}Cleaning up worktree...${NC}"
        git -C "$REPO_ROOT" worktree remove "$WORKTREE_DIR" --force 2>/dev/null || true
    fi
}
trap cleanup EXIT

# ── Main ────────────────────────────────────────────────────────────
resolve_godot
echo -e "${BOLD}Comparing benchmarks: current branch vs ${BASE_REF}${NC}"
echo -e "  Godot: $GODOT4_BIN"
echo ""

# 1. Build & run baseline (base branch via worktree)
echo -e "${CYAN}━━━ Baseline (${BASE_REF}) ━━━${NC}"
git -C "$REPO_ROOT" worktree remove "$WORKTREE_DIR" --force 2>/dev/null || true
git -C "$REPO_ROOT" worktree add "$WORKTREE_DIR" "$BASE_REF"

echo -e "${CYAN}Building baseline...${NC}"
CARGO_TARGET_DIR="$REPO_ROOT/target" cargo build --release \
    --manifest-path "$WORKTREE_DIR/itest/rust/Cargo.toml"

echo -e "${CYAN}Running baseline benchmarks...${NC}"
run_benchmarks "$WORKTREE_DIR/itest" "$RESULTS_DIR/baseline.json" "baseline"

# Remove worktree before building current branch (frees the ref)
git -C "$REPO_ROOT" worktree remove "$WORKTREE_DIR" --force 2>/dev/null || true

# 2. Build & run current branch
echo ""
echo -e "${CYAN}━━━ Current branch ━━━${NC}"
echo -e "${CYAN}Building current branch...${NC}"
cargo build --release --manifest-path "$REPO_ROOT/itest/rust/Cargo.toml"

echo -e "${CYAN}Running current branch benchmarks...${NC}"
run_benchmarks "$SCRIPT_DIR" "$RESULTS_DIR/current.json" "current"

# 3. Compare
echo ""
echo -e "${CYAN}━━━ Comparison ━━━${NC}"
python3 "$REPO_ROOT/.github/scripts/benchmarks-compare.py" \
    "$RESULTS_DIR/baseline.json" \
    "$RESULTS_DIR/current.json" \
    "$RESULTS_DIR/comparison.json"

# 4. Pretty-print the comparison table
python3 -c "
import json, sys

with open('$RESULTS_DIR/comparison.json') as f:
    data = json.load(f)

s = data['summary']
print()
if s['regressions'] > 0:
    print(f'  \033[31;1m{s[\"regressions\"]} regression(s) detected\033[0m')
elif s['improvements'] > 0:
    print(f'  \033[32;1m{s[\"improvements\"]} improvement(s)\033[0m')
else:
    print('  Performance is stable — no significant changes')
print()

name_w = max(len(b['name']) for b in data['benchmarks']) + 2
print(f\"  {'Benchmark':<{name_w}} {'Current':>12} {'Baseline':>12} {'Change':>12}\")
print(f\"  {'─'*name_w} {'─'*12} {'─'*12} {'─'*12}\")

for b in data['benchmarks']:
    current  = b.get('current', '-')
    baseline = b.get('baseline') or '-'
    pct      = b.get('change_pct')

    if b['status'] == 'new':
        change = 'new'
    elif pct is not None:
        sign = '+' if pct > 0 else ''
        icon = {'regression':'🔴','slower':'🟡','faster':'🟢'}.get(b['status'],'')
        change = f'{icon} {sign}{pct:.1f}%'
    else:
        change = ''

    print(f\"  {b['name']:<{name_w}} {current:>12} {baseline:>12} {change:>12}\")

print()
"

echo -e "${GREEN}Done.${NC} Raw results in $RESULTS_DIR/"
