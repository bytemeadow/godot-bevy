from __future__ import annotations

import copy
import json
import sys
import tempfile
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from capture_compare import compare_checkpoint
from capture_schema import CaptureValidationError, load_manifest, loads_json, validate_manifest
from capture import CaptureSession


def manifest():
    checkpoint = {
        "frame": 0,
        "nodes": [{"path": "Icon", "position": [100, 0], "tolerance": 1, "visible": True}],
        "regions": [{
            "name": "sprite", "rect": [10, 10, 20, 20],
            "non_blank": {"background": [0, 0, 0], "tolerance": 2, "min_fraction": 0.5},
            "dominant_colour": {"rgb": [54, 61, 82], "tolerance": 2},
        }],
    }
    last = copy.deepcopy(checkpoint)
    last["frame"] = 60
    return {
        "version": 2, "example": "sample", "scenario": "orbit", "scene": "res://main.tscn",
        "pacing": "fixed", "extensions": {}, "frames": 60, "seed": 7, "viewport": [640, 480],
        "clocks": {"physics_hz": 60, "bevy_step_ns": 16666667},
        "settle": {"stable_frames": 2, "nodes": ["Icon"], "classes": ["Sprite2D"]},
        "checkpoints": [checkpoint, last],
    }


def facts(frame=0):
    return {
        "version": 2, "scenario": "orbit", "scene": "res://main.tscn", "frame": frame,
        "viewport": [640, 480], "physics_ticks": frame,
        "virtual_elapsed_ns": frame * 16666667,
        "fixed_step_ns": 16666668, "fixed_elapsed_ns": frame * 16666668,
        "nodes": [{"path": "Icon", "position": [100, 0], "visible": True}],
        "regions": [{"name": "sprite", "rect": [10, 10, 20, 20],
                     "non_blank_fraction": 0.75, "dominant_colour": [54, 61, 82]}],
    }


class ManifestTests(unittest.TestCase):
    def test_valid_manifest(self):
        self.assertEqual(validate_manifest(manifest()), [])

    def test_rejects_missing_guarantees_and_unknown_fields(self):
        for field in ("seed", "clocks", "settle", "viewport", "checkpoints"):
            value = manifest()
            del value[field]
            with self.subTest(field=field):
                self.assertTrue(validate_manifest(value))
        value = manifest()
        value["audio"] = {"pass": True}
        self.assertTrue(validate_manifest(value))

    def test_rejects_nonfinite_and_boolean_numbers(self):
        for number in (float("nan"), float("inf"), True, -1):
            value = manifest()
            value["checkpoints"][0]["nodes"][0]["tolerance"] = number
            with self.subTest(number=number):
                self.assertTrue(validate_manifest(value))

    def test_checkpoints_cover_reset_and_final_frame_in_order(self):
        for frames in ([1, 60], [0, 59], [0, 0, 60], [0, 61, 60]):
            value = manifest()
            checkpoint = value["checkpoints"][0]
            value["checkpoints"] = [dict(checkpoint, frame=frame) for frame in frames]
            with self.subTest(frames=frames):
                self.assertTrue(validate_manifest(value))

    def test_rect_must_fit_viewport(self):
        value = manifest()
        value["checkpoints"][0]["regions"][0]["rect"] = [639, 10, 2, 20]
        self.assertTrue(validate_manifest(value))

    def test_rejects_duplicate_names_and_paths(self):
        for key in ("nodes", "regions"):
            value = manifest()
            entries = value["checkpoints"][0][key]
            entries.append(copy.deepcopy(entries[0]))
            self.assertTrue(validate_manifest(value))

    def test_rejects_path_traversal(self):
        for path in ("../Icon", "/root/Icon", "Icon/../../Other"):
            value = manifest()
            value["checkpoints"][0]["nodes"][0]["path"] = path
            self.assertTrue(validate_manifest(value))

    def test_rejects_duplicate_json_keys(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "bad.json"
            path.write_text('{"version": 2, "version": 2}')
            with self.assertRaises(CaptureValidationError):
                load_manifest(path)

    def test_rejects_overflowing_json_numbers(self):
        with self.assertRaises(CaptureValidationError):
            loads_json('{"position": [1e999, 0]}')

    def test_negative_manifest_is_valid_but_rejects_the_empty_corner(self):
        path = Path(__file__).resolve().parents[3] / "examples/simple-node2d-movement/capture/orbit-empty-region.json"
        value = load_manifest(path)
        point = value["checkpoints"][0]
        actual = facts()
        actual.update(scenario="orbit-empty-region", viewport=[960, 640])
        actual["regions"] = [{"name": "empty-corner", "rect": [0, 0, 32, 32],
                              "non_blank_fraction": 0, "dominant_colour": [0, 0, 0]}]
        diffs = compare_checkpoint(value, point, actual)
        self.assertEqual([diff["path"] for diff in diffs], ["regions.empty-corner.non_blank_fraction"])

    def test_orbit_manifest_is_valid(self):
        path = Path(__file__).resolve().parents[3] / "examples/simple-node2d-movement/capture/orbit.json"
        self.assertEqual(load_manifest(path)["scenario"], "orbit")


class ComparatorTests(unittest.TestCase):
    def compare(self, actual):
        value = manifest()
        return compare_checkpoint(value, value["checkpoints"][0], actual)

    def test_position_and_colour_tolerances_are_inclusive(self):
        actual = facts()
        actual["nodes"][0]["position"] = [101, -1]
        actual["regions"][0]["dominant_colour"] = [56, 59, 84]
        self.assertEqual(self.compare(actual), [])
        actual["nodes"][0]["position"][0] = 101.001
        self.assertTrue(self.compare(actual))

    def test_empty_region_fails_non_blank(self):
        actual = facts()
        actual["regions"][0]["non_blank_fraction"] = 0
        self.assertIn("regions.sprite.non_blank_fraction", [d["path"] for d in self.compare(actual)])

    def test_hidden_sprite_fails(self):
        actual = facts()
        actual["nodes"][0]["visible"] = False
        self.assertIn("nodes.Icon.visible", [d["path"] for d in self.compare(actual)])

    def test_colour_mismatch_fails(self):
        actual = facts()
        actual["regions"][0]["dominant_colour"] = [57, 61, 82]
        self.assertTrue(self.compare(actual))

    def test_missing_duplicate_and_nonfinite_facts_fail(self):
        for key in ("nodes", "regions", "viewport", "virtual_elapsed_ns"):
            actual = facts()
            del actual[key]
            self.assertTrue(self.compare(actual))
        actual = facts()
        actual["nodes"].append(copy.deepcopy(actual["nodes"][0]))
        self.assertTrue(self.compare(actual))
        actual = facts()
        actual["nodes"][0]["position"][0] = float("nan")
        self.assertTrue(self.compare(actual))

    def test_wrong_frame_scene_viewport_and_clock_fail(self):
        for key, wrong in (("frame", 1), ("scene", "res://other.tscn"),
                           ("viewport", [1, 1]), ("viewport", [640.0, 480]), ("physics_ticks", 1),
                           ("virtual_elapsed_ns", 1)):
            actual = facts()
            actual[key] = wrong
            self.assertTrue(self.compare(actual))


class SessionTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        self.session = CaptureSession(manifest(), self.root)

    def test_requests_are_written_only_after_matching_ready(self):
        self.assertFalse(list(self.root.glob("*.request.json")))
        self.session.accept("CAPTURE_READY scenario=orbit scene=res://main.tscn frame=0")
        self.assertEqual(len(list(self.root.glob("*.request.json"))), 2)

    def test_wrong_or_duplicate_ready_fails(self):
        with self.assertRaises(CaptureValidationError):
            self.session.accept("CAPTURE_READY scenario=wrong scene=res://main.tscn frame=0")
        self.session.accept("CAPTURE_READY scenario=orbit scene=res://main.tscn frame=0")
        with self.assertRaises(CaptureValidationError):
            self.session.accept("CAPTURE_READY scenario=orbit scene=res://main.tscn frame=0")

    def test_facts_before_ready_and_missing_png_fail(self):
        line = "CAPTURE_FACTS frame=0 " + json.dumps(facts())
        with self.assertRaises(CaptureValidationError):
            self.session.accept(line)
        self.session.accept("CAPTURE_READY scenario=orbit scene=res://main.tscn frame=0")
        self.session.accept(line)
        self.assertTrue(self.session.diffs)
        self.assertTrue((self.root / "frame-000000.facts.json").is_file())

    def test_incomplete_run_and_untested_capabilities_never_pass(self):
        verdict = self.session.verdict()
        self.assertNotEqual(verdict["exit_code"], 0)
        self.assertFalse(verdict["complete"])
        for capability in ("audio", "physical_input", "synthetic_input", "browser", "rendering_3d"):
            self.assertEqual(verdict["capabilities"][capability], "untested")


if __name__ == "__main__":
    unittest.main()
