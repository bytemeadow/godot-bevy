#!/usr/bin/env python3
from __future__ import annotations

import copy
import gzip
import json
import shutil
import subprocess
import sys
import tempfile
import xml.etree.ElementTree as ElementTree
from collections import Counter
from pathlib import Path
from typing import Any

import profile_orchestrator
import profile_native
from gecko_to_folded import GeckoProfileError, convert_profile, load_json, write_folded
from profile_compare import (
    ComparisonError,
    create_comparison,
    load_profile,
)
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
    COMPARISON_SCHEMA_NAME,
    DISCLOSURE,
    NATIVE_ARTIFACTS,
    NATIVE_SCHEMA_NAME,
    SCHEMA_NAME,
    SchemaValidationError,
    validate_native_summary,
    validate_profile_comparison,
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
SCHEMA = REPOSITORY / "itest" / "schema" / SCHEMA_NAME
COMPARISON_SCHEMA = (
    REPOSITORY / "itest" / "schema" / COMPARISON_SCHEMA_NAME
)
NATIVE_SCHEMA = REPOSITORY / "itest" / "schema" / NATIVE_SCHEMA_NAME
FIXTURES = Path(__file__).resolve().parent / "fixtures" / "profiling"
INCLUSIVE = FIXTURES / "zones-inclusive.tsv"
SELF = FIXTURES / "zones-self.tsv"
GECKO_PROFILE = FIXTURES / "gecko-profile.json"
GECKO_SYMBOLS = FIXTURES / "gecko-profile.json.syms.json"
GECKO_FOLDED = FIXTURES / "gecko-profile.folded"


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


def write_json(path: Path, document: Any) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(document, indent=2) + "\n", encoding="utf-8")


def write_profile_fixture(
    output: Path,
    run_id: str,
    scale: float = 1.0,
) -> tuple[Path, dict[str, Any]]:
    output.mkdir(parents=True, exist_ok=True)
    _, document = valid_documents(output)
    document["run_id"] = run_id
    for span in document["spans"]:
        for timing_name in ("inclusive", "self"):
            for sample in span[timing_name]["per_sample"]:
                sample["normalized_total_ns"] *= scale
                sample["normalized_count"] *= scale
    path = output / "spans.json"
    write_json(path, document)
    return path, document


def native_fixture(output: Path) -> dict[str, Any]:
    for filename in NATIVE_ARTIFACTS.values():
        content = (
            "<svg>INSTRUMENTED PROFILE — NOT BENCHMARK RESULTS</svg>\n"
            if filename.endswith(".svg")
            else "fixture\n"
        )
        (output / filename).write_text(content, encoding="utf-8")
    environment = {
        "cargo_profile": "profiling",
        "git_commit": "0123456789abcdef",
        "git_short": "0123456",
        "git_dirty": False,
        "os": "fixture-os",
        "arch": "fixture-arch",
        "cpu": "fixture-cpu",
        "rustc_version": "rustc fixture",
        "godot_version": "Godot fixture",
        "samply_version": "0.13.1",
        "features": [],
    }
    document = profile_native.initial_document(
        "native-fixture", environment, "fixture_bench", 5, output
    )
    document["sampling"]["observed_wall_seconds"] = 5.5
    document["samples"] = {
        "count": 600,
        "unknown_leaf_count": 120,
        "unknown_leaf_ratio": 0.2,
    }
    document["symbols"] = {"rust": True, "godot": True}
    document["hotspots"] = [
        {"name": "godot_bevy::fixture", "samples": 480, "percent": 80.0}
    ]
    document["artifacts"] = profile_native.artifact_records(output)
    document["complete"] = True
    document["outcome"] = "pass"
    return document


def rejected(validation: Any, document: Any, schema: Path) -> None:
    try:
        validation(document, schema)
    except SchemaValidationError:
        return
    raise AssertionError("invalid schema document was accepted")


def verify_schemas() -> None:
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        profile_output = root / "profile"
        profile_output.mkdir()
        incomplete, complete = valid_documents(profile_output)
        validate_profile_spans(incomplete, SCHEMA, require_complete=False)
        validate_profile_spans(complete, SCHEMA, require_complete=True)
        print("PASS profile-spans-v1: complete-and-incomplete")

        baseline_path, _ = write_profile_fixture(
            root / "baseline", "schema-baseline"
        )
        current_path, _ = write_profile_fixture(root / "current", "schema-current")
        comparison = create_comparison(
            [
                load_profile(baseline_path, SCHEMA, "baseline", 1),
                load_profile(current_path, SCHEMA, "current", 1),
            ],
            "descriptive",
            "comparison-fixture",
        )
        validate_profile_comparison(
            comparison, COMPARISON_SCHEMA, require_complete=True
        )
        print("PASS profile-comparison-v1: valid")

        native_output = root / "native"
        native_output.mkdir()
        native = native_fixture(native_output)
        incomplete_native = copy.deepcopy(native)
        incomplete_native["complete"] = False
        incomplete_native["outcome"] = "incomplete"
        incomplete_native["sampling"]["observed_wall_seconds"] = None
        incomplete_native["samples"] = {
            "count": 0,
            "unknown_leaf_count": 0,
            "unknown_leaf_ratio": None,
        }
        incomplete_native["symbols"] = {"rust": False, "godot": False}
        incomplete_native["hotspots"] = []
        validate_native_summary(
            incomplete_native, NATIVE_SCHEMA, require_complete=False
        )
        validate_native_summary(native, NATIVE_SCHEMA, require_complete=True)
        print("PASS native-summary-v1: valid")

        for validation, document, schema in (
            (validate_profile_spans, complete, SCHEMA),
            (validate_profile_comparison, comparison, COMPARISON_SCHEMA),
            (validate_native_summary, native, NATIVE_SCHEMA),
        ):
            invalid_disclosure = copy.deepcopy(document)
            invalid_disclosure["benchmark_compatible"] = True
            rejected(validation, invalid_disclosure, schema)
        print("PASS disclosure fields: required")

        for validation, document, schema in (
            (validate_profile_spans, complete, SCHEMA),
            (validate_profile_comparison, comparison, COMPARISON_SCHEMA),
            (validate_native_summary, native, NATIVE_SCHEMA),
        ):
            extension = copy.deepcopy(document)
            extension["metadata"]["fixture-extension"] = {"allowed": True}
            validation(extension, schema, require_complete=True)
            extension["unexpected"] = True
            rejected(validation, extension, schema)

        duplicate_artifact = copy.deepcopy(complete)
        duplicate_artifact["artifacts"][0]["kind"] = "workload"
        rejected(validate_profile_spans, duplicate_artifact, SCHEMA)
        rejected(validate_profile_spans, [], SCHEMA)
        print("PASS extension points: constrained")

    verify_fixtures()


def verify_fixtures() -> None:
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

        gecko_profile = load_json(GECKO_PROFILE)
        gecko_symbols = load_json(GECKO_SYMBOLS)
        conversion = convert_profile(gecko_profile, gecko_symbols)
        require(conversion.sample_count == 6, "Gecko weighted sample count")
        require(conversion.unknown_leaf_count == 0, "Gecko unknown leaf count")
        require(conversion.rust_symbols, "Gecko Rust symbol detection")
        require(conversion.godot_symbols, "Gecko Godot symbol detection")
        folded = Path(temporary) / "fixture.folded"
        write_folded(conversion, folded)
        require(
            folded.read_text(encoding="utf-8")
            == GECKO_FOLDED.read_text(encoding="utf-8"),
            "Gecko folded-stack golden",
        )

        compressed = Path(temporary) / "profile.json.gz"
        with gzip.open(compressed, "wt", encoding="utf-8") as handle:
            json.dump(gecko_profile, handle)
        require(load_json(compressed) == gecko_profile, "gzip Gecko profile parser")

        row_oriented = copy.deepcopy(gecko_profile)
        thread = row_oriented["threads"][0]
        for name, columns in (
            ("samples", ["stack", "time", "weight"]),
            ("stackTable", ["prefix", "frame"]),
            ("frameTable", ["func", "address", "nativeSymbol"]),
            ("funcTable", ["name", "resource"]),
        ):
            table = thread[name]
            thread[name] = {
                "length": table["length"],
                "schema": {column: index for index, column in enumerate(columns)},
                "data": [
                    [table[column][index] for column in columns]
                    for index in range(table["length"])
                ],
            }
        require(
            convert_profile(row_oriented, gecko_symbols).folded == conversion.folded,
            "row-oriented Gecko table parser",
        )

        stackless = copy.deepcopy(gecko_profile)
        stackless["threads"][0]["samples"]["stack"][0] = None
        stackless_conversion = convert_profile(stackless, gecko_symbols)
        require(
            stackless_conversion.sample_count == 6
            and stackless_conversion.unknown_leaf_count == 1,
            "stackless Gecko sample accounting",
        )

        cyclic = copy.deepcopy(gecko_profile)
        cyclic["threads"][0]["stackTable"]["prefix"][0] = 2
        try:
            convert_profile(cyclic, gecko_symbols)
        except GeckoProfileError:
            pass
        else:
            raise AssertionError("cyclic Gecko stack table was accepted")

        require(
            profile_native._looks_like_profiler_failure(
                "perf_event_open failed: Operation not permitted"
            ),
            "Samply permission failure classification",
        )
        failure_message = profile_native._profiler_failure_message()
        if sys.platform.startswith("linux"):
            require(
                "perf_event_paranoid" in failure_message
                and "never changes" in failure_message,
                "Linux Samply permission guidance",
            )
        elif sys.platform == "darwin":
            require(
                "signed Godot" in failure_message
                and "will not" in failure_message,
                "macOS Samply signing guidance",
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


def verify_comparison() -> None:
    with tempfile.TemporaryDirectory() as temporary:
        root = Path(temporary)
        baseline_path, _ = write_profile_fixture(
            root / "baseline", "baseline-1"
        )
        current_path, _ = write_profile_fixture(
            root / "current", "current-1", 1.1
        )
        descriptive_inputs = [
            load_profile(baseline_path, SCHEMA, "baseline", 1),
            load_profile(current_path, SCHEMA, "current", 1),
        ]
        descriptive = create_comparison(
            descriptive_inputs, "descriptive", "descriptive-fixture"
        )
        validate_profile_comparison(
            descriptive, COMPARISON_SCHEMA, require_complete=True
        )
        require(
            descriptive["quality"] == "descriptive"
            and descriptive["noise_analysis"] is False
            and all(
                metric["noise_pct"] is None
                for span in descriptive["spans"]
                for metric in span["metrics"]
            ),
            "descriptive comparison noise contract",
        )
        print("PASS one-pair comparison: quality=descriptive noise=false")

        exploratory = copy.deepcopy(descriptive_inputs)
        for profile in exploratory:
            profile.document["selection"].update(
                {"mode": "filter", "requested": "fixture", "patterns": ["fixture"]}
            )
        try:
            create_comparison(exploratory, "descriptive", "filter-fixture")
        except ComparisonError:
            pass
        else:
            raise AssertionError("exploratory filter profiles were compared")

        interleaved_inputs = []
        for round_number, (baseline_scale, current_scale) in enumerate(
            ((0.9, 1.0), (1.0, 1.15), (1.1, 1.2)), start=1
        ):
            round_baseline, _ = write_profile_fixture(
                root / f"baseline-{round_number}",
                f"baseline-{round_number}",
                baseline_scale,
            )
            round_current, _ = write_profile_fixture(
                root / f"current-{round_number}",
                f"current-{round_number}",
                current_scale,
            )
            interleaved_inputs.extend(
                [
                    load_profile(
                        round_baseline, SCHEMA, "baseline", round_number
                    ),
                    load_profile(round_current, SCHEMA, "current", round_number),
                ]
            )
        interleaved = create_comparison(
            interleaved_inputs, "interleaved", "interleaved-fixture"
        )
        validate_profile_comparison(
            interleaved, COMPARISON_SCHEMA, require_complete=True
        )
        hot = next(span for span in interleaved["spans"] if span["name"] == "hot_zone")
        require(
            interleaved["noise_analysis"] is True
            and all(metric["noise_pct"] is not None for metric in hot["metrics"]),
            "interleaved comparison noise contract",
        )
        print("PASS three-pair comparison: quality=interleaved noise=true")

        compatibility_mutations = (
            ("platform-os", lambda doc: doc["environment"].__setitem__("os", "other-os")),
            ("platform-arch", lambda doc: doc["environment"].__setitem__("arch", "other-arch")),
            ("cpu", lambda doc: doc["environment"].__setitem__("cpu", "other-cpu")),
            ("godot", lambda doc: doc["environment"].__setitem__("godot_version", "other-godot")),
            ("rustc", lambda doc: doc["environment"].__setitem__("rustc_version", "other-rustc")),
            ("tracy", lambda doc: doc["environment"].__setitem__("tracy_version", "other-tracy")),
            ("profile", lambda doc: doc["environment"].__setitem__("cargo_profile", "release")),
            ("features", lambda doc: doc["environment"]["features"].append("other-feature")),
            ("selector", lambda doc: doc["selection"].__setitem__("requested", "other-bench")),
            ("schema", lambda doc: doc.__setitem__("$schema", "profile-spans-v2.schema.json")),
        )
        for label, mutate in compatibility_mutations:
            incompatible_path, incompatible_document = write_profile_fixture(
                root / f"incompatible-{label}", f"incompatible-{label}", 1.1
            )
            mutate(incompatible_document)
            write_json(incompatible_path, incompatible_document)
            try:
                incompatible_input = load_profile(
                    incompatible_path, SCHEMA, "current", 1
                )
                create_comparison(
                    [descriptive_inputs[0], incompatible_input],
                    "descriptive",
                    "incompatible-fixture",
                )
            except ComparisonError:
                pass
            else:
                raise AssertionError(f"{label} incompatibility was accepted")

        incompatible_cli, incompatible_document = write_profile_fixture(
            root / "incompatible-cli", "incompatible-cli", 1.1
        )
        incompatible_document["environment"]["cpu"] = "other-cpu"
        write_json(incompatible_cli, incompatible_document)
        result = subprocess.run(
            [
                sys.executable,
                str(REPOSITORY / "itest" / "compare-profiles.py"),
                str(baseline_path),
                str(incompatible_cli),
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            check=False,
        )
        require(result.returncode == 2, "incompatible profile exit code")
        print("PASS incompatible profiles: exit=2")

        incomplete_path, incomplete_document = write_profile_fixture(
            root / "incomplete", "incomplete"
        )
        incomplete_document["complete"] = False
        incomplete_document["outcome"] = "incomplete"
        incomplete_document["spans"] = []
        write_json(incomplete_path, incomplete_document)
        result = subprocess.run(
            [
                sys.executable,
                str(REPOSITORY / "itest" / "compare-profiles.py"),
                str(baseline_path),
                str(incomplete_path),
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            check=False,
        )
        require(result.returncode == 2, "incomplete profile exit code")

        missing_artifact_path, _ = write_profile_fixture(
            root / "missing-artifact", "missing-artifact"
        )
        (missing_artifact_path.parent / "capture.tracy").unlink()
        missing_artifact = subprocess.run(
            [
                sys.executable,
                str(REPOSITORY / "itest" / "compare-profiles.py"),
                str(baseline_path),
                str(missing_artifact_path),
            ],
            stdout=subprocess.PIPE,
            stderr=subprocess.STDOUT,
            text=True,
            check=False,
        )
        require(missing_artifact.returncode == 2, "missing profile artifact exit code")
        print("PASS incomplete profiles: exit=2")

        unmatched_path, unmatched_document = write_profile_fixture(
            root / "unmatched", "unmatched", 1.1
        )
        unmatched_document["spans"][0]["name"] = "renamed_hot_zone"
        write_json(unmatched_path, unmatched_document)
        unmatched = create_comparison(
            [
                descriptive_inputs[0],
                load_profile(unmatched_path, SCHEMA, "current", 1),
            ],
            "descriptive",
            "unmatched-fixture",
        )
        require(
            unmatched["summary"]["added"] == 1
            and unmatched["summary"]["removed"] == 1,
            "unmatched span identity contract",
        )
        print("PASS unmatched spans: added-and-removed")
        require(
            descriptive["benchmark_compatible"] is False
            and descriptive["disclosure"] == DISCLOSURE,
            "comparison disclosure",
        )
        print("PASS comparison disclosure: diagnostic-only")


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


def verify_compare_live(path: Path) -> None:
    document = json.loads(path.read_text(encoding="utf-8"))
    validate_profile_comparison(
        document, COMPARISON_SCHEMA, require_complete=True
    )
    expected_inputs = [
        (side, round_number)
        for round_number in range(1, 4)
        for side in ("baseline", "current")
    ]
    actual_inputs = [
        (profile["side"], profile["round"]) for profile in document["inputs"]
    ]
    require(
        document["quality"] == "interleaved"
        and document["rounds"]
        == {"baseline": 3, "current": 3, "interleaved": True}
        and actual_inputs == expected_inputs,
        "live interleaving order",
    )
    print("PASS interleaving: baseline,current x3")
    require(document["$schema"] == COMPARISON_SCHEMA_NAME, "comparison schema")
    print("PASS comparison schema: profile-comparison-v1")
    require(
        document["noise_analysis"] is True
        and all(
            [metric["kind"] for metric in span["metrics"]]
            == ["self", "inclusive", "count"]
            for span in document["spans"]
        )
        and any(
            metric["noise_pct"] is not None
            for span in document["spans"]
            if span["status"] == "matched"
            for metric in span["metrics"]
        ),
        "comparison metric and noise output",
    )
    print("PASS comparison metrics: self,inclusive,count,noise")


def verify_native_live(path: Path) -> None:
    document = json.loads(path.read_text(encoding="utf-8"))
    validate_native_summary(document, NATIVE_SCHEMA, require_complete=True)
    samples = document["samples"]
    require(samples["count"] >= 500, "native sample count")
    print("PASS samply child capture: samples>=500")
    sampling = document["sampling"]
    require(
        sampling["rate_hz"] == 1000 and sampling["reuse_threads"] is False,
        "Samply settings",
    )
    print("PASS samply settings: rate=1000 reuse-threads=false")
    require(
        document["symbols"] == {"rust": True, "godot": True},
        "native symbol coverage",
    )
    print("PASS native symbols: rust=true godot=true")
    require(samples["unknown_leaf_ratio"] <= 0.5, "unknown leaf ratio")
    print("PASS unknown leaf ratio: <=50%")

    folded = path.parent / NATIVE_ARTIFACTS["folded-stacks"]
    require(folded.read_text(encoding="utf-8").strip() != "", "folded stacks")
    print("PASS folded stacks: nonempty")
    flamegraph = path.parent / NATIVE_ARTIFACTS["flamegraph"]
    root = ElementTree.parse(flamegraph).getroot()
    require(root.tag.endswith("svg"), "flamegraph SVG root")
    require(DISCLOSURE in flamegraph.read_text(encoding="utf-8"), "flamegraph disclosure")
    print("PASS flamegraph svg: valid")
    print("PASS native-summary-v1: complete=true")
    require(
        document["benchmark_compatible"] is False
        and document["scope"]["kind"] == "whole-process",
        "native profiling disclosure",
    )
    print("PASS disclosure: benchmark_compatible=false")


def verify_workflow() -> None:
    workflow = REPOSITORY / ".github" / "workflows" / "profiling.yml"
    text = workflow.read_text(encoding="utf-8")
    event_block = text.split("on:", 1)[1].split("permissions:", 1)[0]
    require(
        "workflow_dispatch:" in event_block
        and all(
            event not in event_block
            for event in ("pull_request:", "push:", "schedule:")
        ),
        "profiling workflow triggers",
    )
    print("PASS profiling workflow: workflow_dispatch-only")
    require(
        "tracy:" in text
        and "runs-on: ubuntu-latest" in text
        and "./itest/run-profile.sh --bench" in text,
        "Linux Tracy job",
    )
    print("PASS tracy job: linux=true")
    native_job = text.split("  native:", 1)[1]
    require(
        "kernel.perf_event_paranoid=1" in native_job
        and "./itest/run-profile.sh --native --bench" in native_job,
        "native perf permission",
    )
    print("PASS native job: perf-permission-configured")
    require(
        text.count("if: always()") >= 2
        and text.count("uses: actions/upload-artifact@v4") >= 2,
        "always-uploaded profile artifacts",
    )
    print("PASS artifact upload: if=always")
    require("pull_request" not in event_block, "per-PR profiling capture")
    print("PASS per-PR capture: absent")
    changed = subprocess.run(
        [
            "git",
            "diff",
            "--name-only",
            "HEAD",
            "--",
            "godot-bevy-test/schema/itest-report-v1.schema.json",
            "godot-bevy-test/src/report.rs",
            ".github/scripts/benchmarks-compare.py",
            ".github/scripts/benchmarks-merge.py",
            ".github/workflows/benchmarks.yml",
        ],
        cwd=REPOSITORY,
        text=True,
        stdout=subprocess.PIPE,
        check=False,
    )
    require(changed.returncode == 0 and not changed.stdout.strip(), "Tier-1 mutation")
    print("PASS Tier-1 report mutation: absent")


def main() -> int:
    if len(sys.argv) not in {2, 3}:
        print(
            "usage: test_profiling.py "
            "schemas|fixtures|tools|contract|compare|fail-closed|"
            "live|compare-live|native-live|workflow [artifact.json]",
            file=sys.stderr,
        )
        return 2
    mode = sys.argv[1]
    if mode == "schemas" and len(sys.argv) == 2:
        verify_schemas()
    elif mode == "fixtures" and len(sys.argv) == 2:
        verify_fixtures()
    elif mode == "tools" and len(sys.argv) == 2:
        verify_tools()
    elif mode == "contract" and len(sys.argv) == 2:
        verify_contract()
    elif mode == "compare" and len(sys.argv) == 2:
        verify_comparison()
    elif mode == "fail-closed" and len(sys.argv) == 2:
        verify_fail_closed()
    elif mode == "live" and len(sys.argv) == 3:
        verify_live(Path(sys.argv[2]))
    elif mode == "compare-live" and len(sys.argv) == 3:
        verify_compare_live(Path(sys.argv[2]))
    elif mode == "native-live" and len(sys.argv) == 3:
        verify_native_live(Path(sys.argv[2]))
    elif mode == "workflow" and len(sys.argv) == 2:
        verify_workflow()
    else:
        print("invalid profiling test mode", file=sys.stderr)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
