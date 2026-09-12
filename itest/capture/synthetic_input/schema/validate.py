import json
import math
from pathlib import Path


SCHEMA = json.loads(Path(__file__).with_name("extension.json").read_text())


def check(value, schema, path="extension"):
    if "oneOf" in schema:
        return [] if sum(not check(value, option, path) for option in schema["oneOf"]) == 1 else [f"{path}: expected one supported event shape"]
    kind = schema.get("type")
    types = {"object": dict, "array": list, "string": str, "boolean": bool, "integer": int}
    if kind in types and type(value) is not types[kind]:
        return [f"{path}: expected {kind}"]
    if kind == "number" and (type(value) not in (int, float) or (type(value) is float and not math.isfinite(value))):
        return [f"{path}: expected a finite number"]
    if "const" in schema and (type(value) is not type(schema["const"]) or value != schema["const"]):
        return [f"{path}: expected {schema['const']!r}"]
    if "enum" in schema and not any(type(value) is type(item) and value == item for item in schema["enum"]):
        return [f"{path}: expected one of {schema['enum']}"]
    errors = []
    if kind == "object":
        errors += [f"{path}.{key}: required" for key in schema["required"] if key not in value]
        for key, item in value.items():
            if key not in schema["properties"]:
                errors.append(f"{path}.{key}: unknown key")
            else:
                errors += check(item, schema["properties"][key], f"{path}.{key}")
    if kind == "array":
        if len(value) < schema.get("minItems", 0):
            errors.append(f"{path}: too few items")
        if schema.get("uniqueItems") and len({json.dumps(item, sort_keys=True) for item in value}) != len(value):
            errors.append(f"{path}: duplicate items")
        for index, item in enumerate(value):
            errors += check(item, schema["items"], f"{path}[{index}]")
    if kind == "string" and not schema.get("minLength", 0) <= len(value) <= schema.get("maxLength", math.inf):
        errors.append(f"{path}: invalid length")
    if kind in ("integer", "number"):
        for key, invalid in (("minimum", lambda limit: value < limit), ("maximum", lambda limit: value > limit), ("exclusiveMinimum", lambda limit: value <= limit), ("exclusiveMaximum", lambda limit: value >= limit)):
            if key in schema and invalid(schema[key]):
                errors.append(f"{path}: violates {key} {schema[key]}")
    return errors


def validate(value):
    errors = check(value, SCHEMA)
    if errors:
        return errors
    previous = -value["max_latency_frames"]
    held = set()
    for index, cue in enumerate(value["trace"]):
        if cue["action"] not in value["actions"]:
            errors.append(f"trace[{index}]: action must be in actions")
        if cue["frame"] <= previous + value["max_latency_frames"]:
            errors.append(f"trace[{index}]: leave the observation window between injections")
        previous = cue["frame"]
        key = (cue["action"], cue["binding"], cue["code"])
        if cue["pressed"]:
            if key in held:
                errors.append(f"trace[{index}]: repeated press without release")
            held.add(key)
        elif key not in held:
            errors.append(f"trace[{index}]: release without press")
        else:
            held.remove(key)
    if held:
        errors.append("trace: every press requires a release")
    return errors
