#!/usr/bin/env python3
from __future__ import annotations

import argparse
import copy
import json
import os
import platform
import subprocess
import sys
import time
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from gecko_to_folded import GeckoProfileError, convert_profile, load_json, write_folded
from profile_orchestrator import (
    ProfileFailure,
    build_itest,
    cpu_name,
    display_path,
    find_godot,
    git_value,
    make_run_id,
    prepare_output,
    re_safe_name,
    resolve_executable,
    run_text,
    write_gdextension,
)
from profile_schema import (
    DISCLOSURE,
    NATIVE_ARTIFACTS,
    NATIVE_SCHEMA_NAME,
    SchemaValidationError,
    validate_native_summary,
)

SAMPLY_VERSION = "0.13.1"
SAMPLING_RATE = 1000
WORKLOAD_TIMEOUT_SECONDS = 900


def create_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Capture whole-process native CPU samples for an itest benchmark."
    )
    parser.add_argument("--native", action="store_true")
    parser.add_argument("--bench", required=True, help="exact benchmark name")
    parser.add_argument(
        "--seconds",
        type=int,
        default=5,
        help="minimum benchmark workload duration",
    )
    parser.add_argument(
        "--output",
        type=Path,
        help="artifact directory; an existing nonempty directory is rejected",
    )
    return parser


def artifact_records(output: Path) -> list[dict[str, Any]]:
    records = []
    for kind, filename in NATIVE_ARTIFACTS.items():
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


def collect_environment(
    repository: Path,
    godot: str,
    rustc: str,
    samply: str,
    git_short: str,
) -> dict[str, Any]:
    status = subprocess.run(
        ["git", "status", "--porcelain"],
        cwd=repository,
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.DEVNULL,
        check=False,
    )
    samply_version = run_text([samply, "--version"], repository)
    if samply_version.split()[-1:] != [SAMPLY_VERSION]:
        raise ProfileFailure(
            "configuration",
            f"Samply {SAMPLY_VERSION} is required, found {samply_version}",
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
        "samply_version": SAMPLY_VERSION,
        "features": [],
    }


def initial_document(
    run_id: str,
    environment: dict[str, Any],
    benchmark: str,
    seconds: int,
    output: Path,
) -> dict[str, Any]:
    return {
        "$schema": NATIVE_SCHEMA_NAME,
        "schema_version": 1,
        "run_id": run_id,
        "evidence_kind": "native-sampling-profile",
        "benchmark_compatible": False,
        "disclosure": DISCLOSURE,
        "complete": False,
        "outcome": "incomplete",
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "environment": environment,
        "selection": {
            "mode": "exact",
            "requested": benchmark,
            "selected": 1,
            "benchmarks": [benchmark],
        },
        "scope": {
            "kind": "whole-process",
            "includes_startup": True,
            "includes_teardown": True,
            "measurement_window": "spawn-to-exit",
        },
        "sampling": {
            "profiler": "samply",
            "rate_hz": SAMPLING_RATE,
            "reuse_threads": False,
            "minimum_workload_seconds": seconds,
            "observed_wall_seconds": None,
            "workload_extension": "loop-selected-benchmark",
        },
        "samples": {
            "count": 0,
            "unknown_leaf_count": 0,
            "unknown_leaf_ratio": None,
        },
        "symbols": {"rust": False, "godot": False},
        "hotspots": [],
        "errors": [],
        "artifacts": artifact_records(output),
        "metadata": {
            "sample_scope": "whole Godot process and its threads",
            "cpu_sampling_only": True,
            "allocation_counts_available": False,
            "automatic_privilege_or_signing_changes": False,
        },
    }


def write_document(
    document: dict[str, Any], output: Path, schema: Path, *, require_complete: bool
) -> None:
    document["artifacts"] = artifact_records(output)
    validate_native_summary(document, schema, require_complete=require_complete)
    temporary = output / ".native-summary.json.tmp"
    with temporary.open("w", encoding="utf-8") as handle:
        json.dump(document, handle, indent=2, sort_keys=True)
        handle.write("\n")
    os.replace(temporary, output / "native-summary.json")


def record_failure(
    document: dict[str, Any],
    output: Path,
    schema: Path,
    failure: ProfileFailure,
) -> None:
    document["complete"] = False
    document["outcome"] = "error"
    document["generated_at"] = datetime.now(timezone.utc).isoformat()
    document["errors"].append(
        {"kind": failure.kind, "message": str(failure), "metadata": {}}
    )
    try:
        write_document(document, output, schema, require_complete=False)
    except (OSError, SchemaValidationError) as error:
        print(f"Failed to update incomplete native profile checkpoint: {error}", file=sys.stderr)


def _profiler_failure_message() -> str:
    if sys.platform.startswith("linux"):
        return (
            "Samply could not access Linux perf events. Set kernel.perf_event_paranoid=1 "
            "for this profiling session (for example, sudo sysctl -w "
            "kernel.perf_event_paranoid=1) and rerun. This tool never changes "
            "privileges automatically."
        )
    if sys.platform == "darwin":
        return (
            "Samply could not profile the spawned Godot binary. Use an unsigned or locally "
            "signed Godot build. This tool will not run samply setup or re-sign binaries."
        )
    return "Samply could not profile the spawned Godot process; check samply.log."


def _looks_like_profiler_failure(log: str) -> bool:
    lowered = log.lower()
    return any(
        pattern in lowered
        for pattern in (
            "encountered an error during profiling",
            "profiling failed",
            "permission denied",
            "operation not permitted",
            "perf_event",
            "could not obtain the root task",
            "codesign",
            "mach task",
        )
    )


def normalize_symbol_sidecar(profile: Path) -> Path:
    expected = Path(f"{profile}.syms.json")
    if expected.is_file():
        return expected
    alternate = profile.with_suffix(".syms.json")
    if alternate.is_file():
        os.replace(alternate, expected)
        return expected
    raise ProfileFailure(
        "symbols",
        "Samply did not produce its .syms.json sidecar; ensure debug symbols are readable",
    )


def capture(
    repository: Path,
    output: Path,
    benchmark: str,
    seconds: int,
    samply: str,
    godot: str,
) -> float:
    profile = output / NATIVE_ARTIFACTS["profile"]
    samply_log = output / NATIVE_ARTIFACTS["samply-log"]
    godot_log = output / NATIVE_ARTIFACTS["godot-log"]
    environment = os.environ.copy()
    environment.pop("BENCHMARK_FILTER", None)
    environment.pop("BENCHMARK_JSON", None)
    environment.pop("BENCHMARK_JSON_PATH", None)
    environment["BENCHMARK_EXACT"] = benchmark
    environment["GBPROF_NATIVE_SECONDS"] = str(seconds)
    command = [
        samply,
        "record",
        "--rate",
        str(SAMPLING_RATE),
        "--save-only",
        "--unstable-presymbolicate",
        "--output",
        str(profile),
        "--",
        godot,
        "--headless",
        "--path",
        str(repository / "itest" / "godot"),
        "--log-file",
        str(godot_log),
        "addons/godot-bevy/test/BenchRunner.tscn",
        "--quit-after",
        "30000",
    ]
    started = time.monotonic()
    try:
        with samply_log.open("w", encoding="utf-8") as log:
            result = subprocess.run(
                command,
                cwd=repository,
                env=environment,
                stdout=log,
                stderr=subprocess.STDOUT,
                text=True,
                timeout=WORKLOAD_TIMEOUT_SECONDS,
                check=False,
            )
    except subprocess.TimeoutExpired as error:
        raise ProfileFailure(
            "workload-timeout",
            f"native benchmark workload exceeded {WORKLOAD_TIMEOUT_SECONDS} seconds",
            1,
        ) from error
    except OSError as error:
        raise ProfileFailure("configuration", f"failed to launch Samply: {error}") from error
    elapsed = time.monotonic() - started
    if result.returncode != 0:
        try:
            log_text = samply_log.read_text(encoding="utf-8")
        except OSError:
            log_text = ""
        if _looks_like_profiler_failure(log_text) or not profile.is_file():
            raise ProfileFailure("profiler", _profiler_failure_message())
        raise ProfileFailure(
            "workload", f"profiled benchmark workload exited {result.returncode}", 1
        )
    if not profile.is_file() or profile.stat().st_size == 0:
        raise ProfileFailure("capture", "Samply did not produce a native profile")
    return elapsed


def render_flamegraph(
    folded: Path, flamegraph: Path, inferno: str, samply_log: Path
) -> None:
    try:
        with folded.open("r", encoding="utf-8") as source, flamegraph.open(
            "w", encoding="utf-8"
        ) as destination, samply_log.open("a", encoding="utf-8") as log:
            result = subprocess.run(
                [inferno, "--title", DISCLOSURE],
                stdin=source,
                stdout=destination,
                stderr=log,
                text=True,
                timeout=60,
                check=False,
            )
    except (OSError, subprocess.TimeoutExpired) as error:
        raise ProfileFailure("flamegraph", f"inferno-flamegraph failed: {error}") from error
    if result.returncode != 0:
        raise ProfileFailure("flamegraph", f"inferno-flamegraph exited {result.returncode}")
    try:
        svg = flamegraph.read_text(encoding="utf-8")
    except OSError as error:
        raise ProfileFailure("flamegraph", f"failed to read flamegraph SVG: {error}") from error
    if "<svg" not in svg or DISCLOSURE not in svg:
        raise ProfileFailure("flamegraph", "inferno-flamegraph produced an invalid SVG")


def run(args: argparse.Namespace) -> int:
    repository = Path(__file__).resolve().parents[1]
    schema = repository / "itest" / "schema" / NATIVE_SCHEMA_NAME
    benchmark = args.bench.strip()
    if not benchmark:
        raise ProfileFailure("configuration", "--bench must not be empty")
    if args.seconds < 5:
        raise ProfileFailure("configuration", "--seconds must be at least 5")
    selector = re_safe_name(benchmark)
    if not selector:
        raise ProfileFailure("configuration", "benchmark name has no safe path characters")
    run_id, git_short = make_run_id(repository)
    output = (
        args.output.resolve()
        if args.output is not None
        else repository
        / "target"
        / "profiles"
        / "bench"
        / selector
        / "native"
        / run_id
    )

    cargo = resolve_executable(os.environ.get("CARGO", "cargo"), "Cargo")
    rustc = resolve_executable(os.environ.get("RUSTC", "rustc"), "rustc")
    godot = find_godot()
    samply = resolve_executable(os.environ.get("SAMPLY_BIN", "samply"), "Samply")
    inferno = resolve_executable(
        os.environ.get("INFERNO_FLAMEGRAPH_BIN", "inferno-flamegraph"),
        "inferno-flamegraph",
    )
    environment = collect_environment(repository, godot, rustc, samply, git_short)
    build_itest(repository, cargo, repository / "target", features=None)
    prepare_output(output)
    document = initial_document(
        run_id, environment, benchmark, args.seconds, output
    )
    write_document(document, output, schema, require_complete=False)

    try:
        write_gdextension(repository)
        elapsed = capture(
            repository, output, benchmark, args.seconds, samply, godot
        )
        profile_path = output / NATIVE_ARTIFACTS["profile"]
        symbols_path = normalize_symbol_sidecar(profile_path)
        result = convert_profile(load_json(profile_path), load_json(symbols_path))
        folded_path = output / NATIVE_ARTIFACTS["folded-stacks"]
        write_folded(result, folded_path)
        render_flamegraph(
            folded_path,
            output / NATIVE_ARTIFACTS["flamegraph"],
            inferno,
            output / NATIVE_ARTIFACTS["samply-log"],
        )

        document["sampling"]["observed_wall_seconds"] = elapsed
        document["samples"] = {
            "count": result.sample_count,
            "unknown_leaf_count": result.unknown_leaf_count,
            "unknown_leaf_ratio": result.unknown_leaf_ratio,
        }
        document["symbols"] = {
            "rust": result.rust_symbols,
            "godot": result.godot_symbols,
        }
        document["hotspots"] = [
            {
                "name": name,
                "samples": count,
                "percent": count / result.sample_count * 100,
            }
            for name, count in result.leaf_counts.most_common(20)
        ]
        document["generated_at"] = datetime.now(timezone.utc).isoformat()
        write_document(document, output, schema, require_complete=False)

        final_document = copy.deepcopy(document)
        final_document["complete"] = True
        final_document["outcome"] = "pass"
        final_document["generated_at"] = datetime.now(timezone.utc).isoformat()
        write_document(final_document, output, schema, require_complete=True)
    except GeckoProfileError as error:
        failure = ProfileFailure("conversion", str(error))
        record_failure(document, output, schema, failure)
        print(f"Native profile failed: {error}", file=sys.stderr)
        print(f"Partial artifacts: {display_path(output, repository)}", file=sys.stderr)
        return failure.exit_code
    except (OSError, SchemaValidationError) as error:
        failure = ProfileFailure("schema", str(error))
        record_failure(document, output, schema, failure)
        print(f"Native profile failed: {error}", file=sys.stderr)
        print(f"Partial artifacts: {display_path(output, repository)}", file=sys.stderr)
        return failure.exit_code
    except ProfileFailure as failure:
        record_failure(document, output, schema, failure)
        print(f"Native profile failed: {failure}", file=sys.stderr)
        print(f"Partial artifacts: {display_path(output, repository)}", file=sys.stderr)
        return failure.exit_code

    print(
        f"Whole-process samples: {final_document['samples']['count']}; "
        f"unknown leaves: {final_document['samples']['unknown_leaf_ratio']:.1%}"
    )
    summary = output / "native-summary.json"
    print(f"Native profile complete: {display_path(summary, repository)}")
    return 0


def main() -> int:
    args = create_parser().parse_args()
    print(DISCLOSURE, flush=True)
    try:
        return run(args)
    except ProfileFailure as failure:
        print(f"Native profile failed: {failure}", file=sys.stderr)
        return failure.exit_code
    except (OSError, SchemaValidationError) as error:
        print(f"Native profile failed: {error}", file=sys.stderr)
        return 2
    except KeyboardInterrupt:
        print("Native profile interrupted", file=sys.stderr)
        return 2


if __name__ == "__main__":
    raise SystemExit(main())
