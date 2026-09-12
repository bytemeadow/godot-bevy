from __future__ import annotations

import contextlib
import io
import json
import os
import signal
import struct
import subprocess
import sys
import tempfile
import time
import unittest
import zlib
from pathlib import Path
from unittest.mock import patch

sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

import capture
from capture_schema import frame_file
from test_capture import facts, manifest


def png(viewport, row_filter=0):
    def chunk(kind, data):
        return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

    w, h = viewport
    return (b"\x89PNG\r\n\x1a\n"
            + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0))
            + chunk(b"IDAT", zlib.compress((bytes([row_filter]) + b"\0" * w * 3) * h))
            + chunk(b"IEND", b""))


class EvidenceTests(unittest.TestCase):
    def test_valid_png_is_evidence_and_is_not_a_pixel_oracle(self):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            value = manifest()
            session = capture.CaptureSession(value, root)
            for name in ("manifest.json", "stdout.log", "stderr.log"):
                (root / name).write_text("")
            session.accept("CAPTURE_READY scenario=orbit scene=res://main.tscn frame=0")
            for frame in (0, 60):
                (root / frame_file(frame, "png")).write_bytes(png(value["viewport"]))
                session.accept(f"CAPTURE_FACTS frame={frame} " + json.dumps(facts(frame)))
            for name in ("driver.pid", "godot.pid"):
                (root / name).write_text("1\n")
            capture.write_json(root / "instruction.json", session.instruction())
            session.accept("CAPTURE_ACK exit=0")
            self.assertEqual(session.verdict()["exit_code"], 0)

    def test_truncated_png_cannot_pass(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "frame.png"
            path.write_bytes(png([2, 2])[:-12])
            self.assertIsNotNone(capture.png_error(path, [2, 2]))

    def test_invalid_png_filter_cannot_pass(self):
        with tempfile.TemporaryDirectory() as directory:
            path = Path(directory) / "frame.png"
            path.write_bytes(png([2, 2], row_filter=5))
            self.assertIsNotNone(capture.png_error(path, [2, 2]))

    def test_out_of_order_or_duplicate_checkpoint_fails(self):
        with tempfile.TemporaryDirectory() as directory:
            session = capture.CaptureSession(manifest(), Path(directory))
            session.accept("CAPTURE_READY scenario=orbit scene=res://main.tscn frame=0")
            with self.assertRaises(ValueError):
                session.accept("CAPTURE_FACTS frame=60 " + json.dumps(facts(60)))


@unittest.skipUnless(os.name == "posix", "process-group checks require POSIX")
class ProcessTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        project = self.root / "examples/sample/godot"
        (project / ".godot").mkdir(parents=True)
        (project / "rust.gdextension").write_text("")
        self.output = self.root / "evidence"
        self.output.mkdir()

    def run_engine(self, body, timeout=3):
        engine = self.root / "engine"
        engine.write_text(f"#!{sys.executable}\n" + body)
        engine.chmod(0o755)
        with patch.object(capture, "REPOSITORY", self.root), \
                patch.object(capture, "require_capture_library"), contextlib.redirect_stdout(io.StringIO()):
            code = capture.run_capture(manifest(), self.output, str(engine), timeout)
        return code, json.loads((self.output / "verdict.json").read_text())

    def test_exit_zero_without_ready_is_incomplete(self):
        code, verdict = self.run_engine("print('engine startup only')\n")
        self.assertEqual(code, 1)
        self.assertFalse(verdict["complete"])
        self.assertIn("frame-000000.png", verdict["missing_artifacts"])

    def test_driver_delivers_and_preserves_the_comparison_verdict(self):
        value = manifest()
        encoded_png = png(value["viewport"]).hex()
        points = [facts(0), facts(60)]
        points[1]["nodes"][0]["visible"] = False
        body = (
            "import json, os, pathlib, time\n"
            "root = pathlib.Path(os.environ['GODOT_BEVY_CAPTURE_OUTPUT'])\n"
            "print('CAPTURE_READY scenario=orbit scene=res://main.tscn frame=0', flush=True)\n"
            f"for point in {points!r}:\n"
            "    frame = point['frame']\n"
            "    prefix = root / f'frame-{frame:06d}'\n"
            "    while not prefix.with_suffix('.request.json').exists(): time.sleep(0.01)\n"
            f"    prefix.with_suffix('.png').write_bytes(bytes.fromhex({encoded_png!r}))\n"
            "    print(f'CAPTURE_FACTS frame={frame} ' + json.dumps(point), flush=True)\n"
            "while not (root / 'instruction.json').exists(): time.sleep(0.01)\n"
            "code = json.loads((root / 'instruction.json').read_text())['exit_code']\n"
            "print(f'CAPTURE_ACK exit={code}', flush=True)\n"
            "raise SystemExit(code)\n"
        )
        code, verdict = self.run_engine(body)
        self.assertEqual(code, 1)
        self.assertTrue(verdict["complete"])
        self.assertEqual(verdict["errors"], [])
        self.assertEqual(verdict["differences"][0]["path"], "nodes.Icon.visible")

    def test_timeout_retains_evidence_and_terminates_owned_process(self):
        code, verdict = self.run_engine(
            "import os, time\nprint(os.getpid(), flush=True)\ntime.sleep(30)\n", timeout=1,
        )
        self.assertEqual(code, 1)
        self.assertTrue(verdict["errors"])
        pid = int((self.output / "stdout.log").read_text().strip())
        with self.assertRaises(ProcessLookupError):
            os.kill(pid, 0)

    def test_sigterm_retains_verdict_and_terminates_godot(self):
        engine = self.root / "engine"
        engine.write_text(f"#!{sys.executable}\nimport os, time\nprint(os.getpid(), flush=True)\ntime.sleep(30)\n")
        engine.chmod(0o755)
        script = (
            f"import sys\nsys.path.insert(0, {str(Path(capture.__file__).parent)!r})\n"
            "from pathlib import Path\nimport capture\ncapture.require_capture_library = lambda project: None\n"
            f"capture.REPOSITORY = Path({str(self.root)!r})\n"
            f"raise SystemExit(capture.run_capture({manifest()!r}, Path({str(self.output)!r}), {str(engine)!r}, 10))\n"
        )
        driver = subprocess.Popen([sys.executable, "-c", script], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        try:
            deadline = time.monotonic() + 5
            log = self.output / "stdout.log"
            while not log.is_file() or not log.read_text().strip():
                if time.monotonic() >= deadline:
                    self.fail("child did not start")
                time.sleep(0.01)
            pid = int(log.read_text().strip())
            driver.send_signal(signal.SIGTERM)
            driver.communicate(timeout=5)
            self.assertEqual(driver.returncode, 1)
            self.assertTrue((self.output / "verdict.json").is_file())
            with self.assertRaises(ProcessLookupError):
                os.kill(pid, 0)
        finally:
            if driver.poll() is None:
                driver.kill()
            driver.communicate()

    def completed_engine(self, *, mismatch=False, acknowledge=True, exit_code=None, after_instruction=""):
        points = [facts(0), facts(60)]
        if mismatch:
            points[1]["nodes"][0]["visible"] = False
        body = (
            "import json, os, pathlib, time\n"
            "root = pathlib.Path(os.environ['GODOT_BEVY_CAPTURE_OUTPUT'])\n"
            "print('CAPTURE_READY scenario=orbit scene=res://main.tscn frame=0', flush=True)\n"
            f"for point in {points!r}:\n"
            "    frame = point['frame']\n"
            f"    (root / f'frame-{{frame:06d}}.png').write_bytes(bytes.fromhex({png(manifest()['viewport']).hex()!r}))\n"
            "    print(f'CAPTURE_FACTS frame={frame} ' + json.dumps(point), flush=True)\n"
        )
        if acknowledge:
            body += (
                "while not (root / 'instruction.json').exists(): time.sleep(0.01)\n"
                "assert not (root / 'verdict.json').exists()\n"
                "code = json.loads((root / 'instruction.json').read_text())['exit_code']\n"
                + after_instruction +
                "print(f'CAPTURE_ACK exit={code}', flush=True)\n"
                f"raise SystemExit({'code' if exit_code is None else exit_code})\n"
            )
        return body

    def test_exit_zero_requires_instruction_ack_and_preserves_pids(self):
        code, verdict = self.run_engine(self.completed_engine())
        self.assertEqual(code, 0)
        self.assertTrue(verdict['complete'])
        self.assertEqual(verdict['errors'], [])
        self.assertEqual(int((self.output / 'driver.pid').read_text()), os.getpid())
        self.assertGreater(int((self.output / 'godot.pid').read_text()), 0)
        self.assertIn('instruction.json', verdict['required_artifacts'])

    def test_exit_zero_without_reading_instruction_fails(self):
        code, verdict = self.run_engine(self.completed_engine(acknowledge=False))
        self.assertEqual(code, 1)
        self.assertFalse(verdict['complete'])
        self.assertTrue(any('acknowledge' in error for error in verdict['errors']))

    def test_wrong_exit_after_ack_is_incomplete(self):
        for mismatch, status in ((False, 3), (True, 0)):
            with self.subTest(mismatch=mismatch, status=status):
                code, verdict = self.run_engine(self.completed_engine(mismatch=mismatch, exit_code=status))
                self.assertEqual(code, 1)
                self.assertFalse(verdict['complete'])
                self.assertTrue(any('exited with code' in e for e in verdict['errors']))
                for file in self.output.iterdir():
                    if file.is_file():
                        file.unlink()

    def test_capture_error_retains_terminal_failure(self):
        code, verdict = self.run_engine("print('CAPTURE_ERROR \"adapter failed\"', flush=True)\n")
        self.assertEqual(code, 1)
        self.assertIn('adapter failed', ' '.join(verdict['errors']))

    def test_stop_failures_preserve_timeout_exit_and_verdict(self):
        stop = capture._stop
        for failure in (PermissionError('denied'), subprocess.TimeoutExpired('engine', 5)):
            def failing_stop(process):
                stop(process)
                raise failure
            with self.subTest(failure=failure), patch.object(capture, '_stop', side_effect=failing_stop):
                code, verdict = self.run_engine('import time\ntime.sleep(30)\n', timeout=0.1)
                self.assertEqual(code, 1)
                self.assertTrue(any('timed out' in e for e in verdict['errors']))
                self.assertTrue(any('cleanup' in e for e in verdict['errors']))
    def test_configuration_errors_inside_run_capture_exit_two(self):
        with patch.dict(os.environ, {'GODOT_BEVY_ITEST': '1'}):
            code, verdict = self.run_engine('raise RuntimeError("must not launch")\n')
        self.assertEqual(code, 2)
        self.assertFalse(verdict['complete'])
        (self.root / 'examples/sample/godot/.godot').rmdir()
        code, verdict = self.run_engine('raise RuntimeError("must not launch")\n')
        self.assertEqual(code, 2)
        self.assertIn('imported project', ' '.join(verdict['errors']))

    def start_driver(self, body):
        engine = self.root / 'engine'
        engine.write_text(f'#!{sys.executable}\n' + body)
        engine.chmod(0o755)
        script = (
            f"import sys\nsys.path.insert(0, {str(Path(capture.__file__).parent)!r})\n"
            "from pathlib import Path\nimport capture\n"
            "capture.require_capture_library = lambda project: None\n"
            f"capture.REPOSITORY = Path({str(self.root)!r})\n"
            f"raise SystemExit(capture.run_capture({manifest()!r}, Path({str(self.output)!r}), {str(engine)!r}, 10))\n"
        )
        driver = subprocess.Popen([sys.executable, '-c', script], stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        self.addCleanup(self.cleanup_driver, driver)
        return driver

    def cleanup_driver(self, driver):
        if driver.poll() is None:
            driver.kill()
        driver.communicate(timeout=5)
        path = self.output / 'godot.pid'
        if path.exists():
            try:
                os.killpg(int(path.read_text()), signal.SIGKILL)
            except ProcessLookupError:
                pass

    def wait_for_file(self, path):
        deadline = time.monotonic() + 5
        while not path.exists() or not path.read_text().strip():
            if time.monotonic() >= deadline:
                self.fail(f'not written: {path}')
            time.sleep(0.01)

    def test_second_signal_during_cleanup_cannot_skip_verdict_or_sigkill(self):
        driver = self.start_driver(
            "import os, pathlib, signal, time\n"
            "root = pathlib.Path(os.environ['GODOT_BEVY_CAPTURE_OUTPUT'])\n"
            "signal.signal(signal.SIGTERM, lambda *_: (root / 'terminating').write_text('yes'))\n"
            "print(os.getpid(), flush=True)\ntime.sleep(30)\n"
        )
        self.wait_for_file(self.output / 'stdout.log')
        driver.send_signal(signal.SIGTERM)
        self.wait_for_file(self.output / 'terminating')
        driver.send_signal(signal.SIGINT)
        driver.send_signal(signal.SIGTERM)
        driver.communicate(timeout=10)
        self.assertEqual(driver.returncode, 1)
        self.assertTrue((self.output / 'verdict.json').exists())
        with self.assertRaises(ProcessLookupError):
            os.kill(int((self.output / 'godot.pid').read_text()), 0)

    def test_driver_sigkill_leaves_locatable_group_and_no_pass_verdict(self):
        driver = self.start_driver(self.completed_engine(after_instruction='time.sleep(30)\n'))
        self.wait_for_file(self.output / 'instruction.json')
        driver.kill()
        driver.communicate(timeout=5)
        self.assertFalse((self.output / 'verdict.json').exists())
        self.assertEqual(int((self.output / 'driver.pid').read_text()), driver.pid)
        pid = int((self.output / 'godot.pid').read_text())
        self.assertEqual(os.getpgid(pid), pid)

    def test_realtime_launch_omits_fixed_fps_and_prequeues_requests(self):
        value = manifest()
        value['pacing'] = 'realtime'
        body = (
            "import os, pathlib, sys\n"
            "root = pathlib.Path(os.environ['GODOT_BEVY_CAPTURE_OUTPUT'])\n"
            "assert '--fixed-fps' not in sys.argv\n"
            "assert len(list(root.glob('*.request.json'))) == 2\n"
        ) + self.completed_engine()
        with patch(f'{__name__}.manifest', return_value=value):
            code, verdict = self.run_engine(body)
        self.assertEqual(code, 0)
        self.assertNotIn('--fixed-fps', verdict['command'])


if __name__ == "__main__":
    unittest.main()
