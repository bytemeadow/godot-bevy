import http.client
import importlib.util
from pathlib import Path
import runpy
import struct
import tempfile
import threading
import unittest
import zlib


ADAPTER = Path(__file__).resolve().parents[1]


def load(name, path):
    spec = importlib.util.spec_from_file_location(name, path)
    module = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(module)
    return module


serve = load("browser_serve", ADAPTER / "serve.py")
verdict = load("browser_verdict", ADAPTER / "verdict.py")


class HeaderTests(unittest.TestCase):
    def test_headers_on_get_head_and_missing_files(self):
        for variant in ("web", "web-nothreads"):
            with self.subTest(variant=variant), tempfile.TemporaryDirectory() as directory:
                root = Path(directory)
                (root / "index.html").write_text("<canvas></canvas>")
                (root / "game.wasm").write_bytes(b"\0asm")
                server = serve.make_server(root, variant, port=0)
                worker = threading.Thread(target=server.serve_forever, daemon=True)
                worker.start()
                try:
                    for method, path, status in (
                        ("GET", "/", 200),
                        ("HEAD", "/game.wasm", 200),
                        ("GET", "/missing", 404),
                    ):
                        connection = http.client.HTTPConnection(*server.server_address, timeout=5)
                        try:
                            connection.request(method, path)
                            response = connection.getresponse()
                            response.read()
                            self.assertEqual(response.status, status)
                            for header, expected in (
                                ("Cross-Origin-Opener-Policy", "same-origin"),
                                ("Cross-Origin-Embedder-Policy", "require-corp"),
                            ):
                                self.assertEqual(
                                    response.getheader(header),
                                    expected if variant == "web" else None,
                                )
                            if path.endswith(".wasm"):
                                self.assertEqual(response.getheader("Content-Type"), "application/wasm")
                        finally:
                            connection.close()
                finally:
                    server.shutdown()
                    server.server_close()
                    worker.join()

    def test_unknown_variant_is_rejected(self):
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaises(ValueError):
                serve.make_server(Path(directory), "threadish", port=0)


class MotionTests(unittest.TestCase):
    def compare(self, before, after, **options):
        return verdict.motion_difference(before, after, 10, 10, **options)

    def test_identical_frames_have_no_motion(self):
        result = self.compare(bytes(300), bytes(300))
        self.assertEqual(result["changed_pixels"], 0)
        self.assertFalse(result["detected"])

    def test_translated_sprite_exceeds_threshold(self):
        before = bytearray(300)
        after = bytearray(300)
        before[0:12] = bytes([255]) * 12
        after[30:42] = bytes([255]) * 12
        result = self.compare(before, after, threshold=0.05)
        self.assertEqual(result["changed_pixels"], 8)
        self.assertTrue(result["detected"])

    def test_fraction_must_be_strictly_above_threshold(self):
        after = bytes([255, 0, 0]) + bytes(297)
        self.assertFalse(self.compare(bytes(300), after, threshold=0.01)["detected"])
        self.assertTrue(self.compare(bytes(300), after, threshold=0.009)["detected"])

    def test_channel_difference_must_exceed_tolerance(self):
        self.assertFalse(self.compare(bytes(300), bytes([8]) * 300)["detected"])
        self.assertTrue(self.compare(bytes(300), bytes([9, 0, 0]) * 100)["detected"])

    def test_malformed_frames_and_thresholds_are_rejected(self):
        for before, after in ((b"", b""), (bytes(299), bytes(300)), (bytes(300), bytes(303))):
            with self.subTest(lengths=(len(before), len(after))), self.assertRaises(ValueError):
                self.compare(before, after)
        for threshold in (0, -0.1, 1, float("nan"), float("inf")):
            with self.subTest(threshold=threshold), self.assertRaises(ValueError):
                self.compare(bytes(300), bytes(300), threshold=threshold)

    def test_screenshot_png_decodes_rgb_and_rgba_filters(self):
        def chunk(kind, data):
            return struct.pack(">I", len(data)) + kind + data + struct.pack(">I", zlib.crc32(kind + data))

        for colour_type, pixel in ((2, b"\x0a\x14\x1e"), (6, b"\x0a\x14\x1e\xff")):
            for filter_type in range(5):
                with self.subTest(colour_type=colour_type, filter=filter_type):
                    data = b"\x89PNG\r\n\x1a\n"
                    data += chunk(b"IHDR", struct.pack(">IIBBBBB", 1, 1, 8, colour_type, 0, 0, 0))
                    data += chunk(b"IDAT", zlib.compress(bytes([filter_type]) + pixel))
                    data += chunk(b"IEND", b"")
                    with tempfile.TemporaryDirectory() as directory:
                        path = Path(directory) / "frame.png"
                        path.write_bytes(data)
                        self.assertEqual(verdict.read_png(path), (1, 1, b"\x0a\x14\x1e"))
                        path.write_bytes(data[:-1])
                        with self.assertRaises(ValueError):
                            verdict.read_png(path)

    def test_png_filters_reconstruct_neighbouring_pixels_and_rows(self):
        encoded = (
            ([10, 20, 30, 40, 50, 60], [11, 22, 33, 44, 55, 66]),
            ([10, 20, 30, 30, 30, 30], [11, 22, 33, 33, 33, 33]),
            ([10, 20, 30, 40, 50, 60], [1, 2, 3, 4, 5, 6]),
            ([10, 20, 30, 35, 40, 45], [6, 12, 18, 19, 19, 20]),
            ([10, 20, 30, 30, 30, 30], [1, 2, 3, 4, 5, 6]),
        )
        for mode, (first, second) in enumerate(encoded):
            with self.subTest(filter=mode), tempfile.TemporaryDirectory() as directory:
                raw = bytes([mode] + first + [mode] + second)
                data = b"\x89PNG\r\n\x1a\n"
                for kind, payload in (
                    (b"IHDR", struct.pack(">IIBBBBB", 2, 2, 8, 2, 0, 0, 0)),
                    (b"IDAT", zlib.compress(raw)),
                    (b"IEND", b""),
                ):
                    data += struct.pack(">I", len(payload)) + kind + payload
                    data += struct.pack(">I", zlib.crc32(kind + payload))
                path = Path(directory) / "frame.png"
                path.write_bytes(data)
                self.assertEqual(
                    verdict.read_png(path),
                    (2, 2, bytes([10, 20, 30, 40, 50, 60, 11, 22, 33, 44, 55, 66])),
                )


class ContractTests(unittest.TestCase):
    def test_only_empty_extension_is_accepted(self):
        validate = runpy.run_path(str(ADAPTER / "schema/validate.py"))["validate"]
        self.assertEqual(validate({}), [])
        for value in (None, [], False, 0, "", {"enabled": True}):
            with self.subTest(value=value):
                self.assertTrue(validate(value))

    def test_verdict_is_unconditionally_untested(self):
        check = runpy.run_path(str(ADAPTER / "verdict.py"))["verdict"]
        with tempfile.TemporaryDirectory() as directory:
            output = Path(directory)
            (output / "browser").mkdir()
            (output / "browser/probe.json").write_text('{"motion":{"detected":true}}')
            for value in ({}, None, {"browser": "pass"}):
                self.assertEqual(check(value, output), {"browser": "untested"})
        self.assertEqual(check({}, Path("/does-not-exist")), {"browser": "untested"})


if __name__ == "__main__":
    unittest.main()
