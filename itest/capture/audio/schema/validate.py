import json
import math
from pathlib import Path
import re


SCHEMA = json.loads(Path(__file__).with_name("extension.json").read_text(encoding="utf-8"))


def _check(value, schema, path, errors):
    if "$ref" in schema:
        schema = SCHEMA["$defs"][schema["$ref"].split("/")[-1]]
    kind = schema["type"]
    valid_type = {
        "object": lambda: isinstance(value, dict),
        "array": lambda: isinstance(value, list),
        "string": lambda: isinstance(value, str),
        "integer": lambda: type(value) is int,
        "number": lambda: type(value) is int or (type(value) is float and math.isfinite(value)),
    }[kind]()
    if not valid_type:
        errors.append(f"{path}: expected {kind}")
        return
    if "const" in schema and value != schema["const"]:
        errors.append(f"{path}: must be {schema['const']}")
    if "enum" in schema and value not in schema["enum"]:
        errors.append(f"{path}: must be one of {schema['enum']}")
    if kind == "object":
        for key in schema["required"]:
            if key not in value:
                errors.append(f"{path}.{key}: required")
        for key, item in value.items():
            if key not in schema["properties"]:
                errors.append(f"{path}.{key}: unknown field")
            else:
                _check(item, schema["properties"][key], f"{path}.{key}", errors)
    elif kind == "array":
        if not schema["minItems"] <= len(value) <= schema["maxItems"]:
            errors.append(f"{path}: expected {schema['minItems']} items")
        for index, item in enumerate(value):
            _check(item, schema["items"], f"{path}[{index}]", errors)
    elif kind == "string" and "pattern" in schema:
        if not re.fullmatch(schema["pattern"], value):
            errors.append(f"{path}: expected itest/capture/audio/references/<example>/<scenario>.wav")
    elif kind in ("integer", "number"):
        if "minimum" in schema and value < schema["minimum"]:
            errors.append(f"{path}: below {schema['minimum']}")
        if "maximum" in schema and value > schema["maximum"]:
            errors.append(f"{path}: above {schema['maximum']}")


def validate(value) -> list[str]:
    errors = []
    _check(value, SCHEMA, "audio", errors)
    if errors:
        return errors
    cues = value["cues"]
    if [cue["name"] for cue in cues] != ["jump", "gem"]:
        errors.append("audio.cues: expected jump then gem")
    if not cues[0]["frame"] < cues[1]["frame"] < value["stop_frame"] < value["end_frame"]:
        errors.append("audio: cue, stop and end frames must increase strictly")
    margin = value["thresholds"]["alignment_ms"]
    previous_end = 0
    for cue in cues:
        start, end = cue["window_ms"]
        if not previous_end + margin <= start < cue["frame"] * 1000 / 60 < end:
            errors.append(f"audio.cues.{cue['name']}: window must contain its cue and leave alignment room")
        previous_end = end
    if not cues[-1]["frame"] * 1000 / 60 + margin < value["stop_frame"] * 1000 / 60 < previous_end:
        errors.append("audio.stop_frame: must fall inside the last cue window, at least 50 ms after its cue")
    start, end = value["silence_window_ms"]
    if not value["stop_frame"] * 1000 / 60 + margin <= start < end <= value["end_frame"] * 1000 / 60 - margin:
        errors.append("audio.silence_window_ms: must follow stop plus 50 ms and end 50 ms before capture ends")
    if start < previous_end + margin:
        errors.append("audio.silence_window_ms: must follow the last cue window plus alignment room")
    if end - start < 100:
        errors.append("audio.silence_window_ms: must cover at least 100 ms")
    return errors
