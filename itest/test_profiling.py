#!/usr/bin/env python3
from __future__ import annotations

import copy
import json
import shutil
import subprocess
import sys
import tempfile
from collections import Counter
from pathlib import Path
from typing import Any

import profile_orchestrator
from profile_orchestrator import (
    ARTIFACTS,
    ProfileFailure,
    SelectionRequest,
    capture_workload,
    initial_document,
    record_failure,
    selection_request,
)
from profile_schema import (
    DISCLOSURE,
    SCHEMA_NAME,
    SchemaValidationError,
    validate_profile_spans,
)
from profile_tracy import (
    AggregationError,
    aggregate_exports,
    normalize_source_file,
    parse_marker,
    parse_tsv,
)

REPOSITORY = Path(__file__).resolve().parents[1]
SCHEMA = REPOSITORY / "godot-bevy-test" / "schema" / SCHEMA_NAME
FIXTURES = Path(__file__).resolve().parent / "fixtures" / "profiling"
INCLUSIVE = FIXTURES / "zones-inclusive.tsv"
SELF = FIXTURES / "zones-self.tsv"


def require(condition: bool, message: str) -> None:
    if not condition:
        raise AssertionError(message)


def fixture_workload(run_id: str = "fixture-run") -> dict[str, Any]:
    return {
        "benchmark_compatible": False,
        "disclosure": DISCLOSURE,
        "profile_run_id": run_id,
        "selection": {
            "mode": "exact",
            "requested": "fixture_bench",
            "patterns": [],
            "registered": 27,
            "selected": 1,
            "benchmarks": ["fixture_bench"],
        },
        "profiling": {
            "warmup_iterations": 5,
            "sample_iterations": 21,
            "inner_repetitions": {"fixture_bench": 3},
        },
    }


def fixture_environment() -> dict[str, Any]:
    return {
        "cargo_profile": "profiling",
        "git_commit": "0123456789abcdef",
        "git_short": "0123456",
        "git_dirty": False,
        "os": "fixture-os",
        "arch": "fixture-arch",
        "cpu": "fixture-cpu",
        "rustc_version": "rustc fixture",
        "godot_version": "Godot fixture",
        "tracy_version": "0.13.1",
        "features": ["profile-tracy", "trace_bevy", "trace_tracy"],
    }


def fixture_aggregation(run_id: str = "fixture-run") -> dict[str, Any]:
    return aggregate_exports(
        INCLUSIVE,
        SELF,
        fixture_workload(run_id),
        run_id,
        REPOSITORY,
    )


def valid_documents(output: Path) -> tuple[dict[str, Any], dict[str, Any]]:
    request = SelectionRequest("exact", "fixture_bench", [])
    incomplete = initial_document(
        "fixture-run", fixture_environment(), request, output
    )
    complete = copy.deepcopy(incomplete)
    for filename in ARTIFACTS.values():
        (output / filename).write_text("fixture\n", encoding="utf-8")
    complete.update(fixture_aggregation())
    complete["artifacts"] = [
        {
            "kind": kind,
            "path": filename,
            "present": True,
            "size_bytes": (output / filename).stat().st_size,
            "metadata": {},
        }
        for kind, filename in ARTIFACTS.items()
    ]
    complete["complete"] = True
    complete["outcome"] = "pass"
    return incomplete, complete


def verify_schemas() -> None:
    with tempfile.TemporaryDirectory() as temporary:
        incomplete, complete = valid_documents(Path(temporary))
        validate_profile_spans(incomplete, SCHEMA, require_complete=False)
        validate_profile_spans(complete, SCHEMA, require_complete=True)
        print("PASS profile-spans-v1: complete-and-incomplete")

        invalid_disclosure = copy.deepcopy(complete)
        invalid_disclosure["benchmark_compatible"] = True
        try:
            validate_profile_spans(invalid_disclosure, SCHEMA)
        except SchemaValidationError:
            pass
        else:
            raise AssertionError("benchmark disclosure fields were not enforced")
        print("PASS disclosure fields: required")

        extension = copy.deepcopy(complete)
        extension["metadata"]["fixture-extension"] = {"allowed": True}
        validate_profile_spans(extension, SCHEMA, require_complete=True)
        extension["unexpected"] = True
        try:
            validate_profile_spans(extension, SCHEMA)
        except SchemaValidationError:
            pass
        else:
            raise AssertionError("unexpected top-level property was accepted")

        duplicate_artifact = copy.deepcopy(complete)
        duplicate_artifact["artifacts"][0]["kind"] = "workload"
        try:
            validate_profile_spans(duplicate_artifact, SCHEMA)
        except SchemaValidationError:
            pass
        else:
            raise AssertionError("duplicate typed artifact was accepted")

        try:
            validate_profile_spans([], SCHEMA)
        except SchemaValidationError:
            pass
        else:
            raise AssertionError("non-object profile document was accepted")
        print("PASS extension points: constrained")

    aggregation = fixture_aggregation()
    require(
        normalize_source_file(
            str(REPOSITORY / "godot-bevy-test" / "src" / "profiling.rs"),
            REPOSITORY,
        )
        == "godot-bevy-test/src/profiling.rs",
        "repository source-path normalization",
    )
    require(
        normalize_source_file("/nix/store/abc-source/server/TracyWorker.cpp")
        == "<nix-source>/server/TracyWorker.cpp",
        "Nix source-path normalization",
    )
    require(len(aggregation["spans"]) == 2, "fixture aggregation span count")
    require(
        {span["name"] for span in aggregation["spans"]}
        == {"hot_zone", "occasional_zone"},
        "fixture warmup and measured-window filtering",
    )
    hot = next(span for span in aggregation["spans"] if span["name"] == "hot_zone")
    require(
        hot["inclusive"]["occurrence_count"] == 21
        and hot["inclusive"]["total_ns"] == 1050
        and hot["inclusive"]["median_ns"] == 50
        and hot["inclusive"]["p95_ns"] == 59
        and hot["inclusive"]["p99_ns"] is None,
        "fixture inclusive statistics and quantile thresholds",
    )
    require(
        hot["self"]["total_ns"] == 840
        and hot["self"]["median_ns"] == 40
        and hot["self"]["p95_ns"] == 49,
        "fixture self-time statistics",
    )
    occasional = next(
        span for span in aggregation["spans"] if span["name"] == "occasional_zone"
    )
    require(
        occasional["inclusive"]["p95_ns"] is None
        and occasional["inclusive"]["p95_reason"]
        == "requires at least 20 occurrences",
        "fixture sparse quantile reason",
    )

    with tempfile.TemporaryDirectory() as temporary:
        mismatched_self = Path(temporary) / "zones-self.tsv"
        mismatched_self.write_text(
            SELF.read_text(encoding="utf-8").replace(
                "hot_zone\titest/rust/src/benchmarks.rs\t20\t1250",
                "hot_zone\titest/rust/src/benchmarks.rs\t21\t1250",
                1,
            ),
            encoding="utf-8",
        )
        try:
            aggregate_exports(
                INCLUSIVE,
                mismatched_self,
                fixture_workload(),
                "fixture-run",
                REPOSITORY,
            )
        except AggregationError:
            pass
        else:
            raise AssertionError("inclusive/self identity mismatch was accepted")

        def write_high_cardinality(source: Path, destination: Path, duration: int) -> None:
            lines = source.read_text(encoding="utf-8").splitlines()
            zones = [
                "\t".join(
                    [
                        f"dynamic_zone{{id={index}}}",
                        "itest/rust/src/benchmarks.rs",
                        "40",
                        "1255",
                        str(duration),
                        "1",
                        "",
                    ]
                )
                for index in range(101)
            ]
            destination.write_text(
                "\n".join([*lines[:-1], *zones, lines[-1]]) + "\n",
                encoding="utf-8",
            )

        high_inclusive = Path(temporary) / "high-inclusive.tsv"
        high_self = Path(temporary) / "high-self.tsv"
        write_high_cardinality(INCLUSIVE, high_inclusive, 10)
        write_high_cardinality(SELF, high_self, 5)
        high_cardinality = aggregate_exports(
            high_inclusive,
            high_self,
            fixture_workload(),
            "fixture-run",
            REPOSITORY,
        )
        require(
            any(
                warning["kind"] == "high-name-cardinality"
                for warning in high_cardinality["warnings"]
            ),
            "high emitted-name cardinality warning",
        )
    print("PASS profiling fixtures: all")


def read_toml(path: Path) -> dict[str, Any]:
    import tomllib

    with path.open("rb") as handle:
        return tomllib.load(handle)


def verify_tools() -> None:
    for executable in ("tracy-capture", "tracy-csvexport"):
        resolved = shutil.which(executable)
        require(resolved is not None, f"missing {executable}")
        require(
            "0.13.1" in str(Path(resolved).resolve()),
            f"{executable} is not Tracy 0.13.1 (tracy-client-sys 0.28.0 protocol)",
        )
    devenv_nix = (REPOSITORY / "devenv.nix").read_text(encoding="utf-8")
    require(
        "tracy-client-sys in Cargo.lock" in devenv_nix,
        "devenv records the tracy/tracy-client-sys pairing constraint",
    )
    require(
        not (REPOSITORY / "nix" / "tracy-tools.nix").exists(),
        "custom tracy derivation should be gone (nixpkgs tracy matches the lockfile)",
    )
    print("PASS tracy tools: package=0.13.1 capture=true csvexport=true")

    samply = shutil.which("samply")
    inferno = shutil.which("inferno-flamegraph")
    require(samply is not None and inferno is not None, "native profiling tools")
    version = subprocess.run(
        [samply, "--version"],
        text=True,
        stdout=subprocess.PIPE,
        stderr=subprocess.STDOUT,
        check=False,
    )
    require(version.returncode == 0 and "0.13.1" in version.stdout, "samply version")
    print("PASS native tools: samply=0.13.1 inferno=true")

    workspace = read_toml(REPOSITORY / "Cargo.toml")
    profile = workspace["profile"]["profiling"]
    require(
        profile == {"inherits": "release", "debug": 1, "strip": "none"},
        "profiling Cargo profile",
    )
    print("PASS cargo profile: profiling optimized=true debuginfo=true strip=none")

    library = read_toml(REPOSITORY / "godot-bevy" / "Cargo.toml")
    features = library["features"]
    require(features["trace_bevy"] == ["bevy_app/trace", "bevy_ecs/trace"], "trace_bevy topology")
    require("trace_bevy" not in features["default"], "trace_bevy default")
    print("PASS feature topology: trace_bevy=separate default=false")

    harness = read_toml(REPOSITORY / "godot-bevy-test" / "Cargo.toml")
    tracy_features = set(harness["dependencies"]["tracing-tracy"]["features"])
    require(
        {"enable", "ondemand", "flush-on-exit", "only-localhost"}
        <= tracy_features,
        "Tracy automation features",
    )
    print("PASS tracy automation: ondemand flush-on-exit only-localhost")
    require(
        "manual-lifetime" not in tracy_features
        and "broadcast" not in tracy_features,
        "Tracy excluded features",
    )
    print("PASS tracy exclusions: manual-lifetime=false broadcast=false")


def fixture_markers() -> list[Any]:
    return [
        marker
        for zone in parse_tsv(INCLUSIVE, REPOSITORY)
        if (marker := parse_marker(zone)) is not None
    ]


def without_measured(source: Path, destination: Path) -> None:
    lines = source.read_text(encoding="utf-8").splitlines()
    destination.write_text(
        "\n".join(line for line in lines if not line.startswith("__gbprof::measured"))
        + "\n",
        encoding="utf-8",
    )


def verify_pipeline_guards() -> None:
    compare = REPOSITORY / ".github" / "scripts" / "benchmarks-compare.py"
    merge = REPOSITORY / ".github" / "scripts" / "benchmarks-merge.py"
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        profile = root / "profile.json"
        normal = root / "normal.json"
        output = root / "output.json"
        profile.write_text(json.dumps(fixture_workload()), encoding="utf-8")
        normal.write_text(json.dumps({"benchmarks": {}}), encoding="utf-8")
        compare_result = subprocess.run(
            [sys.executable, str(compare), str(normal), str(profile), str(output)],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        merge_result = subprocess.run(
            [sys.executable, str(merge), str(output), str(profile)],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        require(compare_result.returncode != 0, "comparison accepted profile workload")
        require(merge_result.returncode != 0, "merge accepted profile workload")

        normal_compare = subprocess.run(
            [sys.executable, str(compare), str(normal), str(normal), str(output)],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        merged = root / "merged.json"
        normal_merge = subprocess.run(
            [sys.executable, str(merge), str(merged), str(normal), str(normal)],
            text=True,
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            check=False,
        )
        require(normal_compare.returncode == 0, "comparison rejected normal workload")
        require(normal_merge.returncode == 0, "merge rejected normal workload")


def verify_contract() -> None:
    exact = selection_request("fixture_bench", None)
    require(exact.mode == "exact" and exact.requested == "fixture_bench", "exact selector")
    aggregation = fixture_aggregation()
    require(aggregation["selection"]["benchmarks"] == ["fixture_bench"], "exact selection result")
    print("PASS exact selection: one benchmark")

    filtered = selection_request(None, " alpha, ,beta ")
    require(
        filtered.requested == "alpha,beta"
        and filtered.patterns == ["alpha", "beta"],
        "filter normalization",
    )
    print("PASS substring selection: explicit-list-recorded")

    empty = fixture_workload()
    empty["selection"]["selected"] = 0
    empty["selection"]["benchmarks"] = []
    empty["profiling"]["inner_repetitions"] = {}
    try:
        aggregate_exports(INCLUSIVE, SELF, empty, "fixture-run", REPOSITORY)
    except AggregationError:
        pass
    else:
        raise AssertionError("zero selection was accepted")
    print("PASS zero selection: exit=2")

    profile_source = (REPOSITORY / "godot-bevy-test" / "src" / "profiling.rs").read_text()
    itest_source = (REPOSITORY / "itest" / "rust" / "src" / "lib.rs").read_text()
    require("tracing::dispatcher::has_been_set()" in profile_source, "subscriber collision check")
    require("on_stage_init" in itest_source, "subscriber startup hook")
    print("PASS subscriber collision: exit=2")

    counts = Counter(marker.kind for marker in fixture_markers())
    require(counts["run_begin"] == 1 and counts["run_end"] == 1, "run boundaries")
    require(counts["iteration"] == 26, "warmup and sample markers")
    print("PASS lifecycle markers: warmups=5 samples=21")

    hot = next(span for span in aggregation["spans"] if span["name"] == "hot_zone")
    require(
        hot["inclusive"]["per_sample"][0]["normalized_total_ns"] == 40 / 3,
        "inner repetition timing normalization",
    )
    require(
        hot["inclusive"]["per_sample"][0]["normalized_count"] == 1 / 3,
        "inner repetition count normalization",
    )
    print("PASS inner repetitions: normalized")

    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        inclusive = root / "inclusive.tsv"
        self_path = root / "self.tsv"
        without_measured(INCLUSIVE, inclusive)
        without_measured(SELF, self_path)
        fallback = aggregate_exports(
            inclusive, self_path, fixture_workload(), "fixture-run", REPOSITORY
        )
    require(
        next(
            item
            for item in fallback["workload"]["benchmarks"]
            if item["name"] == "fixture_bench"
        )["measured"]
        is False,
        "whole-iteration fallback metadata",
    )
    require(
        "setup_zone" in {span["name"] for span in fallback["spans"]},
        "whole-iteration fallback window",
    )
    print("PASS measured fallback: whole-iteration")

    require(fixture_workload()["benchmark_compatible"] is False, "workload disclosure")
    print("PASS workload disclosure: benchmark_compatible=false")
    verify_pipeline_guards()
    print("PASS benchmark pipeline guard: profiled-input-rejected")


def verify_fail_closed() -> None:
    request = SelectionRequest("exact", "fixture_bench", [])

    def executable(path: Path, source: str) -> Path:
        path.write_text(f"#!/usr/bin/env python3\n{source}", encoding="utf-8")
        path.chmod(0o755)
        return path

    def capture_failure(capture_source: str, godot_source: str) -> ProfileFailure:
        original_select_port = profile_orchestrator.select_port
        profile_orchestrator.select_port = lambda: 43123
        try:
            with tempfile.TemporaryDirectory() as temporary:
                root = Path(temporary)
                capture = executable(root / "capture", capture_source)
                godot = executable(root / "godot", godot_source)
                try:
                    capture_workload(
                        REPOSITORY,
                        root,
                        request,
                        "fixture-run",
                        str(capture),
                        str(godot),
                    )
                except ProfileFailure as failure:
                    document = initial_document(
                        "fixture-run", fixture_environment(), request, root
                    )
                    record_failure(document, root, SCHEMA, failure)
                    checkpoint = json.loads(
                        (root / "spans.json").read_text(encoding="utf-8")
                    )
                    validate_profile_spans(
                        checkpoint, SCHEMA, require_complete=False
                    )
                    require(
                        checkpoint["complete"] is False,
                        "failure checkpoint completeness",
                    )
                    return failure
                raise AssertionError("fake profiling failure completed successfully")
        finally:
            profile_orchestrator.select_port = original_select_port

    timeout = capture_failure(
        "import time\ntime.sleep(20)\n",
        "import os, pathlib, time\npathlib.Path(os.environ['GBPROF_GATE_PATH']).write_text('timeout')\ntime.sleep(20)\n",
    )
    require(timeout.exit_code == 2 and timeout.kind == "connection", "connection timeout code")
    print("PASS connect timeout: exit=2 complete=false")

    workload = capture_failure(
        "import time\ntime.sleep(20)\n",
        "import os, pathlib\npathlib.Path(os.environ['GBPROF_GATE_PATH']).write_text('connected')\nraise SystemExit(1)\n",
    )
    require(workload.exit_code == 1 and workload.kind == "workload", "workload exit code")

    protocol = capture_failure(
        "print('incompatible protocol version')\nraise SystemExit(1)\n",
        "import time\ntime.sleep(20)\n",
    )
    require(protocol.exit_code == 2 and protocol.kind == "capture", "protocol mismatch code")
    print("PASS protocol mismatch: exit=2 complete=false")

    wrong_workload = fixture_workload("wrong-run")
    try:
        aggregate_exports(INCLUSIVE, SELF, wrong_workload, "wrong-run", REPOSITORY)
    except AggregationError:
        pass
    else:
        raise AssertionError("wrong run marker was accepted")
    print("PASS wrong run marker: exit=2 complete=false")

    truncated = capture_failure(
        "import pathlib, sys, time\nout = pathlib.Path(sys.argv[sys.argv.index('-o') + 1])\nout.write_bytes(b'truncated')\ntime.sleep(0.2)\n",
        "import os, pathlib, time\npathlib.Path(os.environ['GBPROF_GATE_PATH']).write_text('connected')\ntime.sleep(0.05)\n",
    )
    require(truncated.exit_code == 2 and truncated.kind == "capture", "truncated capture code")
    print("PASS truncated capture: exit=2 complete=false")

    missing = capture_failure(
        "import time\ntime.sleep(0.2)\n",
        "import os, pathlib, time\npathlib.Path(os.environ['GBPROF_GATE_PATH']).write_text('connected')\ntime.sleep(0.05)\n",
    )
    require(missing.exit_code == 2 and missing.kind == "capture", "capture write code")
    print("PASS capture write failure: exit=2 complete=false")

    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)

        corrupt = root / "corrupt.tsv"
        corrupt.write_text("wrong\theader\n", encoding="utf-8")
        try:
            parse_tsv(corrupt)
        except AggregationError:
            pass
        else:
            raise AssertionError("corrupt export was accepted")

        output = root / "checkpoint"
        output.mkdir()
        document = initial_document(
            "fixture-run",
            fixture_environment(),
            SelectionRequest("exact", "fixture_bench", []),
            output,
        )
        failure = ProfileFailure("export", "corrupt export")
        record_failure(document, output, SCHEMA, failure)
        checkpoint = json.loads((output / "spans.json").read_text(encoding="utf-8"))
        validate_profile_spans(checkpoint, SCHEMA, require_complete=False)
        require(checkpoint["complete"] is False, "failed checkpoint completeness")
        print("PASS corrupt export: exit=2 complete=false")


def verify_live(path: Path) -> None:
    document = json.loads(path.read_text(encoding="utf-8"))
    validate_profile_spans(document, SCHEMA, require_complete=True)
    require(
        document["workload"]["connection_gate"]
        == {"mechanism": "ondemand+Client::is_connected", "timeout_seconds": 10},
        "Tracy connection gate",
    )
    print("PASS tracy gate: connected-before-workload")
    require(document["run_id"], "run identity")
    print("PASS tracy identity: unique-run-marker")
    require(
        document["selection"]["benchmarks"]
        == ["transform_sync_bevy_to_godot_3d"],
        "live selection",
    )
    print("PASS tracy selection: transform_sync_bevy_to_godot_3d")
    require(
        document["workload"]["warmup_iterations"] == 5
        and document["workload"]["sample_iterations"] == 21,
        "live lifecycle windows",
    )
    print("PASS tracy windows: warmups=5 samples=21")
    artifact_kinds = {
        artifact["kind"] for artifact in document["artifacts"] if artifact["present"]
    }
    require({"zones-inclusive", "zones-self"} <= artifact_kinds, "live exports")
    print("PASS tracy exports: inclusive=true self=true")
    require(document["spans"], "live non-marker zones")
    print("PASS tracy spans: non-marker-zones-present")
    print("PASS profile-spans-v1: complete=true")
    require(document["benchmark_compatible"] is False, "live disclosure")
    print("PASS disclosure: benchmark_compatible=false")


def main() -> int:
    if len(sys.argv) not in {2, 3}:
        print(
            "usage: test_profiling.py "
            "schemas|tools|contract|fail-closed|live [spans.json]",
            file=sys.stderr,
        )
        return 2
    mode = sys.argv[1]
    if mode == "schemas" and len(sys.argv) == 2:
        verify_schemas()
    elif mode == "tools" and len(sys.argv) == 2:
        verify_tools()
    elif mode == "contract" and len(sys.argv) == 2:
        verify_contract()
    elif mode == "fail-closed" and len(sys.argv) == 2:
        verify_fail_closed()
    elif mode == "live" and len(sys.argv) == 3:
        verify_live(Path(sys.argv[2]))
    else:
        print("invalid profiling test mode", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
