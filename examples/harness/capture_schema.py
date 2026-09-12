from __future__ import annotations

import copy
import json
import runpy
import math
import re
from pathlib import Path
from typing import Any

REPOSITORY = Path(__file__).resolve().parents[2]
PROTOCOL = json.loads((REPOSITORY / "godot-bevy-test/src/capture/protocol.json").read_text())
ADAPTER_ROOT = Path(__file__).parent
SCHEMA = ADAPTER_ROOT / PROTOCOL["schema"]


class CaptureValidationError(ValueError):
    pass


def _unique_object(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise CaptureValidationError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def loads_json(text: str):
    def reject_constant(value):
        raise CaptureValidationError(f"non-finite JSON number: {value}")

    def finite_float(value):
        number = float(value)
        if not math.isfinite(number):
            reject_constant(value)
        return number

    try:
        return json.loads(text, object_pairs_hook=_unique_object, parse_constant=reject_constant, parse_float=finite_float)
    except ValueError as error:
        raise CaptureValidationError(str(error)) from error


def finite_number(value):
    if type(value) not in (int, float):
        return False
    try:
        return math.isfinite(value)
    except OverflowError:
        return False


def _matches(value, expected):
    return {
        "object": isinstance(value, dict),
        "array": isinstance(value, list),
        "string": isinstance(value, str),
        "integer": type(value) is int,
        "number": finite_number(value),
        "boolean": type(value) is bool,
    }[expected]


def _validate(value, rule, root, path):
    if "$ref" in rule:
        rule = root["$defs"][rule["$ref"].removeprefix("#/$defs/")]
    if "type" in rule and not _matches(value, rule["type"]):
        return [f"{path}: expected {rule['type']}"]
    errors = []
    if "const" in rule and value != rule["const"]:
        errors.append(f"{path}: expected {rule['const']}")
    if "enum" in rule and value not in rule["enum"]:
        errors.append(f"{path}: expected one of {rule['enum']}")
    if isinstance(value, dict):
        for key in rule.get("required", []):
            if key not in value:
                errors.append(f"{path}: missing {key}")
        properties = rule.get("properties", {})
        for key, child in value.items():
            if "propertyNames" in rule:
                errors.extend(_validate(key, rule["propertyNames"], root, f"{path}.{key}"))
            if key in properties:
                errors.extend(_validate(child, properties[key], root, f"{path}.{key}"))
            elif rule.get("additionalProperties") is False:
                errors.append(f"{path}: unexpected {key}")
    if isinstance(value, list):
        if not rule.get("minItems", 0) <= len(value) <= rule.get("maxItems", math.inf):
            errors.append(f"{path}: wrong number of items")
        if rule.get("uniqueItems") and len({json.dumps(v, sort_keys=True) for v in value}) != len(value):
            errors.append(f"{path}: duplicate items")
        for index, child in enumerate(value):
            errors.extend(_validate(child, rule.get("items", {}), root, f"{path}[{index}]"))
    if isinstance(value, str):
        if len(value) < rule.get("minLength", 0) or not re.search(rule.get("pattern", ""), value):
            errors.append(f"{path}: invalid string")
    if type(value) in (int, float):
        if not finite_number(value):
            errors.append(f"{path}: non-finite number")
        elif not rule.get("minimum", -math.inf) <= value <= rule.get("maximum", math.inf):
            errors.append(f"{path}: number out of range")
        elif "exclusiveMinimum" in rule and value <= rule["exclusiveMinimum"]:
            errors.append(f"{path}: must exceed {rule['exclusiveMinimum']}")
    return errors


def validate_manifest(document: Any) -> list[str]:
    if isinstance(document, dict) and document.get("version") == 1:
        return [PROTOCOL["migration"]]
    schema = json.loads(SCHEMA.read_text(encoding="utf-8"))
    errors = _validate(document, schema, schema, "$")
    if errors:
        return errors
    checkpoints = document["checkpoints"]
    frames = [point["frame"] for point in checkpoints]
    if frames != sorted(set(frames)) or frames[0] != 0 or frames[-1] != document["frames"]:
        errors.append("checkpoints: require strictly increasing frames from 0 through frames")
    paths = [document["scene"][6:], *document["settle"]["nodes"]]
    width, height = document["viewport"]
    for point in checkpoints:
        for key, identity in (("nodes", "path"), ("regions", "name")):
            names = [item[identity] for item in point[key]]
            if len(names) != len(set(names)):
                errors.append(f"checkpoints[{point['frame']}].{key}: duplicate {identity}")
        paths.extend(node["path"] for node in point["nodes"])
        for region in point["regions"]:
            x, y, w, h = region["rect"]
            if not w or not h or x + w > width or y + h > height:
                errors.append(f"regions.{region['name']}.rect: outside viewport or empty")
    if any(".." in path.split("/") or "\\" in path for path in paths):
        errors.append("paths: traversal and backslashes are forbidden")
    for name, value in document["extensions"].items():
        try:
            schema_path = ADAPTER_ROOT / name / PROTOCOL["extension_schema"]
            if not schema_path.is_file():
                raise CaptureValidationError(f"missing adapter schema: {schema_path}")
            validate = adapter_function(name, "extension_validator", "validate")
            result = validate(copy.deepcopy(value))
            if not isinstance(result, list) or not all(isinstance(error, str) for error in result):
                raise CaptureValidationError("validate(value) must return list[str]")
            errors.extend(f"extensions.{name}: {error}" for error in result)
        except Exception as error:
            errors.append(f"extensions.{name}: {error}")
    return errors


def adapter_function(name: str, key: str, function: str):
    path = ADAPTER_ROOT / name / PROTOCOL[key]
    if not path.is_file():
        raise CaptureValidationError(f"missing adapter {function}: {path}")
    callback = runpy.run_path(str(path)).get(function)
    if not callable(callback):
        raise CaptureValidationError(f"{path} must define {function}")
    return callback


def load_manifest(path: Path) -> dict:
    document = loads_json(path.read_text(encoding="utf-8"))
    errors = validate_manifest(document)
    if errors:
        raise CaptureValidationError("; ".join(errors))
    return document


def frame_file(frame: int, kind: str) -> str:
    return f"{PROTOCOL['frame_prefix']}{frame:0{PROTOCOL['frame_digits']}d}{PROTOCOL[kind + '_suffix']}"


def write_json(path: Path, document) -> None:
    temporary = path.with_suffix(path.suffix + ".tmp")
    temporary.write_text(json.dumps(document, indent=2, allow_nan=False) + "\n", encoding="utf-8")
    temporary.replace(path)


if __name__ == "__main__":
    import argparse

    parser = argparse.ArgumentParser(description="Validate a capture-v2 scenario without launching it")
    parser.add_argument("manifest", type=Path)
    args = parser.parse_args()
    try:
        load_manifest(args.manifest)
    except (OSError, CaptureValidationError) as error:
        parser.exit(PROTOCOL["exit"]["error"], f"{error}\n")
