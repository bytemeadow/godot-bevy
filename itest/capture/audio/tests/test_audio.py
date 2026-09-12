import copy
import importlib.util
import json
from pathlib import Path
import random
import runpy
import struct
import tempfile
import unittest


ROOT = Path(__file__).resolve().parents[1]
SPEC = importlib.util.spec_from_file_location("audio_capture", ROOT / "audio_capture.py")
audio = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(audio)


class SignalTests(unittest.TestCase):
    rate = 48000
    thresholds = {"correlation": 0.98, "alignment_ms": 50, "level_db": 1, "silence_dbfs": -60}
    cues = [{"name": "jump", "frame": 6, "window_ms": [60, 180]}]

    def setUp(self):
        rng = random.Random(7)
        self.reference = [0.0] * (self.rate // 2 * 2)
        for index in range(4800, 6000):
            value = rng.uniform(-0.3, 0.3)
            self.reference[2 * index:2 * index + 2] = [value, value * 0.7]

    def compare(self, actual):
        return audio.compare_signals(
            self.reference, actual, self.rate, self.cues, self.thresholds, [350, 450]
        )

    def shifted(self, milliseconds):
        count = round(milliseconds * self.rate / 1000) * 2
        if count >= 0:
            return [0.0] * count + self.reference[:len(self.reference) - count]
        return self.reference[-count:] + [0.0] * -count

    def test_correlation_accepts_identical_signal(self):
        result = self.compare(self.reference)
        self.assertEqual(result["status"], "pass")
        self.assertGreaterEqual(result["cues"][0]["correlation"], 0.98)

    def test_correlation_rejects_wrong_cue(self):
        actual = [value * (-1 if index % 4 < 2 else 1) for index, value in enumerate(self.reference)]
        result = self.compare(actual)
        self.assertLess(result["cues"][0]["correlation"], 0.98)
        self.assertEqual(result["status"], "fail")

    def test_alignment_accepts_both_directions_and_fifty_ms_boundary(self):
        for delay in (-30, 30, 50):
            with self.subTest(delay=delay):
                result = self.compare(self.shifted(delay))
                self.assertEqual(result["status"], "pass")
                self.assertAlmostEqual(result["cues"][0]["alignment_ms"], delay, places=4)

    def test_alignment_rejects_beyond_fifty_ms(self):
        self.assertEqual(self.compare(self.shifted(51))["status"], "fail")

    def test_level_accepts_half_db_and_rejects_two_db(self):
        for decibels, expected in ((0.5, "pass"), (-2, "fail"), (2, "fail")):
            with self.subTest(decibels=decibels):
                actual = [value * 10 ** (decibels / 20) for value in self.reference]
                result = self.compare(actual)
                self.assertEqual(result["status"], expected)
                self.assertAlmostEqual(result["cues"][0]["level_db"], decibels, places=4)

    def test_post_stop_silence_is_strict(self):
        for decibels, expected in ((-61, "pass"), (-60, "fail"), (-59, "fail")):
            with self.subTest(decibels=decibels):
                actual = self.reference.copy()
                actual[round(0.4 * self.rate) * 2] = 10 ** (decibels / 20)
                self.assertEqual(self.compare(actual)["status"], expected)

    def test_muted_cue_fails(self):
        result = self.compare([0.0] * len(self.reference))
        self.assertEqual(result["status"], "fail")
        self.assertFalse(result["cues"][0]["pass"])

    def test_one_muted_cue_cannot_hide_in_another(self):
        self.cues = self.cues + [{"name": "gem", "frame": 15, "window_ms": [220, 320]}]
        actual = self.reference.copy()
        self.reference[24000:26400] = self.reference[9600:12000]
        result = self.compare(actual)
        self.assertTrue(result["cues"][0]["pass"])
        self.assertFalse(result["cues"][1]["pass"])
        self.assertEqual(result["status"], "fail")

    def test_silent_reference_cannot_pass(self):
        self.reference = [0.0] * len(self.reference)
        self.assertEqual(self.compare(self.reference)["status"], "fail")

    def test_clipping_and_missing_silence_fail(self):
        actual = self.reference.copy()
        actual[0] = 1.0
        self.assertEqual(self.compare(actual)["status"], "fail")
        self.assertEqual(self.compare(self.reference[:12000])["status"], "fail")

    def test_stereo_polarity_is_preserved(self):
        actual = [value * (-1 if index % 2 else 1) for index, value in enumerate(self.reference)]
        self.assertEqual(self.compare(actual)["status"], "fail")

    def test_extra_cue_outside_windows_fails(self):
        actual = self.reference.copy()
        actual[24000:26400] = self.reference[9600:12000]
        self.assertEqual(self.compare(actual)["status"], "fail")


class EvidenceTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.output = Path(self.directory.name)
        (self.output / "audio").mkdir()
        (self.output / "audio/mixer.f32").write_bytes(struct.pack("<4800f", *([0.0] * 4800)))
        self.value = {"end_frame": 3, "stop_frame": 2, "mixer_rate": 48000, "cues": [{"name": "jump", "frame": 1}]}
        for frame in range(1, 4):
            fact = {
                "frame": frame,
                "sample_start": (frame - 1) * 800,
                "sample_count": 800,
                "discarded_frames": 0,
                "mixer_rate": 48000,
                "driver": "CoreAudio",
                "output_device": "test",
                "issued_ms": frame * 1000 / 60,
                "elapsed_ms": frame * 1000 / 60,
                "cue": "jump" if frame == 1 else None,
                "stop": frame == 2,
                "final": frame == 3,
            }
            (self.output / f"frame-{frame:06d}.audio.json").write_text(json.dumps(fact))

    def test_complete_drains_write_wav(self):
        samples, errors = audio._recording(self.value, self.output)
        self.assertEqual(errors, [])
        self.assertEqual(len(samples), 4800)
        self.assertTrue((self.output / "audio/capture.wav").is_file())

    def test_discarded_samples_dummy_driver_and_wrong_cue_fail(self):
        path = self.output / "frame-000001.audio.json"
        original = json.loads(path.read_text())
        for key, value in (("discarded_frames", 1), ("driver", "Dummy"), ("cue", "gem"), ("mixer_rate", 22050), ("issued_ms", 100)):
            with self.subTest(key=key):
                path.write_text(json.dumps(dict(original, **{key: value})))
                self.assertTrue(audio._recording(self.value, self.output)[1])

    def test_missing_frame_and_truncated_samples_fail(self):
        (self.output / "frame-000003.audio.json").unlink()
        with self.assertRaises(FileNotFoundError):
            audio._recording(self.value, self.output)
        (self.output / "audio/mixer.f32").write_bytes(b"x")
        with self.assertRaisesRegex(ValueError, "incomplete stereo"):
            audio._recording(self.value, self.output)


class ExtensionTests(unittest.TestCase):
    def setUp(self):
        self.validate = runpy.run_path(str(ROOT / "schema/validate.py"))["validate"]
        self.value = {
            "mixer_rate": 48000,
            "reference": "itest/capture/audio/references/platformer-2d/cues.wav",
            "cues": [
                {"name": "jump", "frame": 30, "window_ms": [450, 900]},
                {"name": "gem", "frame": 90, "window_ms": [1450, 1950]},
            ],
            "stop_frame": 100,
            "end_frame": 210,
            "silence_window_ms": [2700, 3400],
            "thresholds": SignalTests.thresholds.copy(),
        }

    def test_valid_extension(self):
        self.assertEqual(self.validate(self.value), [])

    def test_schema_rejects_weak_budgets_bad_order_and_paths(self):
        mutations = [
            ("mixer_rate", 22050),
            ("reference", "itest/capture/audio/references/../cues.wav"),
            ("reference", "/tmp/cues.wav"),
            ("stop_frame", True),
            ("cues", list(reversed(self.value["cues"]))),
            ("silence_window_ms", [1600, 3400]),
            ("unknown", 1),
        ]
        for key, value in mutations:
            with self.subTest(key=key, value=value):
                extension = copy.deepcopy(self.value)
                extension[key] = value
                self.assertTrue(self.validate(extension))
        for key, value in (("correlation", 0.5), ("alignment_ms", 51), ("level_db", 2), ("silence_dbfs", -59)):
            extension = copy.deepcopy(self.value)
            extension["thresholds"][key] = value
            self.assertTrue(self.validate(extension))

    def test_missing_reference_fails_with_approval_instruction(self):
        with tempfile.TemporaryDirectory() as directory:
            result = audio.evaluate(self.value, Path(directory), reference_root=Path(directory))
        self.assertEqual(result["status"], "fail")
        self.assertTrue(any("reviewed reference WAV is missing" in error for error in result["errors"]))

    def test_verdict_uses_runpy_and_sets_only_audio(self):
        callback = runpy.run_path(str(ROOT / "verdict.py"))["verdict"]
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            self.assertEqual(callback(self.value, output), {"audio": "fail"})
            self.assertTrue((output / "audio/audio-verdict.json").is_file())
