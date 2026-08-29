#!/usr/bin/env python3
from __future__ import annotations

import json
import os
import re
import tempfile
import uuid
from datetime import datetime, timezone
from pathlib import Path
from typing import Any

SCHEMA_NAME = "qualification-v1.schema.json"
SCHEMA_VERSION = 1
SCHEMA_KEYWORDS = {
    "$defs",
    "$id",
    "$ref",
    "$schema",
    "additionalProperties",
    "const",
    "description",
    "enum",
    "items",
    "minItems",
    "minLength",
    "minimum",
    "pattern",
    "properties",
    "required",
    "title",
    "type",
    "uniqueItems",
}


class QualificationValidationError(ValueError):
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
    raise QualificationValidationError(f"unsupported schema type {expected!r}")


def _resolve_ref(root: dict[str, Any], reference: str) -> dict[str, Any]:
    if not reference.startswith("#/"):
        raise QualificationValidationError(f"unsupported schema reference {reference!r}")
    node: Any = root
    try:
        for part in reference[2:].split("/"):
            node = node[part.replace("~1", "/").replace("~0", "~")]
    except (KeyError, TypeError) as error:
        raise QualificationValidationError(
            f"unresolved schema reference {reference!r}"
        ) from error
    if not isinstance(node, dict):
        raise QualificationValidationError(
            f"schema reference is not an object: {reference}"
        )
    return node


def _validate(
    value: Any,
    rule: dict[str, Any],
    root: dict[str, Any],
    path: str,
) -> list[str]:
    unsupported = set(rule) - SCHEMA_KEYWORDS
    if unsupported:
        raise QualificationValidationError(
            f"{path}: unsupported schema keywords {sorted(unsupported)!r}"
        )
    if "$ref" in rule:
        return _validate(value, _resolve_ref(root, rule["$ref"]), root, path)

    errors: list[str] = []
    expected = rule.get("type")
    if expected is not None:
        expected_types = [expected] if isinstance(expected, str) else expected
        if not isinstance(expected_types, list) or not all(
            isinstance(item, str) for item in expected_types
        ):
            raise QualificationValidationError(f"{path}: invalid schema type rule")
        if not any(_type_matches(value, item) for item in expected_types):
            return [f"{path}: expected {' or '.join(expected_types)}"]

    if "const" in rule and value != rule["const"]:
        errors.append(f"{path}: expected constant {rule['const']!r}")
    if "enum" in rule and value not in rule["enum"]:
        errors.append(f"{path}: expected one of {rule['enum']!r}")

    if isinstance(value, dict):
        required = rule.get("required", [])
        for key in required:
            if key not in value:
                errors.append(f"{path}: missing required property {key!r}")
        properties = rule.get("properties", {})
        for key, child in value.items():
            if key in properties:
                errors.extend(_validate(child, properties[key], root, f"{path}.{key}"))
            elif rule.get("additionalProperties") is False:
                errors.append(f"{path}: unexpected property {key!r}")
            elif isinstance(rule.get("additionalProperties"), dict):
                errors.extend(
                    _validate(
                        child,
                        rule["additionalProperties"],
                        root,
                        f"{path}.{key}",
                    )
                )

    if isinstance(value, list):
        if len(value) < rule.get("minItems", 0):
            errors.append(f"{path}: expected at least {rule['minItems']} items")
        if rule.get("uniqueItems"):
            canonical = [
                json.dumps(item, sort_keys=True, separators=(",", ":")) for item in value
            ]
            if len(canonical) != len(set(canonical)):
                errors.append(f"{path}: expected unique items")
        item_rule = rule.get("items")
        if item_rule is not None:
            for index, item in enumerate(value):
                errors.extend(_validate(item, item_rule, root, f"{path}[{index}]"))

    if isinstance(value, str):
        if len(value) < rule.get("minLength", 0):
            errors.append(f"{path}: string is too short")
        pattern = rule.get("pattern")
        if pattern is not None and re.search(pattern, value) is None:
            errors.append(f"{path}: does not match {pattern!r}")

    if isinstance(value, (int, float)) and not isinstance(value, bool):
        minimum = rule.get("minimum")
        if minimum is not None and value < minimum:
            errors.append(f"{path}: expected value >= {minimum}")

    return errors


def _duplicates(values: list[Any]) -> bool:
    return len(values) != len(set(values))


def _semantic_errors(document: dict[str, Any]) -> list[str]:
    errors: list[str] = []
    complete = document.get("complete")
    outcome = document.get("outcome")
    if complete is True and outcome in {"incomplete", "error"}:
        errors.append(f"$: a complete run cannot have outcome {outcome!r}")
    if complete is False and outcome in {"pass", "fail", "skip"}:
        errors.append(f"$: outcome {outcome!r} requires complete=true")

    kind = document.get("evidence_kind")
    mutants = document.get("mutants", [])
    faults = document.get("faults", [])
    checks = document.get("checks", [])
    if kind in {"mutation-run", "mutation-baseline"} and (faults or checks):
        errors.append(f"$: {kind} cannot contain fault or aggregate records")
    if kind == "fault-pack" and (mutants or checks):
        errors.append("$: fault-pack cannot contain mutant or aggregate records")
    if kind == "aggregate" and (mutants or faults):
        errors.append("$: aggregate cannot contain mutant or fault records")

    mutant_ids = [record.get("id") for record in mutants]
    if _duplicates(mutant_ids):
        errors.append("$.mutants: duplicate stable identity")
    fault_ids = [(record.get("id"), record.get("profile")) for record in faults]
    if _duplicates(fault_ids):
        errors.append("$.faults: duplicate fault/profile identity")
    check_ids = [record.get("id") for record in checks]
    if _duplicates(check_ids):
        errors.append("$.checks: duplicate check identity")
    artifact_kinds = [record.get("kind") for record in document.get("artifacts", [])]
    if _duplicates(artifact_kinds):
        errors.append("$.artifacts: duplicate artifact kind")

    for index, artifact in enumerate(document.get("artifacts", [])):
        if artifact.get("present") is True and artifact.get("size_bytes") is None:
            errors.append(
                f"$.artifacts[{index}]: present artifact needs size_bytes"
            )

    expected_check_exits = {"pass": 0, "skip": 0, "fail": 1, "error": 2}
    for index, check in enumerate(checks):
        if check.get("exit_code") != expected_check_exits.get(check.get("outcome")):
            errors.append(f"$.checks[{index}]: outcome and exit_code conflict")

    for index, fault in enumerate(faults):
        failed = set(fault.get("failed_tests", []))
        killers = set(fault.get("killer_tests", []))
        matched = fault.get("matched_signatures", [])
        if fault.get("outcome") == "killed" and (not failed.intersection(killers) or not matched):
            errors.append(f"$.faults[{index}]: killed fault lacks attributable failure")
        if fault.get("outcome") == "survived" and (failed or matched):
            errors.append(f"$.faults[{index}]: survived fault contains failure evidence")

    if complete is True and document.get("errors"):
        errors.append("$: complete evidence cannot contain errors")

    if complete is True and outcome != "skip":
        summary = document.get("summary", {})
        actual_summary = tuple(
            summary.get(key)
            for key in ("total", "passed", "failed", "invalid", "skipped")
        )
        expected_summary: tuple[int, int, int, int, int] | None = None
        if kind in {"mutation-run", "mutation-baseline"}:
            expected_summary = (
                len(mutants),
                sum(record.get("outcome") == "caught" for record in mutants),
                sum(record.get("outcome") == "missed" for record in mutants),
                sum(record.get("outcome") == "timeout" for record in mutants),
                sum(record.get("outcome") == "unviable" for record in mutants),
            )
        elif kind == "fault-pack":
            expected_summary = (
                len(faults),
                sum(record.get("outcome") == "killed" for record in faults),
                sum(record.get("outcome") == "survived" for record in faults),
                sum(record.get("outcome") == "invalid" for record in faults),
                0,
            )
        elif checks:
            expected_summary = (
                len(checks),
                sum(record.get("outcome") == "pass" for record in checks),
                sum(record.get("outcome") == "fail" for record in checks),
                sum(record.get("outcome") == "error" for record in checks),
                sum(record.get("outcome") == "skip" for record in checks),
            )
        if expected_summary is not None and actual_summary != expected_summary:
            errors.append("$.summary: result counts do not match normalized records")

        if kind in {"mutation-run", "mutation-baseline"}:
            missed = any(record.get("outcome") == "missed" for record in mutants)
            timeout = any(record.get("outcome") == "timeout" for record in mutants)
            if outcome == "pass" and (missed or timeout):
                errors.append("$: passing mutation evidence contains missed or timed-out mutants")
            if outcome == "fail" and (not missed or timeout):
                errors.append("$: failing mutation evidence does not describe only survivors")
        elif kind == "fault-pack":
            survivors = any(record.get("outcome") == "survived" for record in faults)
            invalid = any(record.get("outcome") == "invalid" for record in faults)
            if outcome == "pass" and (not faults or survivors or invalid):
                errors.append("$: passing fault evidence contains non-kills")
            if outcome == "fail" and (not survivors or invalid):
                errors.append("$: failing fault evidence does not describe only survivors")
        elif checks:
            check_outcomes = {record.get("outcome") for record in checks}
            if outcome == "pass" and check_outcomes != {"pass"}:
                errors.append("$: passing evidence contains a non-passing check")
            if outcome == "fail" and (
                not check_outcomes.intersection({"fail", "skip"})
                or "error" in check_outcomes
            ):
                errors.append("$: failing evidence has no qualification failure")

    if outcome == "skip" and (mutants or faults or checks or document.get("errors")):
        errors.append("$: skipped evidence cannot contain results or errors")

    try:
        generated_at = str(document.get("generated_at", "")).replace("Z", "+00:00")
        parsed = datetime.fromisoformat(generated_at)
        if parsed.tzinfo is None:
            raise ValueError
    except ValueError:
        errors.append("$.generated_at: expected an RFC 3339 timestamp with timezone")
    return errors


def validate_qualification(
    document: Any,
    schema_path: Path,
    *,
    require_complete: bool = False,
) -> None:
    validate_json_schema(document, schema_path)
    errors: list[str] = []
    if isinstance(document, dict):
        errors.extend(_semantic_errors(document))
        if require_complete and (
            document.get("complete") is not True
            or document.get("outcome") in {"incomplete", "error"}
        ):
            errors.append("$: complete non-error evidence required")
    elif require_complete:
        errors.append("$: complete evidence must be an object")
    if errors:
        raise QualificationValidationError("; ".join(errors))


def validate_json_schema(document: Any, schema_path: Path) -> None:
    try:
        schema = json.loads(schema_path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise QualificationValidationError(
            f"could not load qualification schema {schema_path}: {error}"
        ) from error
    if not isinstance(schema, dict):
        raise QualificationValidationError("schema root is not an object")
    errors = _validate(document, schema, schema, "$")
    if errors:
        raise QualificationValidationError("; ".join(errors))


def qualification_exit(document: dict[str, Any]) -> int:
    if document.get("complete") is not True:
        return 2
    return {"pass": 0, "skip": 0, "fail": 1}.get(str(document.get("outcome")), 2)


def load_qualification(
    path: Path,
    schema_path: Path,
    *,
    require_complete: bool = False,
) -> dict[str, Any]:
    try:
        document = json.loads(path.read_text(encoding="utf-8"))
    except (OSError, json.JSONDecodeError) as error:
        raise QualificationValidationError(f"could not load {path}: {error}") from error
    validate_qualification(document, schema_path, require_complete=require_complete)
    return document


def write_qualification(path: Path, document: dict[str, Any], schema_path: Path) -> None:
    validate_qualification(document, schema_path)
    path.parent.mkdir(parents=True, exist_ok=True)
    descriptor, temporary = tempfile.mkstemp(prefix=f".{path.name}.", dir=path.parent)
    try:
        with os.fdopen(descriptor, "w", encoding="utf-8") as handle:
            json.dump(document, handle, indent=2, sort_keys=True)
            handle.write("\n")
        os.replace(temporary, path)
    except BaseException:
        try:
            os.unlink(temporary)
        except FileNotFoundError:
            pass
        raise


def utc_now() -> str:
    return datetime.now(timezone.utc).isoformat().replace("+00:00", "Z")


def new_run_id(prefix: str) -> str:
    stamp = datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%SZ")
    return f"{prefix}-{stamp}-{uuid.uuid4().hex[:12]}"


def empty_summary() -> dict[str, Any]:
    return {
        "total": 0,
        "passed": 0,
        "failed": 0,
        "invalid": 0,
        "skipped": 0,
        "counts": {},
    }


def error_record(kind: str, message: str, check: str | None = None) -> dict[str, Any]:
    return {"kind": kind, "message": message, "check": check}


def new_document(
    evidence_kind: str,
    mode: str,
    environment: dict[str, Any],
    run_id: str | None = None,
) -> dict[str, Any]:
    return {
        "$schema": SCHEMA_NAME,
        "schema_version": SCHEMA_VERSION,
        "run_id": run_id or new_run_id(mode),
        "evidence_kind": evidence_kind,
        "mode": mode,
        "complete": False,
        "outcome": "incomplete",
        "generated_at": utc_now(),
        "environment": environment,
        "summary": empty_summary(),
        "mutants": [],
        "faults": [],
        "checks": [],
        "errors": [],
        "artifacts": [],
        "metadata": {},
    }


def artifact_record(kind: str, path: Path, base: Path) -> dict[str, Any]:
    try:
        size = path.stat().st_size
        present = True
    except OSError:
        size = None
        present = False
    try:
        display_path = path.resolve().relative_to(base.resolve()).as_posix()
    except ValueError:
        display_path = str(path.resolve())
    return {
        "kind": kind,
        "path": display_path,
        "present": present,
        "size_bytes": size,
        "metadata": {},
    }
