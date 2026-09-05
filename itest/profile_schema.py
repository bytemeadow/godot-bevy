#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

SCHEMA_NAME = "profile-spans-v1.schema.json"
COMPARISON_SCHEMA_NAME = "profile-comparison-v1.schema.json"
NATIVE_SCHEMA_NAME = "native-summary-v1.schema.json"
DISCLOSURE = "INSTRUMENTED PROFILE — NOT BENCHMARK RESULTS"
PROFILE_ARTIFACTS = {
    "capture": "capture.tracy",
    "zones-inclusive": "zones-inclusive.tsv",
    "zones-self": "zones-self.tsv",
    "capture-log": "capture.log",
    "godot-log": "godot.log",
    "workload": "workload.json",
}
NATIVE_ARTIFACTS = {
    "profile": "profile.json.gz",
    "symbols": "profile.json.gz.syms.json",
    "folded-stacks": "stacks.folded",
    "flamegraph": "flamegraph.svg",
    "samply-log": "samply.log",
    "godot-log": "godot.log",
}


class SchemaValidationError(ValueError):
    pass


def _type_matches(value: Any, expected: str) -> bool:
    if expected == "null":
        return value is None
    if expected == "boolean":
        return isinstance(value, bool)
    if expected == "integer":
        return isinstance(value, int) and not isinstance(value, bool)
    if expected == "number":
        return isinstance(value, (int, float)) and not isinstance(value, bool)
    if expected == "string":
        return isinstance(value, str)
    if expected == "array":
        return isinstance(value, list)
    if expected == "object":
        return isinstance(value, dict)
    raise SchemaValidationError(f"unsupported schema type {expected!r}")


def _resolve_ref(root: dict[str, Any], reference: str) -> dict[str, Any]:
    if not reference.startswith("#/"):
        raise SchemaValidationError(f"unsupported schema reference {reference!r}")
    node: Any = root
    for part in reference[2:].split("/"):
        node = node[part.replace("~1", "/").replace("~0", "~")]
    if not isinstance(node, dict):
        raise SchemaValidationError(f"schema reference is not an object: {reference}")
    return node


def _validate(value: Any, rule: dict[str, Any], root: dict[str, Any], path: str) -> list[str]:
    if "$ref" in rule:
        return _validate(value, _resolve_ref(root, rule["$ref"]), root, path)

    errors: list[str] = []
    expected = rule.get("type")
    if expected is not None:
        expected_types = [expected] if isinstance(expected, str) else expected
        if not any(_type_matches(value, item) for item in expected_types):
            return [f"{path}: expected {' or '.join(expected_types)}"]

    if "const" in rule and value != rule["const"]:
        errors.append(f"{path}: expected constant {rule['const']!r}")
    if "enum" in rule and value not in rule["enum"]:
        errors.append(f"{path}: expected one of {rule['enum']!r}")

    if isinstance(value, dict):
        required = rule.get("required", [])
        for name in required:
            if name not in value:
                errors.append(f"{path}: missing required property {name!r}")
        properties = rule.get("properties", {})
        if rule.get("additionalProperties") is False:
            for name in value.keys() - properties.keys():
                errors.append(f"{path}: unexpected property {name!r}")
        for name, child in properties.items():
            if name in value:
                errors.extend(_validate(value[name], child, root, f"{path}.{name}"))

    if isinstance(value, list):
        if len(value) < rule.get("minItems", 0):
            errors.append(f"{path}: expected at least {rule['minItems']} items")
        if "maxItems" in rule and len(value) > rule["maxItems"]:
            errors.append(f"{path}: expected at most {rule['maxItems']} items")
        if "items" in rule:
            for index, item in enumerate(value):
                errors.extend(_validate(item, rule["items"], root, f"{path}[{index}]"))

    if isinstance(value, str) and len(value) < rule.get("minLength", 0):
        errors.append(f"{path}: string is shorter than {rule['minLength']}")
    if (
        isinstance(value, (int, float))
        and not isinstance(value, bool)
        and "minimum" in rule
        and value < rule["minimum"]
    ):
        errors.append(f"{path}: value is below {rule['minimum']}")
    return errors


def load_schema(path: Path) -> dict[str, Any]:
    with path.open(encoding="utf-8") as handle:
        schema = json.load(handle)
    if not isinstance(schema, dict):
        raise SchemaValidationError("schema root must be an object")
    return schema


def validate_schema_document(document: Any, schema_path: Path) -> None:
    schema = load_schema(schema_path)
    errors = _validate(document, schema, schema, "$")
    if errors:
        raise SchemaValidationError("\n".join(errors))


def validate_profile_spans(
    document: Any, schema_path: Path, *, require_complete: bool | None = None
) -> None:
    schema = load_schema(schema_path)
    errors = _validate(document, schema, schema, "$")
    if not isinstance(document, dict):
        raise SchemaValidationError("\n".join(errors))

    complete = document.get("complete")
    outcome = document.get("outcome")
    artifacts = document.get("artifacts", [])
    if isinstance(artifacts, list) and all(
        isinstance(artifact, dict) for artifact in artifacts
    ):
        artifact_kinds = [artifact.get("kind") for artifact in artifacts]
        if (
            len(artifact_kinds) != len(PROFILE_ARTIFACTS)
            or not all(isinstance(kind, str) for kind in artifact_kinds)
            or set(artifact_kinds) != set(PROFILE_ARTIFACTS)
        ):
            errors.append("$.artifacts: expected each Tracy artifact exactly once")
        for index, artifact in enumerate(artifacts):
            kind = artifact.get("kind")
            if (
                isinstance(kind, str)
                and kind in PROFILE_ARTIFACTS
                and artifact.get("path") != PROFILE_ARTIFACTS[kind]
            ):
                errors.append(f"$.artifacts[{index}].path: does not match artifact kind")
            present = artifact.get("present")
            size = artifact.get("size_bytes")
            if present is True and (not isinstance(size, int) or isinstance(size, bool)):
                errors.append(
                    f"$.artifacts[{index}].size_bytes: present artifact needs a size"
                )
            if present is False and size is not None:
                errors.append(f"$.artifacts[{index}].size_bytes: absent artifact needs null")
    if require_complete is not None and complete is not require_complete:
        errors.append(f"$.complete: expected {require_complete}")
    if complete is True:
        if outcome != "pass":
            errors.append("$.outcome: complete profiles must pass")
        selection = document.get("selection", {})
        if not isinstance(selection, dict):
            selection = {}
        benchmarks = selection.get("benchmarks", [])
        if not isinstance(benchmarks, list):
            benchmarks = []
        if selection.get("selected") != len(benchmarks) or not benchmarks:
            errors.append(
                "$.selection: complete profiles need a nonempty explicit selected list"
            )
        if not document.get("spans"):
            errors.append("$.spans: complete profiles need at least one non-marker span")
        if document.get("errors"):
            errors.append("$.errors: complete profiles cannot contain errors")
        if not isinstance(artifacts, list) or not all(
            isinstance(artifact, dict) and artifact.get("present")
            for artifact in artifacts
        ):
            errors.append("$.artifacts: complete profiles require every artifact")
    elif complete is False and outcome not in {"incomplete", "error"}:
        errors.append("$.outcome: incomplete profiles must be incomplete or error")

    workload = document.get("workload", {})
    sample_count = (
        workload.get("sample_iterations") if isinstance(workload, dict) else None
    )
    spans = document.get("spans", [])
    if not isinstance(spans, list):
        spans = []
    for span_index, span in enumerate(spans):
        if not isinstance(span, dict):
            continue
        for timing_name in ("inclusive", "self"):
            timing = span.get(timing_name, {})
            if not isinstance(timing, dict):
                continue
            samples = timing.get("per_sample", [])
            if isinstance(sample_count, int) and isinstance(samples, list):
                expected_iterations = list(range(sample_count))
                actual_iterations = [
                    sample.get("iteration") if isinstance(sample, dict) else None
                    for sample in samples
                ]
                if actual_iterations != expected_iterations:
                    errors.append(
                        f"$.spans[{span_index}].{timing_name}.per_sample: "
                        "iterations must be contiguous"
                    )
            for percentile, minimum in (("p95", 20), ("p99", 100)):
                quantile = timing.get(f"{percentile}_ns")
                reason = timing.get(f"{percentile}_reason")
                occurrences = timing.get("occurrence_count")
                if isinstance(occurrences, int) and occurrences < minimum:
                    if quantile is not None or not isinstance(reason, str) or not reason:
                        errors.append(
                            f"$.spans[{span_index}].{timing_name}.{percentile}: "
                            f"requires a null value and reason below {minimum} occurrences"
                        )
                elif isinstance(occurrences, int) and occurrences >= minimum:
                    if quantile is None or reason is not None:
                        errors.append(
                            f"$.spans[{span_index}].{timing_name}.{percentile}: "
                            f"requires a value and null reason at {minimum} occurrences"
                        )

    if errors:
        raise SchemaValidationError("\n".join(errors))


def validate_profile_comparison(
    document: Any, schema_path: Path, *, require_complete: bool | None = None
) -> None:
    schema = load_schema(schema_path)
    errors = _validate(document, schema, schema, "$")
    if not isinstance(document, dict):
        raise SchemaValidationError("\n".join(errors))

    complete = document.get("complete")
    if require_complete is not None and complete is not require_complete:
        errors.append(f"$.complete: expected {require_complete}")
    outcome = document.get("outcome")
    quality = document.get("quality")
    rounds = document.get("rounds", {})
    inputs = document.get("inputs", [])
    spans = document.get("spans", [])
    artifacts = document.get("artifacts", [])

    if complete is True:
        if outcome != "pass" or document.get("errors"):
            errors.append("$: complete comparisons must pass without errors")
        if not isinstance(rounds, dict) or not isinstance(inputs, list):
            errors.append("$: comparison rounds and inputs are invalid")
        else:
            baseline_rounds = rounds.get("baseline")
            current_rounds = rounds.get("current")
            valid_round_counts = (
                isinstance(baseline_rounds, int)
                and not isinstance(baseline_rounds, bool)
                and isinstance(current_rounds, int)
                and not isinstance(current_rounds, bool)
            )
            expected_inputs = (
                [
                    (side, round_number)
                    for round_number in range(1, baseline_rounds + 1)
                    for side in ("baseline", "current")
                ]
                if valid_round_counts and baseline_rounds == current_rounds
                else []
            )
            actual_inputs = [
                (item.get("side"), item.get("round"))
                for item in inputs
                if isinstance(item, dict)
            ]
            if quality == "descriptive":
                if (
                    baseline_rounds != 1
                    or current_rounds != 1
                    or rounds.get("interleaved") is not False
                    or document.get("noise_analysis") is not False
                    or document.get("significance_sigma") is not None
                    or actual_inputs != [("baseline", 1), ("current", 1)]
                ):
                    errors.append(
                        "$: descriptive comparisons require one pair and no noise analysis"
                    )
            elif quality == "interleaved":
                if (
                    not isinstance(baseline_rounds, int)
                    or baseline_rounds < 3
                    or current_rounds != baseline_rounds
                    or rounds.get("interleaved") is not True
                    or document.get("noise_analysis") is not True
                    or document.get("significance_sigma") != 2
                    or actual_inputs != expected_inputs
                ):
                    errors.append(
                        "$: interleaved comparisons require alternating pairs and "
                        "at least three rounds"
                    )
            if not valid_round_counts:
                errors.append("$.rounds: baseline and current must be integer counts")
            elif len(inputs) != baseline_rounds + current_rounds:
                errors.append("$.inputs: count does not match rounds")

        if not isinstance(spans, list) or not spans:
            errors.append("$.spans: complete comparisons require span results")
        else:
            counts = {"matched": 0, "added": 0, "removed": 0}
            for index, span in enumerate(spans):
                if not isinstance(span, dict):
                    continue
                status = span.get("status")
                if isinstance(status, str) and status in counts:
                    counts[status] += 1
                metrics = span.get("metrics", [])
                metric_items = metrics if isinstance(metrics, list) else []
                kinds = [
                    metric.get("kind")
                    for metric in metric_items
                    if isinstance(metric, dict)
                ]
                if kinds != ["self", "inclusive", "count"]:
                    errors.append(
                        f"$.spans[{index}].metrics: expected self, inclusive, count"
                    )
                for metric_index, metric in enumerate(metric_items):
                    if not isinstance(metric, dict):
                        continue
                    baseline = metric.get("baseline")
                    current = metric.get("current")
                    assessment = metric.get("assessment")
                    metric_path = f"$.spans[{index}].metrics[{metric_index}]"
                    expected_unit = (
                        "calls" if metric.get("kind") == "count" else "nanoseconds"
                    )
                    if metric.get("unit") != expected_unit:
                        errors.append(f"{metric_path}.unit: inconsistent with metric kind")
                    if status == "matched":
                        if baseline is None or current is None:
                            errors.append(f"{metric_path}: matched metrics need both values")
                        expected_assessments = (
                            {"descriptive"}
                            if quality == "descriptive"
                            else {"higher", "lower", "within-noise"}
                        )
                        if assessment not in expected_assessments:
                            errors.append(
                                f"{metric_path}.assessment: inconsistent with quality"
                            )
                        if quality == "descriptive" and metric.get("noise_pct") is not None:
                            errors.append(
                                f"{metric_path}.noise_pct: descriptive metrics have no noise"
                            )
                    elif status == "added":
                        if (
                            baseline is not None
                            or current is None
                            or assessment != "added"
                            or metric.get("change_pct") is not None
                            or metric.get("noise_pct") is not None
                        ):
                            errors.append(f"{metric_path}: invalid added metric")
                    elif status == "removed":
                        if (
                            baseline is None
                            or current is not None
                            or assessment != "removed"
                            or metric.get("change_pct") is not None
                            or metric.get("noise_pct") is not None
                        ):
                            errors.append(f"{metric_path}: invalid removed metric")
            summary = document.get("summary", {})
            if not isinstance(summary, dict) or any(
                summary.get(name) != count for name, count in counts.items()
            ):
                errors.append("$.summary: span status counts do not match")

        if (
            not isinstance(artifacts, list)
            or not isinstance(inputs, list)
            or len(artifacts) != len(inputs)
        ):
            errors.append("$.artifacts: expected one artifact record per input")
        elif not all(
            isinstance(artifact, dict)
            and artifact.get("present") is True
            and artifact.get("size_bytes") is not None
            for artifact in artifacts
        ):
            errors.append("$.artifacts: complete comparisons require every input artifact")
        elif any(
            artifact.get("kind") != f"{profile.get('side')}-profile"
            or artifact.get("path") != profile.get("path")
            for artifact, profile in zip(artifacts, inputs)
            if isinstance(artifact, dict) and isinstance(profile, dict)
        ):
            errors.append("$.artifacts: artifact records must match comparison inputs")
    elif complete is False and outcome not in {"incomplete", "error"}:
        errors.append("$.outcome: incomplete comparisons must be incomplete or error")

    if errors:
        raise SchemaValidationError("\n".join(errors))


def validate_native_summary(
    document: Any, schema_path: Path, *, require_complete: bool | None = None
) -> None:
    schema = load_schema(schema_path)
    errors = _validate(document, schema, schema, "$")
    if not isinstance(document, dict):
        raise SchemaValidationError("\n".join(errors))

    complete = document.get("complete")
    if require_complete is not None and complete is not require_complete:
        errors.append(f"$.complete: expected {require_complete}")
    outcome = document.get("outcome")
    artifacts = document.get("artifacts", [])
    if isinstance(artifacts, list) and all(
        isinstance(artifact, dict) for artifact in artifacts
    ):
        kinds = [artifact.get("kind") for artifact in artifacts]
        if (
            len(kinds) != len(NATIVE_ARTIFACTS)
            or not all(isinstance(kind, str) for kind in kinds)
            or set(kinds) != set(NATIVE_ARTIFACTS)
        ):
            errors.append("$.artifacts: expected each native artifact exactly once")
        for index, artifact in enumerate(artifacts):
            kind = artifact.get("kind")
            if isinstance(kind, str) and kind in NATIVE_ARTIFACTS:
                if artifact.get("path") != NATIVE_ARTIFACTS[kind]:
                    errors.append(f"$.artifacts[{index}].path: does not match artifact kind")
            present = artifact.get("present")
            size = artifact.get("size_bytes")
            if present is True and (not isinstance(size, int) or isinstance(size, bool)):
                errors.append(f"$.artifacts[{index}].size_bytes: present artifact needs a size")
            if present is False and size is not None:
                errors.append(f"$.artifacts[{index}].size_bytes: absent artifact needs null")

    if complete is True:
        if outcome != "pass" or document.get("errors"):
            errors.append("$: complete native profiles must pass without errors")
        samples = document.get("samples", {})
        sample_count = samples.get("count") if isinstance(samples, dict) else None
        unknown_count = (
            samples.get("unknown_leaf_count") if isinstance(samples, dict) else None
        )
        unknown_ratio = (
            samples.get("unknown_leaf_ratio") if isinstance(samples, dict) else None
        )
        if (
            not isinstance(sample_count, int)
            or isinstance(sample_count, bool)
            or sample_count < 500
        ):
            errors.append(
                "$.samples.count: complete native profiles require at least 500 samples"
            )
        if (
            not isinstance(unknown_count, int)
            or not isinstance(unknown_ratio, (int, float))
            or unknown_ratio > 0.5
            or not isinstance(sample_count, int)
            or sample_count <= 0
            or abs(unknown_ratio - unknown_count / sample_count) > 1e-9
        ):
            errors.append("$.samples: unknown leaf ratio is invalid or exceeds 50%")
        symbols = document.get("symbols", {})
        if (
            not isinstance(symbols, dict)
            or symbols.get("rust") is not True
            or symbols.get("godot") is not True
        ):
            errors.append(
                "$.symbols: complete native profiles require Rust and Godot symbols"
            )
        sampling = document.get("sampling", {})
        observed = (
            sampling.get("observed_wall_seconds")
            if isinstance(sampling, dict)
            else None
        )
        minimum = (
            sampling.get("minimum_workload_seconds")
            if isinstance(sampling, dict)
            else None
        )
        if (
            not isinstance(sampling, dict)
            or sampling.get("rate_hz") != 1000
            or sampling.get("reuse_threads") is not False
            or not isinstance(observed, (int, float))
            or isinstance(observed, bool)
            or not isinstance(minimum, int)
            or isinstance(minimum, bool)
            or observed < minimum
        ):
            errors.append("$.sampling: native capture settings or duration are invalid")
        if not isinstance(artifacts, list) or not all(
            isinstance(artifact, dict) and artifact.get("present") is True
            for artifact in artifacts
        ):
            errors.append("$.artifacts: complete native profiles require every artifact")
    elif complete is False and outcome not in {"incomplete", "error"}:
        errors.append("$.outcome: incomplete native profiles must be incomplete or error")

    if errors:
        raise SchemaValidationError("\n".join(errors))


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate profile-spans-v1 JSON")
    parser.add_argument("document", type=Path)
    parser.add_argument(
        "--schema",
        type=Path,
        default=Path(__file__).resolve().parents[1]
        / "itest"
        / "schema"
        / SCHEMA_NAME,
    )
    parser.add_argument("--complete", action="store_true")
    args = parser.parse_args()

    try:
        with args.document.open(encoding="utf-8") as handle:
            document = json.load(handle)
        validate_profile_spans(
            document,
            args.schema,
            require_complete=True if args.complete else None,
        )
    except (OSError, json.JSONDecodeError, SchemaValidationError) as error:
        print(error)
        return 2
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
