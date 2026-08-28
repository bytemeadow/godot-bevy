#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
import math
import os
import statistics
import sys
from dataclasses import dataclass
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

from profile_orchestrator import ProfileFailure, format_duration, make_run_id
from profile_schema import (
    COMPARISON_SCHEMA_NAME,
    DISCLOSURE,
    SCHEMA_NAME,
    SchemaValidationError,
    validate_profile_comparison,
    validate_profile_spans,
)

SIGNIFICANCE_SIGMA = 2
METRICS = ("self", "inclusive", "count")
SpanIdentity = tuple[str, str, int]


class ComparisonError(ValueError):
    pass


@dataclass(frozen=True)
class ProfileInput:
    side: str
    round: int
    path: Path
    document: dict[str, Any]


def _load_json(path: Path) -> Any:
    try:
        with path.open(encoding="utf-8") as handle:
            return json.load(handle)
    except (OSError, json.JSONDecodeError) as error:
        raise ComparisonError(f"failed to read {path}: {error}") from error


def load_profile(path: Path, schema_path: Path, side: str, round_number: int) -> ProfileInput:
    document = _load_json(path)
    if not isinstance(document, dict):
        raise ComparisonError(f"{path}: profile root must be an object")
    if document.get("$schema") != SCHEMA_NAME or document.get("schema_version") != 1:
        raise ComparisonError(f"{path}: incompatible profile schema")
    if document.get("complete") is not True:
        raise ComparisonError(f"{path}: incomplete profiles cannot be compared")
    try:
        validate_profile_spans(document, schema_path, require_complete=True)
    except SchemaValidationError as error:
        raise ComparisonError(f"{path}: invalid profile: {error}") from error
    for artifact in document["artifacts"]:
        artifact_path = path.parent / artifact["path"]
        try:
            size = artifact_path.stat().st_size
        except OSError as error:
            raise ComparisonError(
                f"{path}: profile artifact is missing: {artifact['path']}"
            ) from error
        if size != artifact["size_bytes"]:
            raise ComparisonError(
                f"{path}: profile artifact size changed: {artifact['path']}"
            )
    return ProfileInput(side, round_number, path.resolve(), document)


def _selector(document: dict[str, Any]) -> dict[str, Any]:
    selection = document["selection"]
    return {
        "mode": selection["mode"],
        "requested": selection["requested"],
        "patterns": selection["patterns"],
        "benchmarks": selection["benchmarks"],
    }


def compatibility(document: dict[str, Any]) -> dict[str, Any]:
    environment = document["environment"]
    return {
        "profile_schema": document["$schema"],
        "profile_schema_version": document["schema_version"],
        "platform": {"os": environment["os"], "arch": environment["arch"]},
        "cpu": environment["cpu"],
        "versions": {
            "godot": environment["godot_version"],
            "rustc": environment["rustc_version"],
            "tracy": environment["tracy_version"],
        },
        "cargo_profile": environment["cargo_profile"],
        "features": sorted(environment["features"]),
        "selector": _selector(document),
    }


def ensure_compatible(inputs: list[ProfileInput]) -> dict[str, Any]:
    if not inputs:
        raise ComparisonError("comparison needs profile inputs")
    expected = compatibility(inputs[0].document)
    selector = expected["selector"]
    if selector["mode"] != "exact" or len(selector["benchmarks"]) != 1:
        raise ComparisonError("comparisons require one exact benchmark selector")
    labels = {
        "profile_schema": "schema",
        "profile_schema_version": "schema",
        "platform": "platform",
        "cpu": "CPU",
        "versions": "tool versions",
        "cargo_profile": "Cargo profile",
        "features": "feature set",
        "selector": "selector",
    }
    for profile in inputs[1:]:
        actual = compatibility(profile.document)
        for field, label in labels.items():
            if actual[field] != expected[field]:
                raise ComparisonError(
                    f"incompatible profiles: {label} differs in {profile.path}"
                )
    return expected


def span_identity(span: dict[str, Any]) -> SpanIdentity:
    return (
        span["name"],
        span["source_file"],
        span["source_line"],
    )


def _median(values: list[float]) -> float:
    if not values:
        raise ComparisonError("cannot summarize an empty value list")
    return float(statistics.median(values))


def span_metric(span: dict[str, Any], kind: str) -> float:
    timing = span["self" if kind == "count" else kind]
    field = "normalized_count" if kind == "count" else "normalized_total_ns"
    return _median([float(sample[field]) for sample in timing["per_sample"]])


def _relative_stderr(values: list[float]) -> float | None:
    if len(values) < 2:
        return None
    midpoint = statistics.median(values)
    if midpoint == 0:
        return None
    return statistics.stdev(values) / math.sqrt(len(values)) / abs(midpoint)


def change_noise(baseline: list[float], current: list[float]) -> float | None:
    errors = [
        value
        for value in (_relative_stderr(baseline), _relative_stderr(current))
        if value is not None
    ]
    if not errors:
        return None
    return math.sqrt(sum(value * value for value in errors)) * 100


def _change_percent(baseline: float, current: float) -> float | None:
    if baseline == 0:
        return None
    return (current - baseline) / baseline * 100


def _assessment(
    quality: str,
    baseline: float,
    current: float,
    change_pct: float | None,
    noise_pct: float | None,
) -> str:
    if quality == "descriptive":
        return "descriptive"
    if baseline == current:
        return "within-noise"
    if change_pct is None or noise_pct is None:
        return "higher" if current > baseline else "lower"
    if abs(change_pct) <= SIGNIFICANCE_SIGMA * noise_pct:
        return "within-noise"
    return "higher" if change_pct > 0 else "lower"


def _metric(
    kind: str,
    status: str,
    baseline_runs: list[float],
    current_runs: list[float],
    quality: str,
) -> dict[str, Any]:
    baseline = _median(baseline_runs) if baseline_runs else None
    current = _median(current_runs) if current_runs else None
    if status == "added":
        assessment = "added"
    elif status == "removed":
        assessment = "removed"
    else:
        assessment = _assessment(
            quality,
            baseline or 0,
            current or 0,
            _change_percent(baseline or 0, current or 0),
            change_noise(baseline_runs, current_runs) if quality == "interleaved" else None,
        )
    return {
        "kind": kind,
        "unit": "calls" if kind == "count" else "nanoseconds",
        "baseline": baseline,
        "current": current,
        "change_pct": (
            _change_percent(baseline, current)
            if baseline is not None and current is not None
            else None
        ),
        "noise_pct": (
            change_noise(baseline_runs, current_runs)
            if status == "matched" and quality == "interleaved"
            else None
        ),
        "assessment": assessment,
    }


def _side_values(
    inputs: list[ProfileInput], side: str
) -> tuple[list[ProfileInput], dict[SpanIdentity, dict[str, list[float]]]]:
    side_inputs = [profile for profile in inputs if profile.side == side]
    profile_spans = []
    identities = set()
    for profile in side_inputs:
        spans = {}
        for span in profile.document["spans"]:
            identity = span_identity(span)
            if identity in spans:
                raise ComparisonError(f"{profile.path}: duplicate span identity {identity}")
            spans[identity] = span
            identities.add(identity)
        profile_spans.append(spans)
    values = {
        identity: {kind: [] for kind in METRICS}
        for identity in identities
    }
    for spans in profile_spans:
        for identity in identities:
            span = spans.get(identity)
            for kind in METRICS:
                values[identity][kind].append(
                    span_metric(span, kind) if span is not None else 0.0
                )
    return side_inputs, values


def _artifact(profile: ProfileInput) -> dict[str, Any]:
    try:
        size = profile.path.stat().st_size
        present = True
    except OSError:
        size = None
        present = False
    return {
        "kind": f"{profile.side}-profile",
        "path": str(profile.path),
        "present": present,
        "size_bytes": size,
        "metadata": {"round": profile.round, "run_id": profile.document["run_id"]},
    }


def create_comparison(
    inputs: list[ProfileInput], quality: str, run_id: str
) -> dict[str, Any]:
    if quality not in {"descriptive", "interleaved"}:
        raise ComparisonError(f"unsupported comparison quality {quality!r}")
    compatible = ensure_compatible(inputs)
    baseline_inputs, baseline = _side_values(inputs, "baseline")
    current_inputs, current = _side_values(inputs, "current")
    if quality == "descriptive" and (len(baseline_inputs), len(current_inputs)) != (1, 1):
        raise ComparisonError("descriptive comparisons require one profile per side")
    if quality == "interleaved" and (
        len(baseline_inputs) < 3 or len(current_inputs) != len(baseline_inputs)
    ):
        raise ComparisonError("interleaved comparisons require at least three profiles per side")

    all_identities = sorted(baseline.keys() | current.keys())
    benchmark = compatible["selector"]["benchmarks"][0]
    spans = []
    summary = {"matched": 0, "added": 0, "removed": 0}
    for identity in all_identities:
        if identity not in baseline:
            status = "added"
        elif identity not in current:
            status = "removed"
        else:
            status = "matched"
        summary[status] += 1
        name, source_file, source_line = identity
        spans.append(
            {
                "benchmark": benchmark,
                "name": name,
                "source_file": source_file,
                "source_line": source_line,
                "status": status,
                "metrics": [
                    _metric(
                        kind,
                        status,
                        baseline.get(identity, {}).get(kind, []),
                        current.get(identity, {}).get(kind, []),
                        quality,
                    )
                    for kind in METRICS
                ],
            }
        )

    warnings = []
    for profile in inputs:
        for warning in profile.document["warnings"]:
            warnings.append(
                {
                    "kind": warning["kind"],
                    "message": warning["message"],
                    "metadata": {
                        **warning["metadata"],
                        "side": profile.side,
                        "round": profile.round,
                    },
                }
            )

    rounds = len(baseline_inputs)
    document = {
        "$schema": COMPARISON_SCHEMA_NAME,
        "schema_version": 1,
        "run_id": run_id,
        "evidence_kind": "tracy-profile-comparison",
        "benchmark_compatible": False,
        "disclosure": DISCLOSURE,
        "complete": True,
        "outcome": "pass",
        "generated_at": datetime.now(timezone.utc).isoformat(),
        "quality": quality,
        "noise_analysis": quality == "interleaved",
        "significance_sigma": SIGNIFICANCE_SIGMA if quality == "interleaved" else None,
        "compatibility": compatible,
        "rounds": {
            "baseline": rounds,
            "current": len(current_inputs),
            "interleaved": quality == "interleaved",
        },
        "inputs": [
            {
                "side": profile.side,
                "round": profile.round,
                "run_id": profile.document["run_id"],
                "path": str(profile.path),
                "metadata": {},
            }
            for profile in inputs
        ],
        "spans": spans,
        "summary": summary,
        "warnings": warnings,
        "errors": [],
        "artifacts": [_artifact(profile) for profile in inputs],
        "metadata": {
            "span_identity": ["emitted_name", "source_file", "source_line"],
            "process_statistic": "median of per-profile sample medians",
            "noise_formula": "quadrature of relative standard errors",
        },
    }
    return document


def write_comparison(document: dict[str, Any], output: Path, schema_path: Path) -> None:
    validate_profile_comparison(document, schema_path, require_complete=True)
    output.parent.mkdir(parents=True, exist_ok=True)
    temporary = output.with_name(f".{output.name}.tmp")
    with temporary.open("w", encoding="utf-8") as handle:
        json.dump(document, handle, indent=2, sort_keys=True)
        handle.write("\n")
    os.replace(temporary, output)


def _display_metric(metric: dict[str, Any]) -> str:
    if metric["assessment"] in {"added", "removed"}:
        return metric["assessment"]
    change = metric["change_pct"]
    if change is None:
        return metric["assessment"]
    noise = metric["noise_pct"]
    suffix = f" ±{noise:.1f}%" if noise is not None else ""
    return f"{change:+.1f}% {metric['assessment']}{suffix}"


def print_table(document: dict[str, Any]) -> None:
    print(f"Comparison quality: {document['quality']}; diagnostic-only evidence.")
    print(f"{'baseline self':>14} {'current self':>14} {'change':>29}  span")
    ordered = sorted(
        document["spans"],
        key=lambda span: next(
            metric["current"] or metric["baseline"] or 0
            for metric in span["metrics"]
            if metric["kind"] == "self"
        ),
        reverse=True,
    )
    for span in ordered:
        metric = next(metric for metric in span["metrics"] if metric["kind"] == "self")
        baseline = format_duration(metric["baseline"]) if metric["baseline"] is not None else "-"
        current = format_duration(metric["current"]) if metric["current"] is not None else "-"
        location = f"{span['source_file']}:{span['source_line']}"
        print(
            f"{baseline:>14} {current:>14} {_display_metric(metric):>29}  "
            f"{span['benchmark']} :: {span['name']} ({location})"
        )


def create_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Descriptively compare two instrumented Tracy span profiles."
    )
    parser.add_argument("baseline", type=Path)
    parser.add_argument("current", type=Path)
    parser.add_argument("--output", type=Path, help="write profile-comparison-v1 JSON")
    return parser


def main() -> int:
    args = create_parser().parse_args()
    repository = Path(__file__).resolve().parents[1]
    profile_schema = repository / "godot-bevy-test" / "schema" / SCHEMA_NAME
    comparison_schema = (
        repository / "godot-bevy-test" / "schema" / COMPARISON_SCHEMA_NAME
    )
    print(DISCLOSURE)
    try:
        inputs = [
            load_profile(args.baseline, profile_schema, "baseline", 1),
            load_profile(args.current, profile_schema, "current", 1),
        ]
        run_id, _ = make_run_id(repository)
        document = create_comparison(inputs, "descriptive", run_id)
        validate_profile_comparison(document, comparison_schema, require_complete=True)
        if args.output is not None:
            write_comparison(document, args.output, comparison_schema)
        print_table(document)
    except (ComparisonError, ProfileFailure, SchemaValidationError, OSError) as error:
        print(f"Comparison failed: {error}", file=sys.stderr)
        return 2
    if args.output is not None:
        print(f"Comparison complete: {args.output}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
