#!/usr/bin/env python3
from __future__ import annotations

import argparse
import copy
import hashlib
import json
import os
import platform
import secrets
import shutil
import socket
import string
import subprocess
import sys
import time
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from profile_schema import (
    DISCLOSURE,
    PROFILE_ARTIFACTS,
    SCHEMA_NAME,
    SchemaValidationError,
    validate_profile_spans,
)
from profile_tracy import (
    HIGH_CARDINALITY_THRESHOLD,
    AggregationError,
    aggregate_exports,
)

# must match the Tracy bundled by tracy-client-sys in Cargo.lock (0.28.0 -> 0.13.1);
# a mismatch fails the capture handshake, so this is display metadata only
TRACY_VERSION = "0.13.1"
WORKLOAD_TIMEOUT_SECONDS = 900
CAPTURE_TIMEOUT_SECONDS = 60
GATE_STATUS_TIMEOUT_SECONDS = 30
ARTIFACTS = PROFILE_ARTIFACTS


class ProfileFailure(RuntimeError):
    def __init__(self, kind: str, message: str, exit_code: int = 2):
        super().__init__(message)
        self.kind = kind
        self.exit_code = exit_code


@dataclass(frozen=True)
class SelectionRequest:
    mode: str
    requested: str
    patterns: list[str]

    @property
    def directory(self) -> str:
        if self.mode == "exact":
            sanitized = re_safe_name(self.requested)
            if not sanitized:
                raise ProfileFailure("configuration", "benchmark name has no safe path characters")
            return sanitized
        digest = hashlib.sha256(self.requested.encode()).hexdigest()[:8]
        return f"selection-{digest}"


def re_safe_name(value: str) -> str:
    return "".join(
        character if character.isalnum() or character in "._-" else "_"
        for character in value
    )


def parse_filter(value: str) -> tuple[str, list[str]]:
    patterns = [pattern.strip() for pattern in value.split(",") if pattern.strip()]
    if not patterns:
        raise ProfileFailure("configuration", "--filter must contain a nonempty substring")
    return ",".join(patterns), patterns


def selection_request(bench: str | None, filter_value: str | None) -> SelectionRequest:
    if (bench is None) == (filter_value is None):
        raise ProfileFailure("configuration", "exactly one of --bench or --filter is required")
    if bench is not None:
        normalized = bench.strip()
        if not normalized:
            raise ProfileFailure("configuration", "--bench must not be empty")
        return SelectionRequest("exact", normalized, [])
    normalized, patterns = parse_filter(filter_value or "")
    return SelectionRequest("filter", normalized, patterns)


def create_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Capture an instrumented itest benchmark with Tracy.",
        epilog=(
            "The client uses Tracy ondemand mode and the Rust harness polls "
            "Client::is_connected for up to 10 seconds before benchmark work begins."
        ),
    )
    selectors = parser.add_mutually_exclusive_group(required=True)
    selectors.add_argument("--bench", help="exact benchmark name")
    selectors.add_argument("--filter", help="comma-separated benchmark substrings")
    parser.add_argument(
        "--output",
        type=Path,
        help="artifact directory; an existing nonempty directory is rejected",
    )
    parser.add_argument("--repository", type=Path, help=argparse.SUPPRESS)
    parser.add_argument("--target-dir", type=Path, help=argparse.SUPPRESS)
    parser.add_argument("--library-dir", type=Path, help=argparse.SUPPRESS)
    parser.add_argument("--skip-build", action="store_true", help=argparse.SUPPRESS)
    return parser


def run_text(command: list[str], cwd: Path, *, required: bool = True) -> str:
    try:
        result = subprocess.run(
            command,
            cwd=cwd,
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            timeout=30,
            check=False,
        )
    except (OSError, subprocess.TimeoutExpired) as error:
        if required:
            raise ProfileFailure(
                "configuration", f"failed to run {' '.join(command)}: {error}"
            ) from error
        return "unknown"
    output = result.stdout.strip().splitlines()
    if result.returncode != 0 or not output:
        if required:
            raise ProfileFailure(
                "configuration",
                f"{' '.join(command)} failed with exit {result.returncode}",
            )
        return "unknown"
    return output[0].strip()


def resolve_executable(value: str, label: str) -> str:
    resolved = shutil.which(value)
    if resolved is None:
        raise ProfileFailure("configuration", f"{label} executable not found: {value}")
    return resolved


def find_godot() -> str:
    configured = os.environ.get("GODOT4_BIN")
    if configured:
        return resolve_executable(configured, "Godot")
    for candidate in ("godot4", "godot"):
        resolved = shutil.which(candidate)
        if resolved:
            return resolved
    application = Path("/Applications/Godot.app/Contents/MacOS/Godot")
    if application.is_file():
        return str(application)
    raise ProfileFailure(
        "configuration", "Godot executable not found; set GODOT4_BIN"
    )


def git_value(repository: Path, *arguments: str) -> str:
    return run_text(["git", *arguments], repository)


def make_run_id(repository: Path) -> tuple[str, str]:
    git_short = git_value(repository, "rev-parse", "--short", "HEAD")
    timestamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    alphabet = string.ascii_lowercase + string.digits
    suffix = "".join(secrets.choice(alphabet) for _ in range(6))
    return f"{timestamp}-{git_short}-{suffix}", git_short


def cpu_name() -> str:
    if sys.platform == "darwin":
        result = subprocess.run(
            ["sysctl", "-n", "machdep.cpu.brand_string"],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.DEVNULL,
            check=False,
        )
        if result.returncode == 0 and result.stdout.strip():
            return result.stdout.strip()
    if sys.platform.startswith("linux"):
        try:
            for line in Path("/proc/cpuinfo").read_text(encoding="utf-8").splitlines():
                if line.lower().startswith(("model name", "hardware")):
                    return line.split(":", 1)[-1].strip()
        except OSError:
            pass
    return platform.processor() or platform.machine() or "unknown"


def collect_environment(
    repository: Path, godot: str, rustc: str, git_short: str
) -> dict[str, Any]:
    status = subprocess.run(
        ["git", "status", "--porcelain"],
        cwd=repository,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    return {
        "cargo_profile": "profiling",
        "git_commit": git_value(repository, "rev-parse", "HEAD"),
        "git_short": git_short,
        "git_dirty": bool(status.stdout.strip()) if status.returncode == 0 else True,
        "os": platform.system().lower(),
        "arch": platform.machine(),
        "cpu": cpu_name(),
        "rustc_version": run_text([rustc, "--version"], repository),
        "godot_version": run_text([godot, "--version"], repository),
        "tracy_version": TRACY_VERSION,
        "features": ["profile-tracy", "trace_bevy", "trace_tracy"],
    }


def artifact_records(output: Path) -> list[dict[str, Any]]:
    records = []
    for kind, filename in ARTIFACTS.items():
        path = output / filename
        present = path.is_file()
        records.append(
            {
                "kind": kind,
                "path": filename,
                "present": present,
                "size_bytes": path.stat().st_size if present else None,
                "metadata": {},
            }
        )
    return records


def initial_document(
    run_id: str,
    environment: dict[str, Any],
    request: SelectionRequest,
    output: Path,
) -> dict[str, Any]:
    return {
        "$schema": SCHEMA_NAME,
        "schema_version": 1,
        "run_id": run_id,
        "evidence_kind": "tracy-span-profile",
        "benchmark_compatible": False,
        "disclosure": DISCLOSURE,
        "complete": False,
        "outcome": "incomplete",
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "environment": environment,
        "selection": {
            "mode": request.mode,
            "requested": request.requested,
            "patterns": request.patterns,
            "registered": 0,
            "selected": 0,
            "benchmarks": [],
        },
        "workload": {
            "kind": "itest-benchmark",
            "warmup_iterations": 5,
            "sample_iterations": 21,
            "connection_gate": {
                "mechanism": "ondemand+Client::is_connected",
                "timeout_seconds": 10,
            },
            "benchmarks": [],
        },
        "quantiles": {
            "method": "nearest-rank",
            "p95_min_occurrences": 20,
            "p99_min_occurrences": 100,
        },
        "spans": [],
        "warnings": [],
        "errors": [],
        "artifacts": artifact_records(output),
        "metadata": {
            "deadlines_seconds": {
                "capture": CAPTURE_TIMEOUT_SECONDS,
                "connection_status": GATE_STATUS_TIMEOUT_SECONDS,
                "workload": WORKLOAD_TIMEOUT_SECONDS,
            },
            "high_name_cardinality_threshold": HIGH_CARDINALITY_THRESHOLD,
            "marker_prefix": "__gbprof::",
            "timing_scope": "CPU zones",
            "parallel_zone_totals_may_exceed_wall_time": True,
        },
    }


def write_document(
    document: dict[str, Any], output: Path, schema_path: Path, *, require_complete: bool
) -> None:
    document["artifacts"] = artifact_records(output)
    validate_profile_spans(document, schema_path, require_complete=require_complete)
    destination = output / "spans.json"
    temporary = output / ".spans.json.tmp"
    with temporary.open("w", encoding="utf-8") as handle:
        json.dump(document, handle, indent=2, sort_keys=True)
        handle.write("\n")
    os.replace(temporary, destination)


def record_failure(
    document: dict[str, Any],
    output: Path,
    schema_path: Path,
    failure: ProfileFailure,
) -> None:
    document["complete"] = False
    document["outcome"] = "error"
    document["spans"] = []
    document["errors"].append(
        {"kind": failure.kind, "message": str(failure), "metadata": {}}
    )
    document["generated_at"] = datetime.now(timezone.utc).isoformat()
    try:
        write_document(document, output, schema_path, require_complete=False)
    except (OSError, SchemaValidationError) as error:
        print(f"Failed to update incomplete profile checkpoint: {error}", file=sys.stderr)


def prepare_output(path: Path) -> None:
    if path.exists() and (not path.is_dir() or any(path.iterdir())):
        raise ProfileFailure(
            "configuration", f"output directory already exists and is nonempty: {path}"
        )
    path.mkdir(parents=True, exist_ok=True)


def build_itest(
    repository: Path,
    cargo: str,
    target_dir: Path,
    features: str | None = "profile-tracy",
) -> None:
    command = [
        cargo,
        "build",
        "--profile",
        "profiling",
        "--manifest-path",
        str(repository / "itest" / "rust" / "Cargo.toml"),
        "--target-dir",
        str(target_dir),
    ]
    if features is not None:
        command.extend(["--features", features])
    result = subprocess.run(command, cwd=repository, check=False)
    if result.returncode != 0:
        raise ProfileFailure(
            "configuration", f"profiling build failed with exit {result.returncode}"
        )


def write_gdextension(repository: Path, library_directory: Path | None = None) -> None:
    godot_project = repository / "itest" / "godot"
    gdextension = godot_project / "itest.gdextension"
    if library_directory is None:
        library_directory_text = "res://../../target/profiling"
    else:
        library_directory_text = str(library_directory.resolve()).replace("\\", "/")
    gdextension.write_text(
        f"""[configuration]
entry_symbol = "godot_bevy_itest"
compatibility_minimum = 4.2

[libraries]
linux.debug.x86_64 = "{library_directory_text}/libgodot_bevy_itest.so"
linux.release.x86_64 = "{library_directory_text}/libgodot_bevy_itest.so"
windows.debug.x86_64 = "{library_directory_text}/godot_bevy_itest.dll"
windows.release.x86_64 = "{library_directory_text}/godot_bevy_itest.dll"
macos.debug = "{library_directory_text}/libgodot_bevy_itest.dylib"
macos.release = "{library_directory_text}/libgodot_bevy_itest.dylib"
macos.debug.arm64 = "{library_directory_text}/libgodot_bevy_itest.dylib"
macos.release.arm64 = "{library_directory_text}/libgodot_bevy_itest.dylib"
""",
        encoding="utf-8",
    )
    extension_directory = godot_project / ".godot"
    extension_directory.mkdir(exist_ok=True)
    (extension_directory / "extension_list.cfg").write_text(
        "res://itest.gdextension\n", encoding="utf-8"
    )


def select_port() -> int:
    with socket.socket(socket.AF_INET, socket.SOCK_STREAM) as probe:
        probe.bind(("127.0.0.1", 0))
        return int(probe.getsockname()[1])


def stop_process(process: subprocess.Popen[Any]) -> None:
    if process.poll() is not None:
        return
    process.terminate()
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=5)


def validate_capture_file(path: Path) -> None:
    if not path.is_file() or path.stat().st_size < 16:
        raise ProfileFailure("capture", "tracy-capture did not write a valid capture file")


def capture_workload(
    repository: Path,
    output: Path,
    request: SelectionRequest,
    run_id: str,
    tracy_capture: str,
    godot: str,
) -> None:
    port = select_port()
    capture_path = output / ARTIFACTS["capture"]
    workload_path = output / ARTIFACTS["workload"]
    gate_path = output / ".tracy-gate"
    environment = os.environ.copy()
    environment.pop("BENCHMARK_EXACT", None)
    environment.pop("BENCHMARK_FILTER", None)
    if request.mode == "exact":
        environment["BENCHMARK_EXACT"] = request.requested
    else:
        environment["BENCHMARK_FILTER"] = request.requested
    environment.update(
        {
            "BENCHMARK_JSON": "1",
            "BENCHMARK_JSON_PATH": str(workload_path),
            "GBPROF_GATE_PATH": str(gate_path),
            "GBPROF_RUN_ID": run_id,
            "TRACY_PORT": str(port),
        }
    )

    capture_command = [
        tracy_capture,
        "-o",
        str(capture_path),
        "-a",
        "127.0.0.1",
        "-p",
        str(port),
    ]
    godot_command = [
        godot,
        "--headless",
        "--path",
        str(repository / "itest" / "godot"),
        "addons/godot-bevy/test/BenchRunner.tscn",
        "--quit-after",
        "30000",
    ]

    capture_log_path = output / ARTIFACTS["capture-log"]
    godot_log_path = output / ARTIFACTS["godot-log"]
    capture: subprocess.Popen[Any] | None = None
    workload: subprocess.Popen[Any] | None = None
    gate_connected = False
    try:
        try:
            with capture_log_path.open("w", encoding="utf-8") as capture_log, godot_log_path.open(
                "w", encoding="utf-8"
            ) as godot_log:
                capture = subprocess.Popen(
                    capture_command,
                    cwd=repository,
                    stdout=capture_log,
                    stderr=subprocess.STDOUT,
                    text=True,
                )
                workload = subprocess.Popen(
                    godot_command,
                    cwd=repository,
                    env=environment,
                    stdout=godot_log,
                    stderr=subprocess.STDOUT,
                    text=True,
                )
                started = time.monotonic()
                deadline = started + WORKLOAD_TIMEOUT_SECONDS
                gate_deadline = started + GATE_STATUS_TIMEOUT_SECONDS
                while workload.poll() is None:
                    capture_code = capture.poll()
                    if capture_code not in (None, 0):
                        stop_process(workload)
                        raise ProfileFailure(
                            "capture",
                            f"tracy-capture failed during workload with exit {capture_code}",
                        )
                    try:
                        gate_status = gate_path.read_text(encoding="utf-8").strip()
                    except OSError:
                        gate_status = ""
                    if gate_status == "connected":
                        gate_connected = True
                    elif gate_status == "timeout":
                        stop_process(workload)
                        stop_process(capture)
                        raise ProfileFailure(
                            "connection", "Tracy did not connect within 10 seconds"
                        )
                    if not gate_connected and time.monotonic() >= gate_deadline:
                        stop_process(workload)
                        stop_process(capture)
                        raise ProfileFailure(
                            "connection", "Tracy connection gate did not report success"
                        )
                    if time.monotonic() >= deadline:
                        stop_process(workload)
                        stop_process(capture)
                        raise ProfileFailure(
                            "workload-timeout",
                            f"Godot workload exceeded {WORKLOAD_TIMEOUT_SECONDS} seconds",
                            1,
                        )
                    time.sleep(0.05)

                workload_code = workload.returncode
                if not gate_connected:
                    try:
                        gate_connected = (
                            gate_path.read_text(encoding="utf-8").strip()
                            == "connected"
                        )
                    except OSError:
                        pass
                if workload_code != 0:
                    stop_process(capture)
                    if workload_code == 2:
                        raise ProfileFailure(
                            "configuration", "profiled benchmark workload exited 2"
                        )
                    raise ProfileFailure(
                        "workload", f"profiled benchmark workload exited {workload_code}", 1
                    )
                if not gate_connected:
                    stop_process(capture)
                    raise ProfileFailure(
                        "connection", "Tracy connection gate did not report success"
                    )

                try:
                    capture_code = capture.wait(timeout=CAPTURE_TIMEOUT_SECONDS)
                except subprocess.TimeoutExpired as error:
                    stop_process(capture)
                    raise ProfileFailure(
                        "capture-timeout",
                        f"tracy-capture did not finish within {CAPTURE_TIMEOUT_SECONDS} seconds",
                    ) from error
                if capture_code != 0:
                    raise ProfileFailure("capture", f"tracy-capture exited {capture_code}")
        except OSError as error:
            if workload is not None:
                stop_process(workload)
            if capture is not None:
                stop_process(capture)
            raise ProfileFailure(
                "configuration", f"failed to launch profiling process: {error}"
            ) from error
    except KeyboardInterrupt as error:
        if workload is not None:
            stop_process(workload)
        if capture is not None:
            stop_process(capture)
        raise ProfileFailure("interrupted", "profile capture was interrupted") from error
    finally:
        gate_path.unlink(missing_ok=True)

    validate_capture_file(capture_path)


def export_zones(output: Path, tracy_csvexport: str) -> None:
    capture = output / ARTIFACTS["capture"]
    capture_log = output / ARTIFACTS["capture-log"]
    commands = (
        ("zones-inclusive", [tracy_csvexport, "-u", "-s", "\t", str(capture)]),
        ("zones-self", [tracy_csvexport, "-e", "-u", "-s", "\t", str(capture)]),
    )
    for kind, command in commands:
        destination = output / ARTIFACTS[kind]
        try:
            with destination.open(
                "w", encoding="utf-8", newline=""
            ) as exported, capture_log.open("a", encoding="utf-8") as log:
                result = subprocess.run(
                    command,
                    stdout=exported,
                    stderr=log,
                    text=True,
                    timeout=CAPTURE_TIMEOUT_SECONDS,
                    check=False,
                )
        except (OSError, subprocess.TimeoutExpired) as error:
            raise ProfileFailure(
                "export", f"tracy-csvexport for {kind} failed: {error}"
            ) from error
        if result.returncode != 0:
            raise ProfileFailure(
                "export", f"tracy-csvexport for {kind} exited {result.returncode}"
            )


def load_workload(path: Path) -> dict[str, Any]:
    try:
        with path.open(encoding="utf-8") as handle:
            workload = json.load(handle)
    except (OSError, json.JSONDecodeError) as error:
        raise ProfileFailure(
            "workload", f"failed to read profiled workload JSON: {error}"
        ) from error
    if not isinstance(workload, dict):
        raise ProfileFailure("workload", "profiled workload JSON root is not an object")
    return workload


def verify_selection(selection: dict[str, Any], request: SelectionRequest) -> None:
    if (
        selection.get("mode") != request.mode
        or selection.get("requested") != request.requested
        or selection.get("patterns") != request.patterns
    ):
        raise ProfileFailure("identity", "runtime selection metadata does not match the request")


def format_duration(ns: float) -> str:
    if ns >= 1_000_000_000:
        return f"{ns / 1_000_000_000:.2f}s"
    if ns >= 1_000_000:
        return f"{ns / 1_000_000:.2f}ms"
    if ns >= 1_000:
        return f"{ns / 1_000:.2f}us"
    return f"{ns:.0f}ns"


def sample_median(timing: dict[str, Any]) -> float:
    values = sorted(sample["normalized_total_ns"] for sample in timing["per_sample"])
    return float(values[(len(values) - 1) // 2])


def print_table(document: dict[str, Any]) -> None:
    print("CPU zone time; inclusive and self totals may exceed wall time under parallel execution.")
    print(f"{'self/sample':>14} {'inclusive/sample':>18} {'calls/sample':>14}  span")
    ordered = sorted(
        document["spans"], key=lambda span: sample_median(span["self"]), reverse=True
    )
    for span in ordered:
        self_median = sample_median(span["self"])
        inclusive_median = sample_median(span["inclusive"])
        calls = sorted(
            sample["normalized_count"] for sample in span["self"]["per_sample"]
        )[10]
        location = f"{span['source_file']}:{span['source_line']}"
        print(
            f"{format_duration(self_median):>14} "
            f"{format_duration(inclusive_median):>18} {calls:>14.2f}  "
            f"{span['benchmark']} :: {span['name']} ({location})"
        )


def display_path(path: Path, repository: Path) -> str:
    try:
        return str(path.relative_to(repository))
    except ValueError:
        return str(path)


def run(args: argparse.Namespace) -> int:
    tool_repository = Path(__file__).resolve().parents[1]
    repository = (
        args.repository.resolve() if args.repository is not None else tool_repository
    )
    target_dir = (
        args.target_dir.resolve()
        if args.target_dir is not None
        else repository / "target"
    )
    schema_path = tool_repository / "itest" / "schema" / SCHEMA_NAME
    request = selection_request(args.bench, args.filter)
    run_id, git_short = make_run_id(repository)
    output = (
        args.output.resolve()
        if args.output is not None
        else repository
        / "target"
        / "profiles"
        / "bench"
        / request.directory
        / "tracy"
        / run_id
    )
    if output.exists() and (not output.is_dir() or any(output.iterdir())):
        raise ProfileFailure(
            "configuration", f"output directory already exists and is nonempty: {output}"
        )

    cargo = resolve_executable(os.environ.get("CARGO", "cargo"), "Cargo")
    rustc = resolve_executable(os.environ.get("RUSTC", "rustc"), "rustc")
    godot = find_godot()
    tracy_capture = resolve_executable(
        os.environ.get("TRACY_CAPTURE_BIN", "tracy-capture"), "tracy-capture"
    )
    tracy_csvexport = resolve_executable(
        os.environ.get("TRACY_CSVEXPORT_BIN", "tracy-csvexport"), "tracy-csvexport"
    )
    environment = collect_environment(repository, godot, rustc, git_short)

    if not args.skip_build:
        build_itest(repository, cargo, target_dir)
    elif args.library_dir is None:
        raise ProfileFailure("configuration", "--skip-build requires --library-dir")
    else:
        library_name = (
            "libgodot_bevy_itest.dylib"
            if sys.platform == "darwin"
            else "godot_bevy_itest.dll"
            if sys.platform == "win32"
            else "libgodot_bevy_itest.so"
        )
        if not (args.library_dir / library_name).is_file():
            raise ProfileFailure(
                "configuration",
                f"prebuilt profiling library not found: {args.library_dir / library_name}",
            )
    prepare_output(output)
    document = initial_document(run_id, environment, request, output)
    write_document(document, output, schema_path, require_complete=False)

    try:
        library_directory = args.library_dir
        if library_directory is None and target_dir != repository / "target":
            library_directory = target_dir / "profiling"
        write_gdextension(repository, library_directory)
        capture_workload(
            repository, output, request, run_id, tracy_capture, godot
        )
        export_zones(output, tracy_csvexport)
        workload = load_workload(output / ARTIFACTS["workload"])
        aggregation = aggregate_exports(
            output / ARTIFACTS["zones-inclusive"],
            output / ARTIFACTS["zones-self"],
            workload,
            run_id,
            repository,
        )
        verify_selection(aggregation["selection"], request)
        document.update(aggregation)
        document["generated_at"] = datetime.now(timezone.utc).isoformat()
        write_document(document, output, schema_path, require_complete=False)

        final_document = copy.deepcopy(document)
        final_document["complete"] = True
        final_document["outcome"] = "pass"
        final_document["generated_at"] = datetime.now(timezone.utc).isoformat()
        write_document(final_document, output, schema_path, require_complete=True)
    except AggregationError as error:
        failure = ProfileFailure("aggregation", str(error))
        record_failure(document, output, schema_path, failure)
        print(f"Profile failed: {error}", file=sys.stderr)
        print(f"Partial artifacts: {display_path(output, repository)}", file=sys.stderr)
        return failure.exit_code
    except (OSError, subprocess.TimeoutExpired, SchemaValidationError) as error:
        failure = ProfileFailure("schema", str(error))
        record_failure(document, output, schema_path, failure)
        print(f"Profile failed: {error}", file=sys.stderr)
        print(f"Partial artifacts: {display_path(output, repository)}", file=sys.stderr)
        return failure.exit_code
    except ProfileFailure as failure:
        record_failure(document, output, schema_path, failure)
        print(f"Profile failed: {failure}", file=sys.stderr)
        print(f"Partial artifacts: {display_path(output, repository)}", file=sys.stderr)
        return failure.exit_code

    print_table(final_document)
    spans_path = output / "spans.json"
    print(f"Profile complete: {display_path(spans_path, repository)}")
    return 0


def main() -> int:
    args = create_parser().parse_args()
    print(DISCLOSURE, flush=True)
    try:
        return run(args)
    except ProfileFailure as failure:
        print(f"Profile failed: {failure}", file=sys.stderr)
        return failure.exit_code
    except KeyboardInterrupt:
        print("Profile interrupted", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
