import json
from pathlib import Path
import runpy


def evaluate(value, frames):
    injections = [event for frame in frames for event in frame["injections"]]
    if injections != [{"index": index, **cue} for index, cue in enumerate(value["trace"])]:
        return "fail"
    for cue in value["trace"]:
        first = cue["frame"]
        last = first + value["max_latency_frames"]
        window = [f for f in frames if first <= f["frame"] <= last]
        other_binding = "keycode" if cue["binding"] == "physical_keycode" else "physical_keycode"
        observed = [f["frame"] for f in window for e in f["events"] if e.get("event") == "key" and e.get("device") == "keyboard" and e.get("device_id") == 0 and e.get(cue["binding"]) == cue["code"] and e.get(other_binding) == 0 and e.get("pressed") is cue["pressed"] and e.get("echo") is False]
        if len(observed) != 1:
            return "fail"
        edge = "just_pressed" if cue["pressed"] else "just_released"
        if not any(f["frame"] >= observed[0] and f["actions"].get(cue["action"], {}).get("pressed") is cue["pressed"] and f["actions"].get(cue["action"], {}).get(edge) is True for f in window):
            return "fail"
    return "pass"


def verdict(value, output: Path):
    try:
        validate = runpy.run_path(str(Path(__file__).parent / "schema/validate.py"))["validate"]
        if validate(value):
            return {"synthetic_input": "fail"}
        manifest = json.loads((output / "manifest.json").read_text())
        frames = [json.loads((output / f"frame-{frame:06d}.synthetic_input.json").read_text()) for frame in range(1, manifest["frames"] + 1)]
        if any(f.get("frame") != i or f.get("version") != 1 for i, f in enumerate(frames, 1)):
            return {"synthetic_input": "fail"}
        return {"synthetic_input": evaluate(value, frames)}
    except (OSError, ValueError, TypeError, KeyError, AttributeError):
        return {"synthetic_input": "fail"}
