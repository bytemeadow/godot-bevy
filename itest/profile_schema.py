#!/usr/bin/env python3
from __future__ import annotations

import argparse
import json
from pathlib import Path
from typing import Any

SCHEMA_NAME = "profile-spans-v1.schema.json"
DISCLOSURE = "INSTRUMENTED PROFILE — NOT BENCHMARK RESULTS"
PROFILE_ARTIFACTS = {
    "capture": "capture.tracy",
    "zones-inclusive": "zones-inclusive.tsv",
    "zones-self": "zones-self.tsv",
    "capture-log": "capture.log",
    "godot-log": "godot.log",
    "workload": "workload.json",
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


def main() -> int:
    parser = argparse.ArgumentParser(description="Validate profile-spans-v1 JSON")
    parser.add_argument("document", type=Path)
    parser.add_argument(
        "--schema",
        type=Path,
        default=Path(__file__).resolve().parents[1]
        / "godot-bevy-test"
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
