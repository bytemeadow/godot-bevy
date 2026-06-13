from __future__ import annotations

import argparse
import json
import math
import os
import statistics
import sys
from pathlib import Path

# A change must exceed this many standard errors of the per-round medians to
# count as a real regression/improvement (~95% confidence), in addition to the
# magnitude thresholds below. Without per-round data this gate is disabled.
SIGNIFICANCE_SIGMA = 2.0


def create_argument_parser() -> argparse.ArgumentParser:
    """Create and configure the argument parser for benchmark comparison."""
    parser = argparse.ArgumentParser(
        description="Compare benchmark results against a baseline"
    )
    parser.add_argument("baseline", help="Path to the baseline.json file")
    parser.add_argument("bench_results", help="Path to the bench-results.json file")
    parser.add_argument(
        "output_file", help="Path to the output json file for the comparison results"
    )
    parser.add_argument(
        "--baseline-runs",
        nargs="+",
        default=[],
        help="Per-round baseline JSON files (enables noise-aware gating)",
    )
    parser.add_argument(
        "--current-runs",
        nargs="+",
        default=[],
        help="Per-round current JSON files (enables noise-aware gating)",
    )
    return parser


def load_round_medians(paths: list[str]) -> dict[str, list[float]]:
    """Collect per-round median_ns for each benchmark across the given files."""
    runs: dict[str, list[float]] = {}
    for path in paths:
        try:
            with open(path) as f:
                data = json.load(f)
        except (FileNotFoundError, json.JSONDecodeError):
            continue
        for name, d in data.get("benchmarks", {}).items():
            try:
                runs.setdefault(name, []).append(float(d["median_ns"]))
            except (KeyError, ValueError, TypeError):
                pass
    return runs


def relative_stderr(values: list[float]) -> float | None:
    """Standard error of the mean as a fraction of the median (or None if <2)."""
    if len(values) < 2:
        return None
    mid = statistics.median(values)
    if not mid:
        return None
    return (statistics.stdev(values) / math.sqrt(len(values))) / abs(mid)


def change_noise(base_runs, curr_runs, name) -> float | None:
    """1-sigma standard error of the change (percent), combining both sides."""
    present = [
        se
        for se in (
            relative_stderr(base_runs.get(name, [])),
            relative_stderr(curr_runs.get(name, [])),
        )
        if se is not None
    ]
    if not present:
        return None
    return math.sqrt(sum(se * se for se in present)) * 100


def main(args: list[str]) -> None:
    parser = create_argument_parser()
    parsed_args = parser.parse_args(args[1:])  # Skip script name
    bench_results_path = Path(parsed_args.bench_results)
    baseline_path = Path(parsed_args.baseline)
    output_file_path = Path(parsed_args.output_file)

    base_runs = load_round_medians(parsed_args.baseline_runs)
    curr_runs = load_round_medians(parsed_args.current_runs)

    try:
        with open(bench_results_path) as f:
            current_results = json.load(f)
    except FileNotFoundError:
        print(
            f"Error: benchmark results file {bench_results_path} not found, can't continue."
        )
        exit(1)

    baseline = {"benchmarks": {}}
    try:
        with open(baseline_path) as f:
            baseline = json.load(f)
    except FileNotFoundError:
        print(
            f"Warning: baseline file {baseline_path} not found, showing only current results."
        )
        print(f"Directory contents of {baseline_path.parent}:")
        for file in os.listdir(baseline_path.parent):
            print(f"  {file}")
    except json.decoder.JSONDecodeError as json_error:
        print(
            f"Warning: baseline file json parse error {baseline_path}, showing only current results."
        )
        print(json_error)

    baseline_benchmark_names = list(baseline.get("benchmarks", {}).keys())
    comparison = {
        "benchmarks": [],
        "summary": {
            "total": 0,
            "regressions": 0,
            "improvements": 0,
            "new": 0,
            "baseline_count": len(baseline_benchmark_names),
        },
    }

    for name, curr_data in current_results.get("benchmarks", {}).items():
        base_data = baseline.get("benchmarks", {}).get(name)

        entry = {
            "name": name,
            "current": curr_data.get("median_display", "N/A"),
            "baseline": None,
            "change_pct": None,
            "noise_pct": None,
            "status": "new",
        }

        if base_data:
            entry["baseline"] = base_data.get("median_display", "N/A")

            try:
                current_ns = float(curr_data.get("median_ns", 0))
                baseline_ns = float(base_data.get("median_ns", 0))

                if baseline_ns > 0:
                    change_pct = ((current_ns - baseline_ns) / baseline_ns) * 100
                    entry["change_pct"] = change_pct

                    # Noise-aware gating: a change only counts as a regression or
                    # improvement when it also exceeds the per-round measurement
                    # noise. Without per-round data, noise is None and every change
                    # is treated as significant (legacy behavior).
                    noise = change_noise(base_runs, curr_runs, name)
                    entry["noise_pct"] = noise
                    significant = noise is None or abs(change_pct) > (
                        SIGNIFICANCE_SIGMA * noise
                    )

                    if change_pct > 10 and significant:
                        entry["status"] = "regression"
                        comparison["summary"]["regressions"] += 1
                    elif change_pct > 5 and significant:
                        entry["status"] = "slower"
                    elif change_pct < -5 and significant:
                        entry["status"] = "faster"
                        comparison["summary"]["improvements"] += 1
                    else:
                        entry["status"] = "neutral"
            except Exception:
                pass
        else:
            comparison["summary"]["new"] += 1

        comparison["benchmarks"].append(entry)

    comparison["summary"]["total"] = len(comparison["benchmarks"])

    with open(output_file_path, "w") as f:
        json.dump(comparison, f, indent=2)

    # Log summary - regressions are reported in PR comment, don't fail the build
    regressions = comparison["summary"]["regressions"]
    improvements = comparison["summary"]["improvements"]
    new_count = comparison["summary"]["new"]
    baseline_names = baseline_benchmark_names
    if regressions > 0:
        print(f"⚠️  {regressions} regression(s) detected - see PR comment for details")
    if improvements > 0:
        print(f"✅ {improvements} improvement(s) detected")
    if new_count == comparison["summary"]["total"] and comparison["summary"]["total"] > 0:
        if not baseline_names:
            print("⚠️  Baseline (main) produced no benchmark entries.")
        else:
            print(
                f"⚠️  No PR benchmark names match main. Main has {len(baseline_names)}: {baseline_names[:5]}{'...' if len(baseline_names) > 5 else ''}"
            )


if __name__ == "__main__":
    main(sys.argv)
