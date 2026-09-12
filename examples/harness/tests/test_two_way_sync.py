import math
import re
import sys
import unittest
from pathlib import Path

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from capture_compare import compare_checkpoint
from capture_schema import PROTOCOL, load_manifest


EXAMPLE = Path(__file__).resolve().parents[2] / "two-way-sync-demo"


def formula(source, function, frame, drawn_frame_offset):
    match = re.search(rf"{function}\((.+?) / ([\d.]+)\) \* ([\d.]+)", source)
    if match is None:
        raise AssertionError(f"Missing {function} orbit formula")
    clock, divisor, radius = match.groups()
    if clock == "frame":
        index = frame
    elif "get_frames_drawn()" in clock:
        index = drawn_frame_offset + frame
    else:
        raise AssertionError(f"Unknown orbit clock: {clock}")
    return getattr(math, function)(index / float(divisor)) * float(radius)


def facts(manifest, checkpoint, visible=True):
    frame = checkpoint["frame"]
    return {
        "version": PROTOCOL["version"],
        "scenario": manifest["scenario"],
        "scene": manifest["scene"],
        "frame": frame,
        "viewport": manifest["viewport"],
        "physics_ticks": frame,
        "virtual_elapsed_ns": frame * PROTOCOL["step_ns"],
        "fixed_step_ns": PROTOCOL["fixed_step_ns"],
        "fixed_elapsed_ns": frame * PROTOCOL["fixed_step_ns"],
        "nodes": [{
            "path": "Quad",
            "position": [100 * math.sin(frame / 50), 100 * math.cos(frame / 50)],
            "visible": visible,
        }],
        "regions": [{
            "name": region["name"],
            "rect": region["rect"],
            "non_blank_fraction": 1,
            "dominant_colour": [255, 255, 255] if visible else [0, 0, 0],
        } for region in checkpoint["regions"]],
    }


class TwoWaySyncTests(unittest.TestCase):
    def test_checkpoint_positions_ignore_drawn_frame_offset(self):
        manifest = load_manifest(EXAMPLE / "capture/orbit-split.json")
        script = EXAMPLE / "godot/capture/quad.gd"
        x_source = (script if script.exists() else EXAMPLE / "godot/quad.gd").read_text()
        y_source = (EXAMPLE / "rust/src/lib.rs").read_text()
        for offset in (3, 17, 50, 137):
            for checkpoint in manifest["checkpoints"]:
                node = checkpoint["nodes"][0]
                frame = checkpoint["frame"]
                for axis, source, function in ((0, x_source, "sin"), (1, y_source, "cos")):
                    with self.subTest(offset=offset, frame=frame, axis=axis):
                        actual = formula(source, function, frame, offset)
                        self.assertLessEqual(abs(actual - node["position"][axis]), node["tolerance"])

    def test_capture_formula_preserves_the_demo_trajectory(self):
        original = (EXAMPLE / "godot/quad.gd").read_text()
        capture = (EXAMPLE / "godot/capture/quad.gd").read_text()
        for frame in (0, 30, 60, 90, 120):
            self.assertEqual(formula(original, "sin", frame, 0), formula(capture, "sin", frame, 0))

    def test_positive_manifest_matches_the_analytic_circle(self):
        manifest = load_manifest(EXAMPLE / "capture/orbit-split.json")
        self.assertEqual(manifest["viewport"], [960, 640])
        self.assertEqual(manifest["seed"], 3)
        self.assertEqual(manifest["frames"], 120)
        self.assertEqual(manifest["extensions"], {})
        self.assertEqual([point["frame"] for point in manifest["checkpoints"]], [0, 30, 60, 90, 120])
        for point in manifest["checkpoints"]:
            actual = facts(manifest, point)
            self.assertEqual(point["nodes"][0]["tolerance"], 1)
            self.assertEqual(compare_checkpoint(manifest, point, actual), [])
            x, y = actual["nodes"][0]["position"]
            left, top, width, height = point["regions"][0]["rect"]
            self.assertLessEqual(abs(left + width / 2 - (480 + x)), 1)
            self.assertLessEqual(abs(top + height / 2 - (320 + y)), 1)
            self.assertLessEqual(width, 8)
            self.assertLessEqual(height, 8)

    def test_hidden_manifest_fails_only_visibility_at_frame_zero(self):
        manifest = load_manifest(EXAMPLE / "capture/orbit-split-hidden.json")
        differences = []
        for point in manifest["checkpoints"]:
            frame = point["frame"]
            actual = facts(manifest, point, visible=frame != 0)
            differences.extend(
                (frame, diff["path"]) for diff in compare_checkpoint(manifest, point, actual)
            )
        self.assertEqual(differences, [(0, "nodes.Quad.visible")])


if __name__ == "__main__":
    unittest.main()
