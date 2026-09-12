import argparse
import json
import math
from pathlib import Path
import struct
import zlib


def verdict(value, output: Path) -> dict[str, str]:
    return {"browser": "untested"}


def motion_difference(before, after, width, height, *, threshold=0.001, tolerance=8):
    if width <= 0 or height <= 0 or len(before) != width * height * 3 or len(after) != len(before):
        raise ValueError("frames must contain equal, nonempty RGB8 images of the declared size")
    if not math.isfinite(threshold) or not 0 < threshold < 1:
        raise ValueError("motion threshold must be finite and strictly between 0 and 1")
    if not isinstance(tolerance, int) or not 0 <= tolerance < 255:
        raise ValueError("channel tolerance must be an integer in [0,254]")
    changed = sum(
        any(abs(before[index + channel] - after[index + channel]) > tolerance for channel in range(3))
        for index in range(0, len(before), 3)
    )
    fraction = changed / (width * height)
    return {
        "changed_pixels": changed,
        "total_pixels": width * height,
        "changed_fraction": fraction,
        "threshold": threshold,
        "channel_tolerance": tolerance,
        "detected": fraction > threshold,
    }


def read_png(path):
    data = Path(path).read_bytes()
    if data[:8] != b"\x89PNG\r\n\x1a\n":
        raise ValueError("screenshot is not a PNG")
    offset = 8
    header = None
    compressed = bytearray()
    ended = False
    while offset < len(data):
        if offset + 12 > len(data):
            raise ValueError("truncated PNG chunk")
        length, kind = struct.unpack_from(">I4s", data, offset)
        end = offset + 8 + length
        if end + 4 > len(data):
            raise ValueError("truncated PNG chunk")
        payload = data[offset + 8:end]
        if zlib.crc32(kind + payload) != struct.unpack_from(">I", data, end)[0]:
            raise ValueError("invalid PNG checksum")
        if header is None and kind != b"IHDR":
            raise ValueError("PNG must start with IHDR")
        if kind == b"IHDR":
            if header is not None or len(payload) != 13:
                raise ValueError("invalid PNG header")
            header = struct.unpack(">IIBBBBB", payload)
        elif kind == b"IDAT":
            compressed.extend(payload)
        elif kind == b"IEND":
            if payload or end + 4 != len(data):
                raise ValueError("invalid PNG end")
            ended = True
            break
        elif kind[:1].isupper() and kind != b"PLTE":
            raise ValueError("unsupported PNG critical chunk")
        offset = end + 4
    if not ended or header is None or not compressed:
        raise ValueError("incomplete PNG")
    width, height, depth, colour, compression, filtering, interlace = header
    if not (0 < width <= 8192 and 0 < height <= 8192):
        raise ValueError("PNG dimensions must be in [1,8192]")
    if depth != 8 or colour not in (2, 6) or compression or filtering or interlace:
        raise ValueError("expected a non-interlaced RGB8 or RGBA8 screenshot")
    channels = 3 if colour == 2 else 4
    stride = width * channels
    expected_size = (stride + 1) * height
    decoder = zlib.decompressobj()
    raw = decoder.decompress(compressed, expected_size + 1)
    if len(raw) != expected_size or not decoder.eof or decoder.unused_data:
        raise ValueError("invalid PNG pixel data")
    previous = bytearray(stride)
    rgb = bytearray()
    for row_index in range(height):
        start = row_index * (stride + 1)
        mode = raw[start]
        if mode > 4:
            raise ValueError("invalid PNG filter")
        row = bytearray(raw[start + 1:start + 1 + stride])
        for index in range(stride):
            left = row[index - channels] if index >= channels else 0
            above = previous[index]
            corner = previous[index - channels] if index >= channels else 0
            predictor = 0
            if mode == 1:
                predictor = left
            elif mode == 2:
                predictor = above
            elif mode == 3:
                predictor = (left + above) // 2
            elif mode == 4:
                estimate = left + above - corner
                predictor = min((left, above, corner), key=lambda value: abs(estimate - value))
            row[index] = (row[index] + predictor) & 255
        if channels == 3:
            rgb.extend(row)
        else:
            for index in range(0, stride, 4):
                rgb.extend(row[index:index + 3])
        previous = row
    return width, height, bytes(rgb)


def main():
    parser = argparse.ArgumentParser(description="Measure browser screenshot motion; browser remains untested.")
    parser.add_argument("before", type=Path)
    parser.add_argument("after", type=Path)
    parser.add_argument("--threshold", type=float, default=0.001)
    parser.add_argument("--tolerance", type=int, default=8)
    args = parser.parse_args()
    result = {"browser": "untested"}
    code = 1
    try:
        width, height, before = read_png(args.before)
        after_width, after_height, after = read_png(args.after)
        if (width, height) != (after_width, after_height):
            raise ValueError("screenshot dimensions differ")
        result["viewport"] = [width, height]
        result["motion"] = motion_difference(
            before, after, width, height, threshold=args.threshold, tolerance=args.tolerance
        )
        code = 0 if result["motion"]["detected"] else 1
    except (OSError, ValueError, zlib.error) as error:
        result["error"] = str(error)
    print(json.dumps(result, allow_nan=False))
    return code


if __name__ == "__main__":
    raise SystemExit(main())
