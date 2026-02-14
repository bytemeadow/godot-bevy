#!/usr/bin/env python3
import argparse
import json
import re
import sys
from pathlib import Path


def parse_args() -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Assert benchmark JSON names match source benchmark functions"
    )
    parser.add_argument("source_file", help="Path to Rust benchmark source file")
    parser.add_argument("benchmark_json", help="Path to benchmark JSON output")
    return parser.parse_args()


def collect_source_bench_names(source_text: str) -> set[str]:
    pattern = re.compile(
        r"#\[\s*bench(?:\([^\]]*\))?\s*\]\s*\n\s*fn\s+([a-zA-Z0-9_]+)\s*\(",
        re.MULTILINE,
    )
    return {match.group(1) for match in pattern.finditer(source_text)}


def main() -> int:
    args = parse_args()
    source_path = Path(args.source_file)
    benchmark_json_path = Path(args.benchmark_json)

    source_text = source_path.read_text()
    source_names = collect_source_bench_names(source_text)

    with open(benchmark_json_path) as f:
        benchmark_json = json.load(f)

    json_names = set(benchmark_json.get("benchmarks", {}).keys())

    missing_in_json = sorted(source_names - json_names)
    unexpected_in_json = sorted(json_names - source_names)

    if missing_in_json or unexpected_in_json:
        print("Benchmark name mismatch detected:")
        if missing_in_json:
            print(f"  Missing in JSON ({len(missing_in_json)}): {missing_in_json}")
        if unexpected_in_json:
            print(f"  Unexpected in JSON ({len(unexpected_in_json)}): {unexpected_in_json}")
        return 1

    print(f"Benchmark name check passed ({len(source_names)} benchmarks).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
