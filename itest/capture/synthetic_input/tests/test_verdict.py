import copy
import json
from pathlib import Path
import runpy
import tempfile
import unittest


ADAPTER = Path(__file__).resolve().parents[1]
ROOT = ADAPTER.parents[2]
MANIFEST = ROOT / "examples/platformer-2d/capture/input-replay.json"


class ReplayTests(unittest.TestCase):
    def setUp(self):
        self.assertTrue((ADAPTER / "verdict.py").is_file(), "synthetic verdict is required")
        self.value = json.loads(MANIFEST.read_text())["extensions"]["synthetic_input"]
        self.evaluate = runpy.run_path(str(ADAPTER / "verdict.py"))["evaluate"]
        self.frames = []
        for frame in range(1, 121):
            self.frames.append({"version": 1, "frame": frame, "events": [], "injections": [], "actions": {}})
        for index, cue in enumerate(self.value["trace"]):
            event = {"event": "key", "device": "keyboard", "device_id": 0, "echo": False, "keycode": 0, "physical_keycode": 0, "pressed": cue["pressed"]}
            event[cue["binding"]] = cue["code"]
            self.frames[cue["frame"] - 1]["injections"].append({"index": index, **cue})
            self.frames[cue["frame"]]["events"].append(event)
            self.frames[cue["frame"]]["actions"][cue["action"]] = {
                "pressed": cue["pressed"], "just_pressed": cue["pressed"],
                "just_released": not cue["pressed"], "strength": float(cue["pressed"]),
            }

    def test_replay_passes_at_declared_buffer_boundary(self):
        self.assertEqual(self.evaluate(self.value, self.frames), "pass")

    def test_wrong_binding_does_not_pass(self):
        cue = self.value["trace"][0]
        event = self.frames[cue["frame"]]["events"][0]
        event["keycode"], event["physical_keycode"] = event["physical_keycode"], event["keycode"]
        self.assertEqual(self.evaluate(self.value, self.frames), "fail")

    def test_action_outside_buffer_boundary_fails(self):
        cue = self.value["trace"][0]
        observed = self.frames[cue["frame"]]
        self.frames[cue["frame"] + self.value["max_latency_frames"]]["actions"] = observed["actions"]
        observed["actions"] = {}
        self.assertEqual(self.evaluate(self.value, self.frames), "fail")

    def test_missing_release_fails(self):
        cue = next(c for c in self.value["trace"] if not c["pressed"])
        self.frames[cue["frame"]]["events"] = []
        self.assertEqual(self.evaluate(self.value, self.frames), "fail")

    def test_verdict_only_sets_synthetic_input(self):
        callback = runpy.run_path(str(ADAPTER / "verdict.py"))["verdict"]
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            (output / "manifest.json").write_text(MANIFEST.read_text())
            for frame in self.frames:
                (output / f"frame-{frame['frame']:06d}.synthetic_input.json").write_text(json.dumps(frame))
            self.assertEqual(callback(self.value, output), {"synthetic_input": "pass"})

    def test_validator_rejects_unknown_binding_and_overlapping_edges(self):
        validate = runpy.run_path(str(ADAPTER / "schema/validate.py"))["validate"]
        self.assertEqual(validate(self.value), [])
        value = copy.deepcopy(self.value)
        value["trace"][0]["binding"] = "unicode"
        self.assertTrue(validate(value))
        value = copy.deepcopy(self.value)
        value["trace"][1]["frame"] = value["trace"][0]["frame"] + 1
        self.assertTrue(validate(value))
