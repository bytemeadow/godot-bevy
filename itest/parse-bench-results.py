#!/usr/bin/env python3
"""
Parse godot-bevy benchmark output and save as JSON for CI comparison.
"""

import json
import re
import sys
from datetime import datetime
from pathlib import Path


def parse_benchmark_output(output: str) -> dict:
    """Parse benchmark output and extract timing data."""
    results = {
        "timestamp": datetime.utcnow().isoformat(),
        "benchmarks": {}
    }

    # Pattern: benchmark_name    min    median
    # Example: transform_update_individual_3d    6.32ms    6.70ms
    pattern = r'(\w+)\s+([\d.]+)(µs|ms|ns)\s+([\d.]+)(µs|ms|ns)'

    for line in output.split('\n'):
        match = re.search(pattern, line)
        if match:
            name = match.group(1)
            min_value = float(match.group(2))
            min_unit = match.group(3)
            median_value = float(match.group(4))
            median_unit = match.group(5)

            # Convert to nanoseconds for consistent comparison
            min_ns = convert_to_ns(min_value, min_unit)
            median_ns = convert_to_ns(median_value, median_unit)

            results["benchmarks"][name] = {
                "min_ns": min_ns,
                "median_ns": median_ns,
                "min_display": f"{min_value}{min_unit}",
                "median_display": f"{median_value}{median_unit}"
            }

    return results


def convert_to_ns(value: float, unit: str) -> float:
    """Convert time value to nanoseconds."""
    conversions = {
        'ns': 1,
        'µs': 1000,
        'ms': 1_000_000,
        's': 1_000_000_000
    }
    return value * conversions.get(unit, 1)


def compare_benchmarks(current: dict, baseline: dict, threshold: float = 0.90) -> tuple[bool, list, list]:
    """
    Compare current benchmarks against baseline.

    Args:
        current: Current benchmark results
        baseline: Baseline benchmark results to compare against
        threshold: Performance ratio threshold (default 0.90 = allow 10% slowdown)
                   Values < 1.0 mean slower is acceptable up to that ratio.
                   E.g., 0.90 means current can be 1/0.90 = 1.11x slower (11% regression)

    Returns:
        (passed, regressions, all_comparisons) where:
        - passed: True if no regressions exceed threshold
        - regressions: list of benchmarks that regressed beyond threshold
        - all_comparisons: list of all benchmark comparisons for detailed reporting
    """
    regressions = []
    all_comparisons = []

    for name, current_data in current["benchmarks"].items():
        if name not in baseline["benchmarks"]:
            all_comparisons.append({
                "name": name,
                "status": "new",
                "current": current_data,
                "baseline": None,
                "change_pct": None
            })
            continue

        baseline_data = baseline["benchmarks"][name]
        current_median = current_data["median_ns"]
        baseline_median = baseline_data["median_ns"]

        # Calculate change percentage (negative = faster, positive = slower)
        change_pct = ((current_median - baseline_median) / baseline_median) * 100
        ratio = current_median / baseline_median

        comparison = {
            "name": name,
            "current": current_data,
            "baseline": baseline_data,
            "change_pct": change_pct
        }

        # Determine status
        if ratio > (1 / threshold):  # Regression beyond threshold
            comparison["status"] = "regression"
            regressions.append((name, change_pct, current_data, baseline_data))
        elif change_pct > 0:
            comparison["status"] = "slower"
        elif change_pct < -5:  # Improvement > 5%
            comparison["status"] = "faster"
        else:
            comparison["status"] = "neutral"

        all_comparisons.append(comparison)

    passed = len(regressions) == 0
    return passed, regressions, all_comparisons


def format_regression_report(regressions: list, all_comparisons: list) -> str:
    """Format comprehensive comparison report for CI output."""
    if not regressions:
        return "✅ No performance regressions detected!"

    report = ["⚠️ Performance Regressions Detected:", ""]

    for name, regression_pct, current, baseline in regressions:
        report.append(f"**{name}**:")
        report.append(f"  - Baseline: {baseline['median_display']}")
        report.append(f"  - Current:  {current['median_display']}")
        report.append(f"  - Change: +{regression_pct:.1f}% slower")
        report.append("")

    return "\n".join(report)


def format_comparison_json(all_comparisons: list) -> str:
    """Format all comparisons as JSON for the comment workflow."""
    return json.dumps({"comparisons": all_comparisons}, indent=2)


def main():
    import argparse

    parser = argparse.ArgumentParser(description='Parse benchmark results')
    parser.add_argument('--output', help='Output JSON file')
    parser.add_argument('--baseline', help='Baseline JSON file for comparison')
    parser.add_argument('--threshold', type=float, default=0.90,
                        help='Regression threshold (0.90 = allow 10%% slowdown)')
    parser.add_argument('--json-input', action='store_true',
                        help='Input is already JSON (for comparison-only mode)')
    parser.add_argument('input_file', nargs='?', help='Input file (or stdin)')

    args = parser.parse_args()

    # Read input
    if args.input_file:
        with open(args.input_file) as f:
            output = f.read()
    else:
        output = sys.stdin.read()

    # Parse benchmarks
    if args.json_input:
        results = json.loads(output)
    else:
        results = parse_benchmark_output(output)

    if not results["benchmarks"]:
        print("❌ No benchmarks found in output", file=sys.stderr)
        sys.exit(1)

    # Save results
    if args.output:
        with open(args.output, 'w') as f:
            json.dump(results, f, indent=2)
        print(f"✅ Saved {len(results['benchmarks'])} benchmarks to {args.output}")

    # Compare against baseline if provided
    if args.baseline:
        with open(args.baseline) as f:
            baseline = json.load(f)

        passed, regressions, all_comparisons = compare_benchmarks(results, baseline, args.threshold)

        # Save detailed comparison data for PR comments
        comparison_file = args.output.replace('.json', '-comparison.json') if args.output else 'comparison.json'
        with open(comparison_file, 'w') as f:
            json.dump({"comparisons": all_comparisons, "passed": passed}, f, indent=2)

        print("\n" + format_regression_report(regressions, all_comparisons))

        if not passed:
            sys.exit(1)
    else:
        print(f"✅ Parsed {len(results['benchmarks'])} benchmarks")

    return 0


if __name__ == '__main__':
    sys.exit(main())
