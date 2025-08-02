# Performance Benchmarking

This directory contains automated benchmarking tools for comparing the performance of Godot (GDScript) and Bevy (Rust) implementations across different performance tests.

## Quick Start

### Run a single benchmark:
```bash
# Test Godot implementation with 5000 entities
./benchmark.sh -i godot -c 5000

# Test Bevy implementation with 5000 entities
./benchmark.sh -i bevy -c 5000
```

### Run regression tests:
```bash
# Run full comparison across multiple entity counts
./regression_test.py --counts 1000 2000 5000 10000

# Save results as baseline
./regression_test.py --save-baseline baseline.json

# Check for regressions against baseline
./regression_test.py --baseline baseline.json
```

## Command Line Interface

The benchmarks can be run directly using Godot's command line:

```bash
godot --headless godot/project.godot \
    --implementation=godot \
    --entity-count=5000 \
    --duration=10 \
    --output=results.json
```

### Parameters:
- `--implementation`: Choose `godot` or `bevy`
- `--entity-count`: Number of entities to simulate
- `--duration`: Test duration in seconds
- `--output`: JSON file to save results

## Benchmark Scripts

### `benchmark.sh`
Simple shell script for running individual benchmarks.

Options:
- `--godot PATH`: Path to Godot executable
- `-i, --implementation`: Implementation to test (godot or bevy)
- `-c, --count`: Number of entities
- `-d, --duration`: Test duration in seconds
- `-o, --output`: Output directory

### `regression_test.py`
Python script for automated regression testing.

Options:
- `--godot`: Path to Godot executable
- `--counts`: List of entity counts to test
- `--duration`: Duration of each benchmark
- `--baseline`: Baseline file for regression testing
- `--save-baseline`: Save results as new baseline
- `--threshold`: Performance threshold (default: 0.9)

## Results Format

Benchmark results are saved as JSON with the following structure:

```json
{
    "implementation": "godot",
    "entity_count": 5000,
    "duration": 10.0,
    "frame_count": 600,
    "avg_fps": 60.0,
    "min_fps": 55.2,
    "max_fps": 62.1,
    "p95_fps": 58.5,
    "p99_fps": 56.1,
    "avg_frame_time_ms": 16.67,
    "min_frame_time_ms": 16.10,
    "max_frame_time_ms": 18.12,
    "timestamp": "2024-01-20 15:30:45"
}
```

## Continuous Integration

To use in CI/CD pipelines:

```yaml
# Example GitHub Actions workflow
- name: Run Performance Benchmark
  run: |
    cd examples/perf-test
    ./regression_test.py --baseline baseline.json --threshold 0.85
```

This will fail the build if performance drops more than 15% below baseline.

## Performance Tips

1. Run benchmarks on a consistent machine/environment
2. Close other applications to reduce variance
3. Use longer durations (30+ seconds) for more stable results
4. Run multiple iterations and average the results
5. Consider system warm-up by discarding first few seconds

## Troubleshooting

If benchmarks fail to run:
1. Ensure Godot is in your PATH or specify with `--godot`
2. Check that the Rust library is built: `cargo build --release`
3. Verify the project structure hasn't changed
4. Check console output for specific error messages
