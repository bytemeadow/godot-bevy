import json
from pathlib import Path


def validate(value) -> list[str]:
    schema = json.loads(Path(__file__).with_name("extension.json").read_text())
    if not isinstance(value, dict):
        return ["browser extension must be an object"]
    if len(value) > schema["maxProperties"]:
        return ["browser extension must be empty while godot-bevy#268 is blocked"]
    return []
