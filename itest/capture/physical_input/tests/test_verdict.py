import copy
import json
from pathlib import Path
import runpy
import tempfile
import unittest


ADAPTER = Path(__file__).resolve().parents[1]
ROOT = ADAPTER.parents[2]
MANIFEST = ROOT / "examples/platformer-2d/capture/devices.json"


def recording(value):
    frames = []
    for index, step in enumerate(value["steps"]):
        expected = step["expected"]
        event = {
            "step": index,
            "time_ns": index * 10_000_000_000 + 1_000_000_000,
            "device": step["device"],
            "device_id": 0,
            "identity": "fixture",
            "class": "InputEvent",
            "text": "fixture",
            "matched": True,
            "echo": False,
            "actions": {},
        }
        if "action" in expected:
            event.update(event="key" if step["device"] == "keyboard" else "joy_button")
            event["actions"][expected["action"]] = {
                "pressed": expected["state"] == "pressed",
                "released": expected["state"] == "released",
            }
        else:
            event.update({key: item for key, item in expected.items() if key not in ("state", "direction", "threshold", "neutral_tolerance", "target")})
            if expected["event"] == "joy_axis":
                event["value"] = float(expected["direction"])
            else:
                event["pressed"] = expected["state"] == "pressed"
                event["over_start"] = True
        actions = {
            action: {"pressed": False, "just_pressed": False, "just_released": False, "strength": 0.0}
            for action in value["actions"]
        }
        if "action" in expected:
            pressed = expected["state"] == "pressed"
            actions[expected["action"]].update(pressed=pressed, just_pressed=pressed, just_released=not pressed, strength=float(pressed))
        frames.append({
            "version": 1, "frame": index + 1,
            "time_ns": event["time_ns"],
            "prompts": [{"step": index, "time_ns": index * 10_000_000_000, "prompt": step["prompt"]}],
            "events": [event],
            "results": [{"step": index, "time_ns": event["time_ns"], "status": "success"}],
            "actions": actions,
            "started": index >= 1,
            "finished": index + 1 == len(value["steps"]),
        })
    frames[-1]["missing_devices"] = []
    return frames


class VerdictTests(unittest.TestCase):
    def setUp(self):
        self.assertTrue((ADAPTER / "verdict.py").is_file(), "physical verdict is required")
        self.value = json.loads(MANIFEST.read_text())["extensions"]["physical_input"]
        self.frames = recording(self.value)
        self.evaluate = runpy.run_path(str(ADAPTER / "verdict.py"))["evaluate"]

    def test_complete_session_passes(self):
        self.assertEqual(self.evaluate(self.value, self.frames), "pass")

    def test_deadline_is_inclusive(self):
        frame = self.frames[0]
        frame["time_ns"] = frame["events"][0]["time_ns"] = frame["results"][0]["time_ns"] = 5_000_000_000
        self.assertEqual(self.evaluate(self.value, self.frames), "pass")

    def test_late_event_cannot_pass(self):
        frame = self.frames[0]
        frame["time_ns"] = frame["events"][0]["time_ns"] = frame["results"][0]["time_ns"] = 5_000_000_001
        self.assertEqual(self.evaluate(self.value, self.frames), "fail")

    def test_on_time_arrival_can_be_processed_after_the_deadline(self):
        frame = self.frames[0]
        frame["events"][0]["time_ns"] = 5_000_000_000
        frame["time_ns"] = frame["results"][0]["time_ns"] = 5_016_666_667
        self.assertEqual(self.evaluate(self.value, self.frames), "pass")

    def test_neutral_requires_an_arriving_event(self):
        index = next(i for i, s in enumerate(self.value["steps"]) if s["expected"].get("direction") == 0)
        self.frames[index]["events"] = []
        self.assertEqual(self.evaluate(self.value, self.frames), "fail")

    def test_direction_is_not_neutral(self):
        index = next(i for i, s in enumerate(self.value["steps"]) if s["expected"].get("direction") == 0)
        self.frames[index]["events"][0]["value"] = 0.4
        self.assertEqual(self.evaluate(self.value, self.frames), "fail")

    def test_neutral_must_come_from_same_controller(self):
        index = next(i for i, s in enumerate(self.value["steps"]) if s["expected"].get("direction") == 0)
        self.frames[index]["events"][0]["device_id"] = 1
        self.assertEqual(self.evaluate(self.value, self.frames), "fail")

    def test_unrelated_controller_does_not_hide_the_correct_neutral(self):
        index = next(i for i, s in enumerate(self.value["steps"]) if s["expected"].get("direction") == 0)
        events = self.frames[index]["events"]
        events.insert(0, {**events[0], "device_id": 1, "time_ns": events[0]["time_ns"] - 1, "matched": False})
        self.assertEqual(self.evaluate(self.value, self.frames), "pass")

    def test_missing_hardware_is_untested_not_pass(self):
        for frame in self.frames:
            frame["events"] = [e for e in frame["events"] if e["device"] != "controller"]
        self.frames[-1]["missing_devices"] = ["controller"]
        self.assertEqual(self.evaluate(self.value, self.frames), "untested")

    def test_wrong_controller_event_proves_presence_but_not_success(self):
        for frame in self.frames:
            for event in frame["events"]:
                if event["device"] == "controller":
                    event.update(event="joy_button", actions={}, button=99, pressed=True)
        self.assertEqual(self.evaluate(self.value, self.frames), "fail")

    def test_missing_actions_observation_fails(self):
        index = next(i for i, s in enumerate(self.value["steps"]) if "action" in s["expected"])
        self.frames[index]["actions"] = {}
        self.assertEqual(self.evaluate(self.value, self.frames), "fail")

    def test_start_click_requires_the_button_signal(self):
        for frame in self.frames:
            frame["started"] = False
        self.assertEqual(self.evaluate(self.value, self.frames), "fail")

    def test_missing_final_evidence_fails(self):
        self.assertEqual(self.evaluate(self.value, self.frames[:-1]), "fail")

    def test_verdict_reads_standard_evidence_leaf(self):
        manifest = json.loads(MANIFEST.read_text())
        manifest["frames"] = len(self.frames)
        callback = runpy.run_path(str(ADAPTER / "verdict.py"))["verdict"]
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            (output / "manifest.json").write_text(json.dumps(manifest))
            for frame in self.frames:
                (output / f"frame-{frame['frame']:06d}.physical_input.json").write_text(json.dumps(frame))
            self.assertEqual(callback(self.value, output), {"physical_input": "pass"})
            (output / "frame-000001.physical_input.json").unlink()
            self.assertEqual(callback(self.value, output), {"physical_input": "fail"})

    def test_validator_rejects_invalid_deadline_and_unknown_key(self):
        validate = runpy.run_path(str(ADAPTER / "schema/validate.py"))["validate"]
        self.assertEqual(validate(self.value), [])
        for deadline in (0, -1, True, float("nan"), "5"):
            value = copy.deepcopy(self.value)
            value["steps"][0]["deadline_seconds"] = deadline
            self.assertTrue(validate(value))
        value = copy.deepcopy(self.value)
        value["unexpected"] = True
        self.assertTrue(validate(value))
