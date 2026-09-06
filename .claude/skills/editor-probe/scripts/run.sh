#!/bin/bash
set -euo pipefail
exec python3 - "$0" "$@" <<'DRIVER'
import json
import os
from pathlib import Path
import re
import shutil
import signal
import subprocess
import sys
import tempfile

here = Path(sys.argv[1]).resolve().parent
repo = here.parents[3]
godot = os.environ.get("GODOT4_BIN", "godot")
timeout = int(os.environ.get("EDITOR_PROBE_TIMEOUT", "240"))
log_path = Path(os.environ.get("EDITOR_PROBE_LOG", "/tmp/editor-probe.log"))


def import_status(project):
    status = subprocess.check_output([
        "git", "-C", str(project), "status", "--porcelain=v1", "-z",
        "--untracked-files=all", "--", "*.import",
    ])
    entries = iter(status.split(b"\0"))
    paths = {}
    for entry in entries:
        if not entry:
            continue
        paths[repo / os.fsdecode(entry[3:])] = entry[:2]
        if b"R" in entry[:2] or b"C" in entry[:2]:
            next(entries)
    return paths


def stop(editor):
    if editor is not None and editor.poll() is None:
        editor.terminate()
        try:
            editor.wait(timeout=10)
        except subprocess.TimeoutExpired:
            editor.kill()
            editor.wait()


def run_project(project, probes, log):
    addon = project / "addons/editor_probe"
    scene = project / "scenes/editor_probe.tscn"
    scene_uid = scene.with_suffix(".tscn.uid")
    for path in (addon, scene, scene_uid):
        if path.exists() or path.is_symlink():
            raise ValueError(f"temporary probe path already exists: {path}")
    project_file = project / "project.godot"
    imports = {path: path.read_bytes() if path.is_file() else None
               for path in import_status(project)}
    editor = None
    with tempfile.TemporaryDirectory(prefix="editor-probe-") as temporary:
        backup = Path(temporary) / "project.godot"
        shutil.copy2(project_file, backup)
        try:
            addon.mkdir(parents=True)
            scene.parent.mkdir(parents=True, exist_ok=True)
            shutil.copy2(here / "plugin.gd", addon / "plugin.gd")
            (addon / "probe.json").write_text(json.dumps(probes))
            (addon / "plugin.cfg").write_text(
                '[plugin]\nname="editor_probe"\ndescription="temporary editor probe"\n'
                'author="editor-probe skill"\nversion="0"\nscript="plugin.gd"\n'
            )
            text = project_file.read_text()
            entry = '"res://addons/editor_probe/plugin.cfg"'
            section = re.search(r"(?ms)^\[editor_plugins\]\s*\n(.*?)(?=^\[|\Z)", text)
            if section:
                settings = section.group(1)
                enabled = re.search(r"enabled=PackedStringArray\((.*?)\)", settings, re.S)
                if enabled:
                    inner = enabled.group(1).strip()
                    settings = (settings[:enabled.start(1)]
                                + (f"{inner}, {entry}" if inner else entry)
                                + settings[enabled.end(1):])
                else:
                    settings += f"enabled=PackedStringArray({entry})\n"
                text = text[:section.start(1)] + settings + text[section.end(1):]
            else:
                text += f"\n[editor_plugins]\n\nenabled=PackedStringArray({entry})\n"
            project_file.write_text(text)
            offset = log.tell()
            print(f"editor-probe: project={project.relative_to(repo)} probes={len(probes)}", flush=True)
            editor = subprocess.Popen([godot, "--editor", "--path", str(project)],
                                      stdout=log, stderr=subprocess.STDOUT)
            timed_out = False
            try:
                editor.wait(timeout=timeout)
            except subprocess.TimeoutExpired:
                timed_out = True
                stop(editor)
            log.seek(offset)
            output = log.read()
            log.seek(0, os.SEEK_END)
            verdicts = re.findall(r"^EDITOR_PROBE verdict=(\d+) .*", output, re.M)
            for line in output.splitlines():
                if line.startswith("EDITOR_PROBE"):
                    print(line)
            reason = (f"editor timed out after {timeout}s" if timed_out else
                      f"editor exited with status {editor.returncode} before completing probes; see {log_path}")
            for probe in probes[len(verdicts):]:
                print(f"EDITOR_PROBE verdict=5 class={probe['class']} "
                      f"property={probe['property']} mismatches={json.dumps([reason])}")
            if timed_out or len(verdicts) != len(probes) or "EDITOR_PROBE complete" not in output:
                print(f"editor-probe: {reason}", file=sys.stderr)
                return 5
            return max(map(int, verdicts))
        finally:
            stop(editor)
            shutil.copy2(backup, project_file)
            shutil.rmtree(addon, ignore_errors=True)
            scene.unlink(missing_ok=True)
            scene_uid.unlink(missing_ok=True)
            changed = [str(path.relative_to(repo)) for path, status in import_status(project).items()
                       if status[1:2] == b"M" and path not in imports]
            if changed:
                subprocess.run(["git", "-C", str(repo), "checkout", "--", *changed], check=True)
            for path, data in imports.items():
                if data is None:
                    path.unlink(missing_ok=True)
                else:
                    path.write_bytes(data)


def main():
    args = sys.argv[2:]
    if len(args) != 2:
        raise ValueError("usage: run.sh <project dir> <probe.json> | run.sh --manifest <file>")
    config = json.loads(Path(args[1]).read_text())
    if args[0] == "--manifest":
        if not isinstance(config, list) or not config:
            raise ValueError("manifest must be a non-empty JSON list of probes")
        probes = config
    else:
        probes = [dict(config, project=str(Path(args[0]).resolve().relative_to(repo)))]
    projects = {}
    for probe in probes:
        for key in ("project", "class", "property", "shots"):
            if not isinstance(probe.get(key), str) or not probe[key]:
                raise ValueError(f"probe requires a non-empty {key}: {probe}")
        if "value" not in probe or not isinstance(probe.get("expect", {}), dict):
            raise ValueError(f"probe requires value and expect must be an object: {probe}")
        project = (repo / probe["project"]).resolve()
        if Path(probe["project"]).is_absolute() or not project.is_relative_to(repo):
            raise ValueError(f"project must be relative to the repo root: {probe['project']}")
        if not (project / "project.godot").is_file():
            raise ValueError(f"missing project.godot: {project}")
        projects.setdefault(project, []).append(probe)
    log_path.parent.mkdir(parents=True, exist_ok=True)
    result = 0
    with log_path.open("w+") as log:
        for project, probes in projects.items():
            result = max(result, run_project(project, probes, log))
    return result


def interrupted(signum, frame):
    raise KeyboardInterrupt


signal.signal(signal.SIGTERM, interrupted)
try:
    sys.exit(main())
except (OSError, ValueError, subprocess.CalledProcessError) as error:
    print(f"editor-probe: {error}", file=sys.stderr)
    sys.exit(2)
except KeyboardInterrupt:
    sys.exit(130)
DRIVER
