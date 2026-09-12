import json
from pathlib import Path
import runpy


GOLDEN = Path(__file__).resolve().parents[3] / "examples/platformer-2d/capture/device-input.json"


def matches(step, event):
    expected = step["expected"]
    if event.get("device") != step["device"] or event.get("echo", False):
        return False
    if "action" in expected:
        return event.get("actions", {}).get(expected["action"], {}).get(expected["state"]) is True
    if event.get("event") != expected["event"]:
        return False
    if expected["event"] == "joy_axis":
        if event.get("axis") != expected["axis"]:
            return False
        value = event.get("value")
        if type(value) not in (int, float) or not -1 <= value <= 1:
            return False
        if expected["direction"] == 0:
            return abs(value) <= expected["neutral_tolerance"]
        return value * expected["direction"] >= expected["threshold"]
    if expected["event"] == "key":
        if event.get(expected["binding"]) != expected["code"]:
            return False
    elif event.get("button") != expected["button"]:
        return False
    if expected.get("target") == "start" and event.get("over_start") is not True:
        return False
    return event.get("pressed") is (expected["state"] == "pressed")


def evaluate(value, frames):
    if not frames or frames[-1].get("finished") is not True or "missing_devices" not in frames[-1]:
        return "fail"
    prompts = [prompt for frame in frames for prompt in frame["prompts"]]
    results = [result for frame in frames for result in frame["results"]]
    events = [event for frame in frames for event in frame["events"]]
    steps = value["steps"]
    if [p["step"] for p in prompts] != list(range(len(steps))) or [r["step"] for r in results] != list(range(len(steps))):
        return "fail"
    if any(type(e.get("time_ns")) is not int or type(e.get("device_id")) is not int or not e.get("identity") for e in events):
        return "fail"
    if any(a["time_ns"] > b["time_ns"] for a, b in zip(events, events[1:])):
        return "fail"
    missing = sorted({s["device"] for s in steps} - {e["device"] for e in events})
    if frames[-1]["missing_devices"] != missing:
        return "fail"
    if missing:
        return "untested"
    directions = {}
    presses = {}
    previous_end = -1
    for index, (step, prompt, result) in enumerate(zip(steps, prompts, results)):
        start = prompt["time_ns"]
        end = start + round(step["deadline_seconds"] * 1_000_000_000)
        if prompt["prompt"] != step["prompt"] or start < previous_end or result["status"] != "success" or result["time_ns"] < start:
            return "fail"
        previous_end = result["time_ns"]
        expected = step["expected"]
        identity = (step["device"], expected.get("action"), expected.get("event"), expected.get("button"), expected.get("binding"), expected.get("code"))
        released = expected.get("state") == "released" or expected.get("direction") == 0
        previous_device = directions.get(expected.get("axis")) if expected.get("event") == "joy_axis" else presses.get(identity)
        candidates = [
            event for event in events
            if event.get("step") == index
            and start <= event["time_ns"] <= min(end, result["time_ns"])
            and matches(step, event)
            and (not released or event["device_id"] == previous_device)
        ]
        if not candidates:
            return "fail"
        event = candidates[0]
        if expected.get("target") == "start" and expected["state"] == "released":
            if not any(f.get("started") is True and event["time_ns"] <= f["time_ns"] <= result["time_ns"] for f in frames):
                return "fail"
        if expected.get("event") == "joy_axis":
            axis = expected["axis"]
            if expected["direction"] == 0:
                directions.pop(axis)
            else:
                directions[axis] = event["device_id"]
        else:
            if expected["state"] == "released":
                presses.pop(identity)
            else:
                presses[identity] = event["device_id"]
        if "action" in expected:
            pressed = expected["state"] == "pressed"
            observations = [f["actions"].get(expected["action"], {}) for f in frames if event["time_ns"] <= f["time_ns"] <= result["time_ns"]]
            if not any(a.get("pressed") is pressed and a.get("just_pressed" if pressed else "just_released") is True for a in observations):
                return "fail"
    return "pass"


def verdict(value, output: Path):
    try:
        validate = runpy.run_path(str(Path(__file__).parent / "schema/validate.py"))["validate"]
        if validate(value) or json.loads(GOLDEN.read_text()) != [{"device": s["device"], "expected": s["expected"]} for s in value["steps"]]:
            return {"physical_input": "fail"}
        manifest = json.loads((output / "manifest.json").read_text())
        frames = [json.loads((output / f"frame-{frame:06d}.physical_input.json").read_text()) for frame in range(1, manifest["frames"] + 1)]
        if any(f.get("frame") != i or f.get("version") != 1 for i, f in enumerate(frames, 1)):
            return {"physical_input": "fail"}
        return {"physical_input": evaluate(value, frames)}
    except (OSError, ValueError, TypeError, KeyError, AttributeError):
        return {"physical_input": "fail"}
