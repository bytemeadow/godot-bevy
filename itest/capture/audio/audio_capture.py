from array import array
import cmath
import json
import math
from pathlib import Path
import runpy
import sys
import wave


ADAPTER = Path(__file__).resolve().parent
REPOSITORY = ADAPTER.parents[2]


def _fft(values, inverse=False):
    size = len(values)
    other = 0
    for index in range(1, size):
        bit = size >> 1
        while other & bit:
            other ^= bit
            bit >>= 1
        other ^= bit
        if index < other:
            values[index], values[other] = values[other], values[index]
    width = 2
    while width <= size:
        step = cmath.exp((2j if inverse else -2j) * math.pi / width)
        half = width // 2
        for start in range(0, size, width):
            factor = 1
            for index in range(start, start + half):
                left, right = values[index], values[index + half] * factor
                values[index], values[index + half] = left + right, left - right
                factor *= step
        width *= 2
    if inverse:
        for index in range(size):
            values[index] /= size


def _convolve(left, right):
    size = 1 << (len(left) + len(right) - 2).bit_length()
    left = list(left) + [0j] * (size - len(left))
    right = list(right) + [0j] * (size - len(right))
    _fft(left)
    _fft(right)
    for index in range(size):
        left[index] *= right[index]
    _fft(left, inverse=True)
    return left


def _cue(reference, actual, rate, cue, thresholds):
    start, end = (round(ms * rate / 1000) * 2 for ms in cue["window_ms"])
    margin = round(thresholds["alignment_ms"] * rate / 1000)
    template = reference[start:end]
    result = {"name": cue["name"], "correlation": 0.0, "alignment_ms": None, "level_db": None, "pass": False}
    if start < margin * 2 or end > len(reference) or end + margin * 2 > len(actual):
        result["error"] = "cue window or alignment samples are missing"
        return result
    reference_energy = sum(value * value for value in template)
    if reference_energy <= len(template) * 1e-12:
        result["error"] = "reference cue is silent"
        return result
    search = actual[start - margin * 2:end + margin * 2]
    products = _convolve(template[::-1], search)
    energy = [0.0]
    for value in search:
        energy.append(energy[-1] + value * value)
    best = (-1.0, 0, 0.0)
    for shift in range(-margin, margin + 1):
        offset = 2 * (shift + margin)
        measured_energy = max(0.0, energy[offset + len(template)] - energy[offset])
        correlation = 0.0
        if measured_energy > 0:
            correlation = products[offset + len(template) - 1].real / math.sqrt(reference_energy * measured_energy)
        if correlation > best[0]:
            best = (correlation, shift, measured_energy)
    correlation, shift, measured_energy = best
    level = 10 * math.log10(measured_energy / reference_energy) if measured_energy > 0 else None
    result.update(
        correlation=max(-1.0, min(1.0, correlation)),
        alignment_ms=shift * 1000 / rate,
        level_db=level,
    )
    result["pass"] = correlation >= thresholds["correlation"] and level is not None and abs(level) <= thresholds["level_db"] + 1e-9
    return result


def compare_signals(reference, actual, rate, cues, thresholds, silence_window_ms):
    errors = []
    if not reference or not actual or len(reference) % 2 or len(actual) % 2:
        return {"status": "fail", "errors": ["missing or incomplete stereo recording"], "cues": []}
    if any(not math.isfinite(value) for signal in (reference, actual) for value in signal):
        return {"status": "fail", "errors": ["non-finite audio samples"], "cues": []}
    for name, signal in (("reference", reference), ("recording", actual)):
        if max(abs(value) for value in signal) >= 1:
            errors.append(f"{name} contains clipping")
        cursor = 0
        for cue in cues:
            start = round((cue["window_ms"][0] - thresholds["alignment_ms"]) * rate / 1000) * 2
            end = round((cue["window_ms"][1] + thresholds["alignment_ms"]) * rate / 1000) * 2
            if any(abs(value) >= 10 ** (thresholds["silence_dbfs"] / 20) for value in signal[cursor:start]):
                errors.append(f"{name}: unexpected audio before {cue['name']}")
            cursor = end
        if any(abs(value) >= 10 ** (thresholds["silence_dbfs"] / 20) for value in signal[cursor:]):
            errors.append(f"{name}: unexpected audio after the cue windows")
    if abs(len(reference) - len(actual)) / 2 > rate * thresholds["alignment_ms"] / 1000:
        errors.append("recording duration differs from the reference by more than 50 ms")
    results = [_cue(reference, actual, rate, cue, thresholds) for cue in cues]
    errors.extend(f"{cue['name']}: correlation, alignment or level failed" for cue in results if not cue["pass"])
    start, end = (round(ms * rate / 1000) * 2 for ms in silence_window_ms)
    peak = None
    for name, signal in (("reference", reference), ("recording", actual)):
        if end > len(signal) or end <= start:
            errors.append(f"{name}: post-stop silence samples are missing")
            continue
        measured = max(abs(value) for value in signal[start:end])
        if name == "recording":
            peak = measured
        if measured >= 10 ** (thresholds["silence_dbfs"] / 20):
            errors.append(f"{name}: post-stop silence must be below -60 dBFS peak")
    return {
        "status": "fail" if errors else "pass",
        "errors": errors,
        "cues": results,
        "post_stop_peak": peak,
        "post_stop_peak_dbfs": 20 * math.log10(peak) if peak else None,
    }


def read_wav(path, rate):
    with wave.open(str(path), "rb") as source:
        if source.getnchannels() != 2 or source.getsampwidth() != 2 or source.getframerate() != rate:
            raise ValueError(f"reference WAV must be stereo PCM16 at {rate} Hz")
        raw = source.readframes(source.getnframes())
        if len(raw) != source.getnframes() * 4:
            raise ValueError("reference WAV is truncated")
    samples = array("h")
    samples.frombytes(raw)
    if sys.byteorder != "little":
        samples.byteswap()
    if any(value in (-32768, 32767) for value in samples):
        raise ValueError("reference WAV contains clipped PCM16 samples")
    return [value / 32768 for value in samples]


def write_wav(path, samples, rate):
    encoded = array("h", (max(-32768, min(32767, round(value * 32768))) for value in samples))
    if sys.byteorder != "little":
        encoded.byteswap()
    with wave.open(str(path), "wb") as output:
        output.setparams((2, 2, rate, 0, "NONE", "not compressed"))
        output.writeframes(encoded.tobytes())


def _recording(value, output):
    errors = []
    raw = (output / "audio/mixer.f32").read_bytes()
    if len(raw) % 8 or not raw:
        raise ValueError("audio/mixer.f32: missing or incomplete stereo float32 samples")
    samples = array("f")
    samples.frombytes(raw)
    if sys.byteorder != "little":
        samples.byteswap()
    if any(not math.isfinite(sample) for sample in samples):
        raise ValueError("audio/mixer.f32: non-finite samples")
    if any(abs(sample) >= 1 for sample in samples):
        errors.append("audio/mixer.f32: recording contains clipping")
    write_wav(output / "audio/capture.wav", samples, value["mixer_rate"])
    count, previous_time = 0, 0.0
    expected_cues = {cue["frame"]: cue["name"] for cue in value["cues"]}
    devices = set()
    for frame in range(1, value["end_frame"] + 1):
        path = output / f"frame-{frame:06d}.audio.json"
        fact = json.loads(path.read_text(encoding="utf-8"))
        if fact["frame"] != frame or fact["sample_start"] != count or type(fact["sample_count"]) is not int or fact["sample_count"] < 0:
            raise ValueError(f"{path.name}: discontinuous samples or wrong frame")
        count += fact["sample_count"]
        if fact["mixer_rate"] != value["mixer_rate"] or fact["discarded_frames"] != 0:
            errors.append(f"frame {frame}: wrong mixer rate or discarded samples")
        driver = fact["driver"]
        if not isinstance(driver, str) or not driver or driver.lower() == "dummy":
            errors.append(f"frame {frame}: real audio driver required")
        devices.add((driver, fact["output_device"]))
        issued, elapsed = fact["issued_ms"], fact["elapsed_ms"]
        if not all(type(t) in (int, float) and math.isfinite(t) for t in (issued, elapsed)):
            raise ValueError(f"frame {frame}: invalid wall clock")
        if not previous_time <= issued <= elapsed or not frame * 1000 / 60 - 1 <= issued <= elapsed <= frame * 1000 / 60 + 50:
            errors.append(f"frame {frame}: realtime pacing exceeded the 50 ms budget")
        if abs(count * 1000 / value["mixer_rate"] - elapsed) > 50:
            errors.append(f"frame {frame}: mixer sample clock differs from wall time by more than 50 ms")
        if fact["cue"] != expected_cues.get(frame) or fact["stop"] != (frame == value["stop_frame"]):
            errors.append(f"frame {frame}: cue order, frame or stop command differs")
        if fact["final"] != (frame == value["end_frame"]):
            errors.append(f"frame {frame}: missing or premature final drain")
        previous_time = elapsed
    if count * 2 != len(samples):
        errors.append("audio/mixer.f32: sample count does not match all per-frame drains")
    if len(devices) != 1:
        errors.append("audio driver or output device changed during capture")
    return list(samples), errors


def evaluate(value, output, reference_root=REPOSITORY):
    validation = runpy.run_path(str(ADAPTER / "schema/validate.py"))["validate"](value)
    if validation:
        return {"status": "fail", "errors": validation, "cues": []}
    errors = []
    reference_path = reference_root / value["reference"]
    reference = None
    if not reference_path.is_file():
        errors.append(f"reviewed reference WAV is missing: {value['reference']}; record and approve it using itest/capture/audio/README.md")
    else:
        try:
            if not reference_path.resolve().is_relative_to((reference_root / "itest/capture/audio/references").resolve()):
                raise ValueError("reference WAV escapes the audio references directory")
            reference = read_wav(reference_path, value["mixer_rate"])
        except (OSError, ValueError, wave.Error, EOFError) as error:
            errors.append(str(error))
    measured = None
    try:
        measured, capture_errors = _recording(value, output)
        errors.extend(capture_errors)
    except (OSError, ValueError, KeyError, TypeError) as error:
        errors.append(f"audio evidence: {error}")
    result = {"cues": []}
    if reference is not None and measured is not None:
        result = compare_signals(reference, measured, value["mixer_rate"], value["cues"], value["thresholds"], value["silence_window_ms"])
        errors.extend(result["errors"])
    result.update(
        status="fail" if errors else "pass",
        errors=errors,
        reference=value["reference"],
        required_artifacts=["audio/mixer.f32", "audio/capture.wav", "audio/audio-verdict.json"]
        + [f"frame-{frame:06d}.audio.json" for frame in range(1, value["end_frame"] + 1)],
    )
    return result
