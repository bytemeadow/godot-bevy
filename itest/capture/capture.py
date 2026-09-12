#!/usr/bin/env python3
from __future__ import annotations

import argparse
import configparser
import copy
import ctypes
import json
import os
import platform
import queue
import re
import signal
import struct
import subprocess
import sys
import threading
import time
import uuid
import zlib
from datetime import datetime, timezone
from pathlib import Path

import capture_schema
from capture_compare import compare_checkpoint
from capture_schema import (
    CaptureValidationError, PROTOCOL, REPOSITORY, frame_file, load_manifest,
    loads_json, validate_manifest, write_json,
)


def png_error(path: Path, viewport: list[int]) -> str | None:
    try:
        data = path.read_bytes()
        if data[:8] != b"\x89PNG\r\n\x1a\n":
            return "missing PNG signature"
        offset = 8
        pixels = bytearray()
        channels = None
        while offset + 12 <= len(data):
            size = struct.unpack_from(">I", data, offset)[0]
            kind = data[offset + 4:offset + 8]
            body = data[offset + 8:offset + 8 + size]
            end = offset + 12 + size
            if end > len(data) or zlib.crc32(kind + body) != struct.unpack_from(">I", data, end - 4)[0]:
                return "truncated PNG or bad CRC"
            if offset == 8:
                if kind != b"IHDR" or len(body) != 13:
                    return "missing IHDR"
                w, h, depth, colour, compression, filtering, interlace = struct.unpack(">IIBBBBB", body)
                if [w, h] != viewport or depth != 8 or colour not in (2, 6) or any((compression, filtering, interlace)):
                    return "unexpected PNG dimensions or format"
                channels = 3 if colour == 2 else 4
            if kind == b"IDAT":
                pixels.extend(body)
            if kind == b"IEND":
                if size or end != len(data) or not pixels or channels is None:
                    return "invalid PNG ending"
                raw = zlib.decompress(pixels)
                stride = 1 + viewport[0] * channels
                if len(raw) != viewport[1] * stride:
                    return "incomplete PNG pixels"
                if any(raw[row] > 4 for row in range(0, len(raw), stride)):
                    return "invalid PNG row filter"
                return None
            offset = end
        return "missing IEND"
    except (OSError, ValueError, struct.error, zlib.error) as error:
        return str(error)


class CaptureSession:
    def __init__(self, manifest: dict, output: Path):
        self.manifest = manifest
        self.output = output
        self.ready = False
        self.scene = None
        self.seen = []
        self.diffs = []
        self.errors = []
        self.extension_files = []
        self.capabilities = {name: "untested" for name in (*PROTOCOL["untested"], *manifest["extensions"])}
        self.expected_exit = None
        self.acknowledged = False
        self.configuration_error = False
        if manifest["pacing"] == "realtime":
            self.write_requests()

    def write_requests(self):
        for checkpoint in self.manifest["checkpoints"]:
            frame = checkpoint["frame"]
            write_json(self.output / frame_file(frame, "request"), {"version": PROTOCOL["version"], "frame": frame})

    def accept(self, line: str) -> None:
        tokens = line.split(maxsplit=1)
        token = tokens[0] if tokens else ""
        if token == PROTOCOL["error"]:
            raise CaptureValidationError(line)
        if token == PROTOCOL["ready"]:
            match = re.fullmatch(re.escape(PROTOCOL["ready"]) + r" scenario=([^\s]+) scene=(res://[^\s]+\.tscn) frame=0", line)
            if self.ready or not match or match[1] != self.manifest["scenario"]:
                raise CaptureValidationError(f"unexpected READY: {line}")
            self.ready = True
            self.scene = match[2]
            if self.manifest["pacing"] == "fixed":
                self.write_requests()
            return
        if token == PROTOCOL["ack"]:
            expected = f"{PROTOCOL['ack']} exit={self.expected_exit}"
            if self.expected_exit is None or self.acknowledged or line != expected:
                raise CaptureValidationError(f"unexpected acknowledgement: {line}")
            self.acknowledged = True
            return
        if token == PROTOCOL["extension"]:
            match = re.fullmatch(re.escape(token) + r" adapter=([a-z0-9][a-z0-9_-]*) frame=([0-9]{1,7}) (.+)", line)
            if not match or match[1] not in self.manifest["extensions"]:
                raise CaptureValidationError("malformed or undeclared extension facts")
            name, frame, payload = match[1], int(match[2]), match[3]
            filename = f"{PROTOCOL['frame_prefix']}{frame:0{PROTOCOL['frame_digits']}d}.{name}{PROTOCOL['extension_suffix']}"
            if frame > self.manifest["frames"] or filename in self.extension_files or self.expected_exit is not None:
                raise CaptureValidationError("duplicate, late or out-of-range extension facts")
            loads_json(payload)
            (self.output / filename).write_text(payload + "\n", encoding="utf-8")
            self.extension_files.append(filename)
            return
        if token != PROTOCOL["facts"]:
            return
        match = re.fullmatch(re.escape(token) + r" frame=([0-9]{1,7}) (.+)", line)
        if not self.ready or not match:
            raise CaptureValidationError("FACTS before READY or malformed FACTS")
        frame = int(match[1])
        checkpoints = self.manifest["checkpoints"]
        if len(self.seen) >= len(checkpoints) or frame != checkpoints[len(self.seen)]["frame"]:
            raise CaptureValidationError(f"unexpected checkpoint frame {frame}")
        facts = loads_json(match[2])
        write_json(self.output / frame_file(frame, "facts"), facts)
        diffs = compare_checkpoint(self.manifest, checkpoints[len(self.seen)], facts, scene=self.scene)
        error = png_error(self.output / frame_file(frame, "png"), self.manifest["viewport"])
        if error:
            self.errors.append(f"frame {frame}: invalid PNG: {error}")
            diffs.append({"path": "png", "expected": "complete viewport PNG", "actual": error})
        write_json(self.output / frame_file(frame, "diff"), diffs)
        self.diffs.extend(dict(diff, frame=frame) for diff in diffs)
        self.seen.append(frame)

    def evaluate_extensions(self):
        for name, value in self.manifest["extensions"].items():
            path = capture_schema.ADAPTER_ROOT / name / PROTOCOL["extension_verdict"]
            if not path.is_file():
                continue
            try:
                callback = capture_schema.adapter_function(name, "extension_verdict", "verdict")
                result = callback(copy.deepcopy(value), self.output)
            except Exception as error:
                raise CaptureValidationError(f"extensions.{name} verdict: {error}") from error
            if not isinstance(result, dict) or set(result) != {name} or result[name] not in ("pass", "fail", "untested"):
                raise CaptureValidationError(f"extensions.{name}: verdict must set exactly its own capability")
            self.capabilities[name] = result[name]

    def instruction(self):
        self.evaluate_extensions()
        self.expected_exit = self.comparison_verdict()["exit_code"]
        return {"version": PROTOCOL["version"], "scenario": self.manifest["scenario"], "exit_code": self.expected_exit}

    def comparison_verdict(self) -> dict:
        required = [PROTOCOL[key] for key in ("manifest_file", "stdout_file", "stderr_file", "verdict_file")]
        for point in self.manifest["checkpoints"]:
            required.extend(frame_file(point["frame"], kind) for kind in ("png", "facts", "diff"))
        required.extend(self.extension_files)
        complete = self.ready and self.seen == [p["frame"] for p in self.manifest["checkpoints"]]
        missing = [name for name in required if name != PROTOCOL["verdict_file"] and not (self.output / name).is_file()]
        complete = complete and not missing and not self.errors
        rendering_passed = complete and not self.diffs
        passed = rendering_passed and "fail" not in self.capabilities.values()
        return {
            "version": PROTOCOL["version"], "example": self.manifest["example"], "scenario": self.manifest["scenario"],
            "scene": self.scene, "outcome": "pass" if passed else "fail", "complete": complete,
            "exit_code": PROTOCOL["exit"]["pass" if passed else "fail"],
            "required_artifacts": required, "missing_artifacts": missing,
            "checkpoints": self.seen, "differences": self.diffs, "errors": self.errors,
            "capabilities": {"rendering_2d": "pass" if rendering_passed else "fail", **self.capabilities},
        }

    def verdict(self) -> dict:
        verdict = self.comparison_verdict()
        required = [PROTOCOL[key] for key in ("instruction_file", "driver_pid_file", "godot_pid_file")]
        required.extend(frame_file(p["frame"], "request") for p in self.manifest["checkpoints"])
        verdict["required_artifacts"].extend(required)
        verdict["missing_artifacts"].extend(name for name in required if not (self.output / name).is_file())
        verdict["acknowledged"] = self.acknowledged
        if not self.acknowledged or verdict["missing_artifacts"]:
            verdict.update(complete=False, outcome="fail", exit_code=PROTOCOL["exit"]["fail"])
            verdict["capabilities"]["rendering_2d"] = "fail"
        if self.configuration_error:
            verdict.update(complete=False, outcome="error", exit_code=PROTOCOL["exit"]["error"])
        return verdict


def require_capture_library(project: Path) -> None:
    config = configparser.ConfigParser(interpolation=None)
    try:
        with (project / "rust.gdextension").open(encoding="utf-8") as descriptor:
            config.read_file(descriptor)
        system = {"Darwin": "macos", "Linux": "linux", "Windows": "windows"}[platform.system()]
        arch = {"aarch64": "arm64", "AMD64": "x86_64"}.get(platform.machine(), platform.machine())
        features = {system, arch, "debug"}
        entries = [(key, value) for key, value in config["libraries"].items()
                   if {system, "debug"} <= set(key.split('.')) <= features]
        if not entries:
            raise ValueError("no matching debug library in rust.gdextension")
        _, path = max(entries, key=lambda item: len(item[0].split('.')))
        path = path.strip().strip('"')
        library = (project / path[6:]).resolve() if path.startswith("res://") else Path(path).resolve()
        loaded = ctypes.CDLL(str(library))
        marker = getattr(loaded, PROTOCOL["capture_symbol"])
        marker.argtypes = []
        marker.restype = ctypes.c_uint32
        if marker() != PROTOCOL["version"]:
            raise ValueError("capture protocol version mismatch")
    except (OSError, KeyError, ValueError, AttributeError, configparser.Error) as error:
        raise CaptureValidationError(f"{error}; build the descriptor's debug cdylib with --features capture") from error


def _stop(process: subprocess.Popen) -> list[str]:
    errors = []
    if os.name == "posix":
        for sig, wait in ((signal.SIGTERM, 2), (signal.SIGKILL, 5)):
            try:
                os.killpg(process.pid, sig)
            except ProcessLookupError:
                pass
            except OSError as error:
                if process.poll() is None:
                    errors.append(str(error))
                    try:
                        process.kill()
                    except OSError as failure:
                        errors.append(str(failure))
            try:
                process.wait(timeout=wait)
            except subprocess.TimeoutExpired as error:
                if sig == signal.SIGKILL:
                    errors.append(str(error))
    elif process.poll() is None:
        subprocess.run(["taskkill", "/PID", str(process.pid), "/T", "/F"], capture_output=True, check=False)
        process.wait(timeout=5)
    return errors


def run_capture(manifest: dict, output: Path, godot: str, timeout: float) -> int:
    errors = validate_manifest(manifest)
    if errors:
        print("; ".join(errors), file=sys.stderr)
        return PROTOCOL["exit"]["error"]
    session = CaptureSession(manifest, output)
    project = REPOSITORY / "examples" / manifest["example"] / "godot"
    write_json(output / PROTOCOL["manifest_file"], manifest)
    (output / PROTOCOL["driver_pid_file"]).write_text(f"{os.getpid()}\n", encoding="utf-8")
    command = [godot, "--path", str(project), "--windowed", "--render-thread", "safe",
               "--resolution", "x".join(map(str, manifest["viewport"])), "--scene", manifest["scene"]]
    if manifest["pacing"] == "fixed":
        command.extend(["--fixed-fps", str(PROTOCOL["fps"])])
    environment = dict(os.environ)
    for key, value in {
        "scenario": manifest["scenario"], "manifest": str(output / PROTOCOL["manifest_file"]),
        "output": str(output), "timeout": str(timeout),
    }.items():
        environment[PROTOCOL["env"][key]] = value
    process = None
    reader = None
    cleaning = False

    def interrupted(signum, _frame):
        nonlocal cleaning
        if not cleaning:
            cleaning = True
            raise KeyboardInterrupt(f"terminated by signal {signum}")

    previous = {sig: signal.signal(sig, interrupted) for sig in (signal.SIGTERM, signal.SIGINT)}
    try:
        with (output / PROTOCOL["stdout_file"]).open("w", encoding="utf-8") as stdout_log, \
                (output / PROTOCOL["stderr_file"]).open("w", encoding="utf-8") as stderr_log:
            lines = queue.Queue()

            def read_stdout():
                try:
                    for line in process.stdout:
                        stdout_log.write(line)
                        stdout_log.flush()
                        lines.put(line.rstrip("\r\n"))
                except (OSError, ValueError):
                    pass
                finally:
                    lines.put(None)

            try:
                if not 0 < timeout <= 3600:
                    raise CaptureValidationError("timeout must be in (0, 3600] seconds")
                if environment.get("GODOT_BEVY_ITEST"):
                    raise CaptureValidationError("GODOT_BEVY_ITEST must be unset for capture")
                if not (project / "rust.gdextension").is_file() or not (project / ".godot").is_dir():
                    raise CaptureValidationError("capture requires a built extension, rust.gdextension and an imported project")
                require_capture_library(project)
                for name in manifest["extensions"]:
                    (output / name).mkdir(exist_ok=True)
                process = subprocess.Popen(command, cwd=project, env=environment, stdout=subprocess.PIPE,
                                           stderr=stderr_log, text=True, encoding="utf-8", errors="replace",
                                           start_new_session=(os.name == "posix"),
                                           creationflags=subprocess.CREATE_NEW_PROCESS_GROUP if os.name == "nt" else 0)
                (output / PROTOCOL["godot_pid_file"]).write_text(f"{process.pid}\n", encoding="utf-8")
                reader = threading.Thread(target=read_stdout, daemon=True)
                reader.start()
                deadline = time.monotonic() + timeout
                while True:
                    remaining = deadline - time.monotonic()
                    if remaining <= 0:
                        raise TimeoutError("capture timed out")
                    try:
                        line = lines.get(timeout=remaining)
                    except queue.Empty as error:
                        raise TimeoutError("capture timed out waiting for stdout") from error
                    if line is None:
                        break
                    session.accept(line)
                    if len(session.seen) == len(manifest["checkpoints"]) and session.expected_exit is None:
                        write_json(output / PROTOCOL["instruction_file"], session.instruction())
                status = process.wait(timeout=max(0.01, deadline - time.monotonic()))
                if not session.acknowledged:
                    session.errors.append("Godot did not acknowledge the instruction")
                if status != session.expected_exit:
                    session.errors.append(f"Godot exited with code {status}; expected verdict {session.expected_exit}")
            except (OSError, ValueError, TimeoutError, subprocess.TimeoutExpired, KeyboardInterrupt) as error:
                session.configuration_error = process is None and not isinstance(error, KeyboardInterrupt)
                session.errors.append(str(error) or "interrupted")
            finally:
                cleaning = True
                if process is not None:
                    try:
                        session.errors.extend(f"cleanup: {error}" for error in _stop(process))
                    except (OSError, subprocess.TimeoutExpired) as error:
                        session.errors.append(f"cleanup: {error}")
                    if reader is not None:
                        reader.join(timeout=5)
                    if reader is None or not reader.is_alive():
                        process.stdout.close()
        verdict = session.verdict()
        verdict["command"] = command
        write_json(output / PROTOCOL["verdict_file"], verdict)
        print(json.dumps({"evidence": str(output), "outcome": verdict["outcome"], "exit_code": verdict["exit_code"]}))
        return verdict["exit_code"]
    finally:
        for sig, handler in previous.items():
            signal.signal(sig, handler)


def main() -> int:
    parser = argparse.ArgumentParser(description="Capture a prepared example; builds and imports nothing")
    parser.add_argument("--example", required=True)
    parser.add_argument("--check-library", action="store_true")
    parser.add_argument("--scenario", default=os.environ.get(PROTOCOL["env"]["scenario"]))
    parser.add_argument("--manifest", type=Path)
    parser.add_argument("--godot", default=os.environ.get("GODOT4_BIN") or os.environ.get("GODOT") or "godot")
    parser.add_argument("--run", default=datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ-") + uuid.uuid4().hex[:8])
    parser.add_argument("--timeout", type=float, default=60)
    args = parser.parse_args()
    try:
        for value in (args.example, args.run):
            if not re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9_-]*", value):
                raise CaptureValidationError(f"invalid path identifier: {value}")
        if args.check_library:
            require_capture_library(REPOSITORY / "examples" / args.example / "godot")
            return PROTOCOL["exit"]["pass"]
        if not 0 < args.timeout <= 3600:
            raise CaptureValidationError("timeout must be in (0, 3600] seconds")
        if args.manifest is None:
            if not args.scenario or not re.fullmatch(r"[a-z0-9][a-z0-9_-]*", args.scenario):
                raise CaptureValidationError("a scenario or manifest is required")
            args.manifest = REPOSITORY / "examples" / args.example / "capture" / f"{args.scenario}.json"
        manifest = load_manifest(args.manifest)
        if manifest["example"] != args.example or (args.scenario and manifest["scenario"] != args.scenario):
            raise CaptureValidationError("manifest example/scenario does not match the requested run")
        output = REPOSITORY / PROTOCOL["evidence_root"] / args.run / args.example / manifest["scenario"]
        output.mkdir(parents=True, exist_ok=False)
        return run_capture(manifest, output.absolute(), args.godot, args.timeout)
    except (OSError, CaptureValidationError) as error:
        print(str(error), file=sys.stderr)
        return PROTOCOL["exit"]["error"]


if __name__ == "__main__":
    raise SystemExit(main())
