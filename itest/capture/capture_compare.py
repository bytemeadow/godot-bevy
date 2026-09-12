from __future__ import annotations

from capture_schema import PROTOCOL, finite_number


def compare_checkpoint(manifest: dict, checkpoint: dict, facts: object, *, scene: str | None = None) -> list[dict]:
    diffs = []

    def mismatch(path, expected, actual):
        diffs.append({"path": path, "expected": expected, "actual": actual})

    def equal(path, expected, actual):
        if (type(actual) is not type(expected) or actual != expected
                or (isinstance(expected, list) and any(type(a) is not type(e) for a, e in zip(actual, expected)))):
            mismatch(path, expected, actual)

    def vector(path, expected, actual, tolerance, rgb=False):
        if (not isinstance(actual, list) or len(actual) != len(expected)
                or not all(finite_number(v) for v in actual)
                or (rgb and not all(type(v) is int and 0 <= v <= 255 for v in actual))
                or any(abs(a - e) > tolerance for a, e in zip(actual, expected))):
            mismatch(path, {"value": expected, "tolerance": tolerance}, actual)

    if not isinstance(facts, dict):
        mismatch("facts", "object", facts)
        return diffs
    frame = checkpoint["frame"]
    for key, expected in {
        "version": PROTOCOL["version"], "scenario": manifest["scenario"], "scene": scene or manifest["scene"],
        "frame": frame, "viewport": manifest["viewport"],
        "fixed_step_ns": PROTOCOL["fixed_step_ns"],
        "virtual_elapsed_ns": frame * manifest["clocks"]["bevy_step_ns"],
    }.items():
        equal(key, expected, facts.get(key))

    ticks = facts.get("physics_ticks")
    if type(ticks) is not int or ticks < 0 or (frame == 0 and ticks != 0):
        mismatch("physics_ticks", "nonnegative integer; zero at frame 0", ticks)
    else:
        if manifest["pacing"] == "fixed":
            equal("physics_ticks", frame, ticks)
        equal("fixed_elapsed_ns", ticks * PROTOCOL["fixed_step_ns"], facts.get("fixed_elapsed_ns"))

    for key, identity in (("nodes", "path"), ("regions", "name")):
        actual = facts.get(key)
        if not isinstance(actual, list) or not all(isinstance(v, dict) for v in actual):
            mismatch(key, "array of facts", actual)
            continue
        names = [v.get(identity) for v in actual]
        expected_names = [v[identity] for v in checkpoint[key]]
        if (not all(isinstance(name, str) for name in names)
                or len(names) != len(set(names)) or set(names) != set(expected_names)):
            mismatch(key, expected_names, names)
            continue
        indexed = {v[identity]: v for v in actual}
        for expected in checkpoint[key]:
            name = expected[identity]
            found = indexed[name]
            path = f"{key}.{name}"
            if key == "nodes":
                vector(path + ".position", expected["position"], found.get("position"), expected["tolerance"])
                equal(path + ".visible", expected["visible"], found.get("visible"))
            else:
                equal(path + ".rect", expected["rect"], found.get("rect"))
                fraction = found.get("non_blank_fraction")
                minimum = expected["non_blank"]["min_fraction"]
                if not finite_number(fraction) or not minimum <= fraction <= 1:
                    mismatch(path + ".non_blank_fraction", {"minimum": minimum}, fraction)
                colour = expected["dominant_colour"]
                vector(path + ".dominant_colour", colour["rgb"], found.get("dominant_colour"), colour["tolerance"], rgb=True)
    return diffs
