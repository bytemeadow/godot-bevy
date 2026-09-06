#!/bin/bash
# usage: run.sh <godot project dir> <probe.json>
# Launches the Godot editor on the project with a temporary probe plugin, prints the probe's
# output, restores the project, and exits with the probe's verdict.
set -u
PROJ="$(cd "$1" && pwd)"
CFG="$(cd "$(dirname "$2")" && pwd)/$(basename "$2")"
HERE="$(cd "$(dirname "$0")" && pwd)"
GODOT="${GODOT4_BIN:-godot}"
TIMEOUT="${EDITOR_PROBE_TIMEOUT:-240}"
LOG="${EDITOR_PROBE_LOG:-/tmp/editor-probe.log}"
ADDON="$PROJ/addons/editor_probe"
PROBE_SCENE="$PROJ/scenes/editor_probe.tscn"
BACKUP="$(mktemp)"

cleanup() {
    cp "$BACKUP" "$PROJ/project.godot"
    rm -f "$BACKUP"
    rm -rf "$ADDON" "$PROBE_SCENE" "$PROBE_SCENE.uid"
    git -C "$PROJ" checkout -- . 2>/dev/null
}
trap cleanup EXIT

cp "$PROJ/project.godot" "$BACKUP"
mkdir -p "$ADDON" "$PROJ/scenes"
cp "$HERE/plugin.gd" "$ADDON/plugin.gd"
cp "$CFG" "$ADDON/probe.json"
printf '[plugin]\n\nname="editor_probe"\ndescription="temporary editor probe"\nauthor="editor-probe skill"\nversion="0"\nscript="plugin.gd"\n' > "$ADDON/plugin.cfg"
python3 - "$PROJ/project.godot" <<'EOF'
import re, sys
path = sys.argv[1]
text = open(path).read()
entry = '"res://addons/editor_probe/plugin.cfg"'
match = re.search(r'enabled=PackedStringArray\((.*?)\)', text)
if match:
    inner = match.group(1).strip()
    new = entry if not inner else f"{inner}, {entry}"
    text = text[:match.start(1)] + new + text[match.end(1):]
else:
    text += f"\n[editor_plugins]\n\nenabled=PackedStringArray({entry})\n"
open(path, "w").write(text)
EOF

: > "$LOG"
"$GODOT" --editor --path "$PROJ" > "$LOG" 2>&1 &
pid=$!
for _ in $(seq 1 "$TIMEOUT"); do
    kill -0 "$pid" 2>/dev/null || break
    sleep 1
done
if kill -0 "$pid" 2>/dev/null; then
    kill "$pid"
    wait "$pid" 2>/dev/null
    echo "editor-probe: timed out after ${TIMEOUT}s" >&2
    grep -E "EDITOR_PROBE" "$LOG"
    exit 5
fi
wait "$pid"
status=$?
grep -E "EDITOR_PROBE" "$LOG"
verdict=$(sed -n 's/^EDITOR_PROBE verdict=//p' "$LOG" | tail -1)
exit "${verdict:-$status}"
