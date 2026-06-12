#!/bin/bash
set -eo pipefail

# Compare benchmarks between the current branch and a base branch (default: main).
# Uses a git worktree so the working tree stays untouched.
#
# Both builds happen first, then runs are interleaved (base, current, base,
# current, ...) so thermal drift and background load hit both sides equally.
# Each side's runs are merged (median of medians) before comparison.
#
# Usage:
#   ./compare-benches.sh              # compare against main
#   ./compare-benches.sh other-branch # compare against other-branch
#   ./compare-benches.sh main         # main vs main = noise floor of this machine
#
# Environment:
#   BENCH_ROUNDS=N         interleaved rounds per side (default: 3)
#   BENCHMARK_FILTER=pat   comma-separated substrings; only matching benchmarks run

CYAN='\033[0;36m'
GREEN='\033[0;32m'
RED='\033[0;31m'
YELLOW='\033[0;33m'
BOLD='\033[1m'
NC='\033[0m'

BASE_REF="${1:-main}"
ROUNDS="${BENCH_ROUNDS:-3}"
SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
WORKTREE_DIR="$REPO_ROOT/.bench-baseline"
RESULTS_DIR="$SCRIPT_DIR/.bench-results"
BASELINE_LIB_DIR="$RESULTS_DIR/baseline-lib"

rm -rf "$RESULTS_DIR"
mkdir -p "$RESULTS_DIR" "$BASELINE_LIB_DIR"

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

# ── Project setup: point .gdextension at a library dir and import ──
setup_project() {
    local godot_dir="$1"
    local lib_dir="$2"

    cat > "$godot_dir/itest.gdextension" << EOF
[configuration]
entry_symbol = "godot_bevy_itest"
compatibility_minimum = 4.2

[libraries]
linux.debug.x86_64 = "$lib_dir/libgodot_bevy_itest.so"
linux.release.x86_64 = "$lib_dir/libgodot_bevy_itest.so"
windows.debug.x86_64 = "$lib_dir/godot_bevy_itest.dll"
windows.release.x86_64 = "$lib_dir/godot_bevy_itest.dll"
macos.debug = "$lib_dir/libgodot_bevy_itest.dylib"
macos.release = "$lib_dir/libgodot_bevy_itest.dylib"
macos.debug.arm64 = "$lib_dir/libgodot_bevy_itest.dylib"
macos.release.arm64 = "$lib_dir/libgodot_bevy_itest.dylib"
EOF

    mkdir -p "$godot_dir/.godot"
    echo "res://itest.gdextension" > "$godot_dir/.godot/extension_list.cfg"

    "$GODOT4_BIN" --headless --path "$godot_dir" --import --quit 2>/dev/null || true
}

# ── Single benchmark run, writing JSON to the given path ───────────
run_once() {
    local godot_dir="$1"
    local json_out="$2"
    local label="$3"

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
echo -e "  Rounds: $ROUNDS per side, interleaved"
if [ -n "$BENCHMARK_FILTER" ]; then
    echo -e "  Filter: $BENCHMARK_FILTER"
fi
echo ""

# Both workspaces (worktree and repo) emit target/release/libgodot_bevy_itest.*
# under the shared target dir, and cargo can consider a unit fresh even when the
# dylib on disk came from the *other* workspace. Clean the local crates before
# each build so the linked library always matches the source being built.
clean_local_crates() {
    local manifest="$1"
    CARGO_TARGET_DIR="$REPO_ROOT/target" cargo clean --release \
        -p godot-bevy -p godot-bevy-test -p godot-bevy-itest \
        --manifest-path "$manifest"
}

# 1. Build baseline (base branch via detached worktree, shared target dir)
echo -e "${CYAN}━━━ Building baseline (${BASE_REF}) ━━━${NC}"
git -C "$REPO_ROOT" worktree remove "$WORKTREE_DIR" --force 2>/dev/null || true
git -C "$REPO_ROOT" worktree add --detach "$WORKTREE_DIR" "$BASE_REF"

clean_local_crates "$WORKTREE_DIR/itest/rust/Cargo.toml"
CARGO_TARGET_DIR="$REPO_ROOT/target" cargo build --release \
    --manifest-path "$WORKTREE_DIR/itest/rust/Cargo.toml"

# Stash the baseline library: building the current branch would overwrite it
for lib in libgodot_bevy_itest.so libgodot_bevy_itest.dylib godot_bevy_itest.dll; do
    if [ -f "$REPO_ROOT/target/release/$lib" ]; then
        cp "$REPO_ROOT/target/release/$lib" "$BASELINE_LIB_DIR/"
    fi
done

# 2. Build current branch
echo ""
echo -e "${CYAN}━━━ Building current branch ━━━${NC}"
clean_local_crates "$REPO_ROOT/itest/rust/Cargo.toml"
CARGO_TARGET_DIR="$REPO_ROOT/target" cargo build --release \
    --manifest-path "$REPO_ROOT/itest/rust/Cargo.toml"

# 3. Import both Godot projects
setup_project "$WORKTREE_DIR/itest/godot" "$BASELINE_LIB_DIR"
setup_project "$SCRIPT_DIR/godot" "$REPO_ROOT/target/release"

# 4. Interleaved runs: base, current, base, current, ...
for round in $(seq 1 "$ROUNDS"); do
    echo ""
    echo -e "${CYAN}━━━ Round ${round}/${ROUNDS}: baseline ━━━${NC}"
    run_once "$WORKTREE_DIR/itest/godot" \
        "$RESULTS_DIR/baseline-run${round}.json" "baseline-run${round}"

    echo ""
    echo -e "${CYAN}━━━ Round ${round}/${ROUNDS}: current ━━━${NC}"
    run_once "$SCRIPT_DIR/godot" \
        "$RESULTS_DIR/current-run${round}.json" "current-run${round}"
done

# 5. Merge each side's runs (median of medians)
python3 "$REPO_ROOT/.github/scripts/benchmarks-merge.py" \
    "$RESULTS_DIR/baseline.json" "$RESULTS_DIR"/baseline-run*.json
python3 "$REPO_ROOT/.github/scripts/benchmarks-merge.py" \
    "$RESULTS_DIR/current.json" "$RESULTS_DIR"/current-run*.json

# 6. Compare
echo ""
echo -e "${CYAN}━━━ Comparison ━━━${NC}"
# Quiet: the noise-aware table below is the authoritative local summary
python3 "$REPO_ROOT/.github/scripts/benchmarks-compare.py" \
    "$RESULTS_DIR/baseline.json" \
    "$RESULTS_DIR/current.json" \
    "$RESULTS_DIR/comparison.json" > /dev/null

# 7. Pretty-print the comparison table with per-benchmark noise estimates
python3 "$SCRIPT_DIR/print-comparison.py" "$RESULTS_DIR"

echo -e "${GREEN}Done.${NC} Raw results in $RESULTS_DIR/"
