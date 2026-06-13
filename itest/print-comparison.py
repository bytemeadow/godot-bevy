#!/usr/bin/env python3
"""Pretty-print a benchmark comparison with a per-benchmark noise estimate.

Usage: print-comparison.py <results-dir>

Expects comparison.json plus the per-round baseline-run*.json /
current-run*.json files produced by compare-benches.sh.

The Noise column is the standard error of the measured change, derived from the
spread of per-round medians on each side. Unlike a max-min spread, the standard
error *shrinks* as more rounds are added (it divides by sqrt(n)), so running
more rounds tightens the band instead of inflating it — the correct incentive
for a noisy benchmark. A change is only called a regression/improvement when it
exceeds 2x that standard error (~95% confidence); smaller changes are marked '~'
and not counted.
"""

import glob
import json
import math
import statistics
import sys

RED = "\033[31;1m"
GREEN = "\033[32;1m"
NC = "\033[0m"

# A change must exceed this many standard errors to count as real (~95% CI).
SIGNIFICANCE_SIGMA = 2.0


def round_medians(pattern):
    runs = {}
    for path in sorted(glob.glob(pattern)):
        with open(path) as f:
            for name, data in json.load(f)["benchmarks"].items():
                runs.setdefault(name, []).append(float(data["median_ns"]))
    return runs


def relative_stderr(values):
    """Standard error of the mean as a fraction of the median, or None if <2 samples.

    Converges as samples grow: more rounds shrink the band (stderr = stdev/sqrt(n))
    rather than widening it the way a max-min spread does.
    """
    if len(values) < 2:
        return None
    mid = statistics.median(values)
    if not mid:
        return None
    return (statistics.stdev(values) / math.sqrt(len(values))) / abs(mid)


def change_noise(base_runs, curr_runs, name):
    """1-sigma standard error of the change (percent), combining both sides."""
    base_se = relative_stderr(base_runs.get(name, []))
    curr_se = relative_stderr(curr_runs.get(name, []))
    present = [se for se in (base_se, curr_se) if se is not None]
    if not present:
        return None
    # Error of a ratio combines in quadrature; with one side missing, use the other.
    return math.sqrt(sum(se * se for se in present)) * 100


def main():
    results_dir = sys.argv[1]

    with open(f"{results_dir}/comparison.json") as f:
        comparison = json.load(f)

    benchmarks = comparison["benchmarks"]
    if not benchmarks:
        print("No benchmarks in comparison.")
        return

    base_runs = round_medians(f"{results_dir}/baseline-run*.json")
    curr_runs = round_medians(f"{results_dir}/current-run*.json")

    regressions = 0
    improvements = 0
    rows = []

    for b in benchmarks:
        pct = b.get("change_pct")

        noise = change_noise(base_runs, curr_runs, b["name"])
        noise_str = f"±{noise:.1f}%" if noise is not None else "-"

        # With a single round there is no spread to judge against; fall back
        # to treating every change as significant.
        significant = pct is not None and (
            noise is None or abs(pct) > SIGNIFICANCE_SIGMA * noise
        )

        if b["status"] == "new":
            change = "new"
        elif pct is None:
            change = ""
        elif not significant:
            change = f"~ {'+' if pct > 0 else ''}{pct:.1f}%"
        else:
            icon = {"regression": "🔴", "slower": "🟡", "faster": "🟢"}.get(
                b["status"], ""
            )
            sign = "+" if pct > 0 else ""
            change = f"{icon} {sign}{pct:.1f}%".strip()
            if b["status"] == "regression":
                regressions += 1
            elif b["status"] == "faster":
                improvements += 1

        rows.append((b["name"], b.get("current", "-"), b.get("baseline") or "-", change, noise_str))

    print()
    if regressions:
        print(f"  {RED}{regressions} regression(s) significant beyond noise{NC}")
    elif improvements:
        print(f"  {GREEN}{improvements} improvement(s) significant beyond noise{NC}")
    else:
        print("  Performance is stable — no statistically significant changes")
    print()

    name_w = max(len(name) for name, *_ in rows) + 2
    print(f"  {'Benchmark':<{name_w}} {'Current':>12} {'Baseline':>12} {'Change':>12} {'Noise':>8}")
    print(f"  {'─' * name_w} {'─' * 12} {'─' * 12} {'─' * 12} {'─' * 8}")
    for name, current, baseline, change, noise_str in rows:
        print(f"  {name:<{name_w}} {current:>12} {baseline:>12} {change:>12} {noise_str:>8}")
    print()
    print(
        "  ~ = within 2x noise (not significant at ~95%). Noise = standard error of"
    )
    print(
        "  the change; it shrinks with more rounds (BENCH_ROUNDS). Needs ≥2 rounds."
    )


if __name__ == "__main__":
    main()
