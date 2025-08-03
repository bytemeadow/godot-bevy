#!/usr/bin/env python3

"""
Boids Performance Regression Test
Runs benchmarks and compares against baseline performance
"""

import json
import subprocess
import sys
import os
import argparse
from datetime import datetime
from typing import Dict, List, Tuple

class BenchmarkRunner:
    def __init__(self, godot_path: str = "godot"):
        self.godot_path = godot_path
        self.results_dir = "benchmark_results"
        os.makedirs(self.results_dir, exist_ok=True)

    def run_benchmark(self, implementation: str, entity_count: int, duration: float = 10.0) -> Dict:
        """Run a single benchmark and return results"""
        timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
        output_file = f"{self.results_dir}/benchmark_{implementation}_{entity_count}entities_{timestamp}.json"

        # Build command
        cmd = [
            self.godot_path,
            "--headless",
            f"--implementation={implementation}",
            f"--entity-count={entity_count}",
            f"--duration={duration}",
            f"--output={output_file}"
        ]

        print(f"🚀 Running {implementation} benchmark with {entity_count} entities...")
        print(f"   Command: {' '.join(cmd)}")

        try:
            # Change to godot directory
            original_dir = os.getcwd()
            os.chdir("godot")

            # Run benchmark with timeout and without capturing output
            # This prevents hanging on output buffer issues
            # Calculate timeout based on entity count - higher counts need more warmup time
            warmup_time = min(60, max(30, entity_count // 200))  # 30-60s warmup based on entity count
            total_timeout = duration + warmup_time + 30  # warmup + benchmark + shutdown
            
            result = subprocess.run(
                cmd,
                timeout=total_timeout,
                check=True
            )

            # Change back to original directory
            os.chdir(original_dir)

            # Wait a moment for file to be written
            import time
            time.sleep(0.5)

            # Check both possible locations for the output file
            possible_paths = [
                output_file,
                os.path.join("godot", output_file),
                output_file.replace("../", "")  # In case it was saved with ../
            ]

            for path in possible_paths:
                if os.path.exists(path):
                    print(f"✅ Found results at: {path}")
                    with open(path, 'r') as f:
                        return json.load(f)

            # If not found, list what files exist to debug
            print(f"❌ Results file not found. Searched paths:")
            for path in possible_paths:
                print(f"   - {path}")

            # List files in results directory to help debug
            if os.path.exists(self.results_dir):
                print(f"\nFiles in {self.results_dir}:")
                for f in sorted(os.listdir(self.results_dir))[-5:]:  # Show last 5 files
                    print(f"   - {f}")

            return None

        except subprocess.TimeoutExpired:
            print(f"❌ Benchmark timed out after {total_timeout} seconds")
            os.chdir(original_dir)
            return None
        except subprocess.CalledProcessError as e:
            print(f"❌ Benchmark failed with exit code: {e.returncode}")
            os.chdir(original_dir)
            return None
        except Exception as e:
            print(f"❌ Unexpected error: {e}")
            os.chdir(original_dir)
            return None

    def run_comparison(self, entity_counts: List[int], duration: float = 10.0) -> Dict:
        """Run benchmarks for both implementations across multiple entity counts"""
        results = {
            "godot": {},
            "bevy": {},
            "metadata": {
                "timestamp": datetime.now().isoformat(),
                "duration": duration,
                "entity_counts": entity_counts
            }
        }

        for count in entity_counts:
            print(f"\n📊 Testing with {count} entities...")

            # Run Godot benchmark
            godot_result = self.run_benchmark("godot", count, duration)
            if godot_result:
                results["godot"][count] = godot_result

            # Run Bevy benchmark
            bevy_result = self.run_benchmark("bevy", count, duration)
            if bevy_result:
                results["bevy"][count] = bevy_result

        return results

    def analyze_results(self, results: Dict) -> Dict:
        """Analyze benchmark results and calculate performance metrics"""
        analysis = {
            "performance_ratios": {},
            "summary": {}
        }

        for count in results["metadata"]["entity_counts"]:
            if count in results["godot"] and count in results["bevy"]:
                godot_fps = results["godot"][count]["avg_fps"]
                bevy_fps = results["bevy"][count]["avg_fps"]

                ratio = bevy_fps / godot_fps if godot_fps > 0 else 0

                analysis["performance_ratios"][count] = {
                    "godot_fps": godot_fps,
                    "bevy_fps": bevy_fps,
                    "speedup": ratio,
                    "percent_faster": (ratio - 1) * 100
                }

        # Calculate average speedup
        speedups = [v["speedup"] for v in analysis["performance_ratios"].values()]
        if speedups:
            analysis["summary"]["avg_speedup"] = sum(speedups) / len(speedups)
            analysis["summary"]["min_speedup"] = min(speedups)
            analysis["summary"]["max_speedup"] = max(speedups)

        return analysis

    def check_regression(self, current_results: Dict, baseline_file: str, threshold: float = 0.9) -> Tuple[bool, str]:
        """Check if current results show regression compared to baseline"""
        try:
            with open(baseline_file, 'r') as f:
                baseline = json.load(f)
        except FileNotFoundError:
            return True, "No baseline file found"

        regressions = []

        # Check each implementation and boid count
        for impl in ["godot", "bevy"]:
            for count, data in current_results[impl].items():
                if str(count) in baseline.get(impl, {}):
                    current_fps = data["avg_fps"]
                    baseline_fps = baseline[impl][str(count)]["avg_fps"]

                    if current_fps < baseline_fps * threshold:
                        percent_drop = ((baseline_fps - current_fps) / baseline_fps) * 100
                        regressions.append(
                            f"{impl} @ {count} entities: {current_fps:.1f} FPS "
                            f"(was {baseline_fps:.1f} FPS, -{percent_drop:.1f}%)"
                        )

        if regressions:
            return False, "\n".join(regressions)
        else:
            return True, "No regressions detected"

def main():
    parser = argparse.ArgumentParser(description="Boids Performance Regression Test")
    parser.add_argument("--godot", default="godot", help="Path to Godot executable")
    parser.add_argument("--entity-counts", nargs="+", type=int, default=[1000, 2000, 5000, 10000],
                      help="List of entity counts to test")
    parser.add_argument("--duration", type=float, default=10.0,
                      help="Duration of each benchmark in seconds")
    parser.add_argument("--baseline", help="Baseline results file for regression testing")
    parser.add_argument("--save-baseline", help="Save results as new baseline")
    parser.add_argument("--threshold", type=float, default=0.9,
                      help="Performance threshold for regression detection (default: 0.9)")

    args = parser.parse_args()

    # Build Rust library first
    print("🔨 Building Rust library...")
    subprocess.run(["cargo", "build", "--release", "--manifest-path", "rust/Cargo.toml"], check=True)

    # Run benchmarks
    runner = BenchmarkRunner(args.godot)
    results = runner.run_comparison(args.entity_counts, args.duration)

    # Analyze results
    analysis = runner.analyze_results(results)

    # Print results
    print("\n" + "="*50)
    print("📊 BENCHMARK RESULTS")
    print("="*50)

    for count, data in analysis["performance_ratios"].items():
        print(f"\n{count} boids:")
        print(f"  Godot: {data['godot_fps']:.1f} FPS")
        print(f"  Bevy:  {data['bevy_fps']:.1f} FPS")
        print(f"  Speedup: {data['speedup']:.2f}x ({data['percent_faster']:.1f}% faster)")

    if analysis["summary"]:
        print(f"\nAverage speedup: {analysis['summary']['avg_speedup']:.2f}x")
        print(f"Min speedup: {analysis['summary']['min_speedup']:.2f}x")
        print(f"Max speedup: {analysis['summary']['max_speedup']:.2f}x")

    # Save baseline if requested
    if args.save_baseline:
        with open(args.save_baseline, 'w') as f:
            json.dump(results, f, indent=2)
        print(f"\n✅ Baseline saved to: {args.save_baseline}")

    # Check for regression
    if args.baseline:
        passed, message = runner.check_regression(results, args.baseline, args.threshold)
        print(f"\n{'✅' if passed else '❌'} Regression Test: {message}")

        if not passed:
            sys.exit(1)

    # Save full results
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    os.makedirs(runner.results_dir, exist_ok=True)
    full_results_file = f"{runner.results_dir}/full_comparison_{timestamp}.json"
    with open(full_results_file, 'w') as f:
        json.dump({
            "results": results,
            "analysis": analysis
        }, f, indent=2)

    print(f"\n💾 Full results saved to: {full_results_file}")

if __name__ == "__main__":
    main()
