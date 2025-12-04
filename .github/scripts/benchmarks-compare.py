import argparse
import json
import os
import sys
from pathlib import Path


def create_argument_parser() -> argparse.ArgumentParser:
    """Create and configure the argument parser for benchmark comparison."""
    parser = argparse.ArgumentParser(description='Compare benchmark results against a baseline')
    parser.add_argument('baseline', help='Path to the baseline.json file')
    parser.add_argument('bench_results', help='Path to the bench-results.json file')
    parser.add_argument('output_file', help='Path to the output json file for the comparison results')
    return parser


def main(args: list[str]) -> None:
    parser = create_argument_parser()
    parsed_args = parser.parse_args(args[1:])  # Skip script name
    bench_results_path = Path(parsed_args.bench_results)
    baseline_path = Path(parsed_args.baseline)
    output_file_path = Path(parsed_args.output_file)

    try:
        with open(bench_results_path) as f:
            current_results = json.load(f)
    except FileNotFoundError:
        print(f"Error: benchmark results file {bench_results_path} not found, can't continue.")
        exit(1)

    baseline = {"benchmarks": {}}
    try:
        with open(baseline_path) as f:
            baseline = json.load(f)
    except FileNotFoundError:
        print(f"Warning: baseline file {baseline_path} not found, showing only current results.")
        print(f"Directory contents of {baseline_path.parent}:")
        for file in os.listdir(baseline_path.parent):
            print(f"  {file}")
    except json.decoder.JSONDecodeError as json_error:
        print(f"Warning: baseline file json parse error {baseline_path}, showing only current results.")
        print(json_error)

    comparison = {
        "benchmarks": [],
        "summary": {"total": 0, "regressions": 0, "improvements": 0, "new": 0}
    }

    for name, curr_data in current_results.get("benchmarks", {}).items():
        base_data = baseline.get("benchmarks", {}).get(name)

        entry = {
            "name": name,
            "current": curr_data.get("median_display", "N/A"),
            "baseline": None,
            "change_pct": None,
            "status": "new"
        }

        if base_data:
            entry["baseline"] = base_data.get("median_display", "N/A")

            try:
                current_ns = float(curr_data.get("median_ns", 0))
                baseline_ns = float(base_data.get("median_ns", 0))

                if baseline_ns > 0:
                    change_pct = ((current_ns - baseline_ns) / baseline_ns) * 100
                    entry["change_pct"] = change_pct

                    if change_pct > 10:
                        entry["status"] = "regression"
                        comparison["summary"]["regressions"] += 1
                    elif change_pct > 5:
                        entry["status"] = "slower"
                    elif change_pct < -5:
                        entry["status"] = "faster"
                        comparison["summary"]["improvements"] += 1
                    else:
                        entry["status"] = "neutral"
            except:
                pass
        else:
            comparison["summary"]["new"] += 1

        comparison["benchmarks"].append(entry)

    comparison["summary"]["total"] = len(comparison["benchmarks"])

    with open(output_file_path, 'w') as f:
        json.dump(comparison, f, indent=2)

    # Exit with error if regressions detected
    sys.exit(1 if comparison["summary"]["regressions"] > 0 else 0)


if __name__ == "__main__":
    main(sys.argv)
