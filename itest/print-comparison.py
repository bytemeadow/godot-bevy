#!/usr/bin/env python3
"""Pretty-print a benchmark comparison with a per-benchmark noise estimate.

Usage: print-comparison.py <results-dir>

Expects comparison.json plus the per-round baseline-run*.json /
current-run*.json files produced by compare-benches.sh. The Noise column is
the larger spread of per-round medians on either side; changes within that
spread are marked '~' and not counted as regressions or improvements.
"""

import glob
import json
import statistics
import sys

RED = "\033[31;1m"
GREEN = "\033[32;1m"
NC = "\033[0m"


def round_medians(pattern):
    runs = {}
    for path in sorted(glob.glob(pattern)):
        with open(path) as f:
            for name, data in json.load(f)["benchmarks"].items():
                runs.setdefault(name, []).append(float(data["median_ns"]))
    return runs


def spread_pct(values):
    if len(values) < 2:
        return None
    mid = statistics.median(values)
    if not mid:
        return None
    return (max(values) - min(values)) / mid * 100


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

        spreads = []
        for side in (base_runs, curr_runs):
            spread = spread_pct(side.get(b["name"], []))
            if spread is not None:
                spreads.append(spread)
        noise = max(spreads) if spreads else None
        noise_str = f"±{noise:.1f}%" if noise is not None else "-"

        # With a single round there is no spread to judge against; fall back
        # to treating every change as significant.
        significant = pct is not None and (noise is None or abs(pct) > noise)

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
        print(f"  {RED}{regressions} regression(s) exceed run-to-run noise{NC}")
    elif improvements:
        print(f"  {GREEN}{improvements} improvement(s) exceed run-to-run noise{NC}")
    else:
        print("  Performance is stable — no changes beyond run-to-run noise")
    print()

    name_w = max(len(name) for name, *_ in rows) + 2
    print(f"  {'Benchmark':<{name_w}} {'Current':>12} {'Baseline':>12} {'Change':>12} {'Noise':>8}")
    print(f"  {'─' * name_w} {'─' * 12} {'─' * 12} {'─' * 12} {'─' * 8}")
    for name, current, baseline, change, noise_str in rows:
        print(f"  {name:<{name_w}} {current:>12} {baseline:>12} {change:>12} {noise_str:>8}")
    print()
    print("  ~ = within noise (spread of per-round medians); Noise needs ≥2 rounds")


if __name__ == "__main__":
    main()
