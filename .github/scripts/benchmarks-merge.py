#!/usr/bin/env python3
import argparse
import json
import statistics
from datetime import datetime, timezone
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Merge multiple benchmark JSON files into one aggregated result"
    )
    parser.add_argument("output", help="Output benchmark JSON path")
    parser.add_argument(
        "inputs",
        nargs="+",
        help="Input benchmark JSON paths (at least one)",
    )
    return parser.parse_args()


def format_duration(ns: float) -> str:
    if ns >= 1_000_000_000:
        return f"{ns / 1_000_000_000:.2f}s"
    if ns >= 1_000_000:
        return f"{ns / 1_000_000:.2f}ms"
    if ns >= 1_000:
        return f"{ns / 1_000:.2f}us"
    return f"{ns:.0f}ns"


def load_json(path: Path) -> dict:
    with open(path) as f:
        return json.load(f)


def main() -> None:
    args = parse_args()
    output_path = Path(args.output)
    input_paths = [Path(path) for path in args.inputs]

    if not input_paths:
        raise SystemExit("No input benchmark files provided")

    runs = [load_json(path) for path in input_paths]

    name_sets = [set(run.get("benchmarks", {}).keys()) for run in runs]
    baseline_names = name_sets[0]
    for i, names in enumerate(name_sets[1:], start=2):
        if names != baseline_names:
            missing = sorted(baseline_names - names)
            extra = sorted(names - baseline_names)
            raise SystemExit(
                f"Benchmark name mismatch in input #{i}: missing={missing} extra={extra}"
            )

    merged_benchmarks = {}
    for name in sorted(baseline_names):
        median_values = []
        min_values = []
        for run in runs:
            data = run["benchmarks"][name]
            median_values.append(float(data["median_ns"]))
            min_values.append(float(data["min_ns"]))

        merged_median_ns = statistics.median(median_values)
        merged_min_ns = statistics.median(min_values)

        merged_benchmarks[name] = {
            "median_display": format_duration(merged_median_ns),
            "median_ns": str(int(round(merged_median_ns))),
            "min_display": format_duration(merged_min_ns),
            "min_ns": str(int(round(merged_min_ns))),
        }

    merged = {
        "benchmarks": merged_benchmarks,
        "environment": runs[0].get("environment", {}),
        "timestamp": datetime.now(timezone.utc).isoformat(),
    }

    with open(output_path, "w") as f:
        json.dump(merged, f, indent=2)


if __name__ == "__main__":
    main()
