# godot-bevy Benchmarking

This document describes the benchmarking infrastructure for godot-bevy, which uses fast, deterministic micro-benchmarks to detect performance regressions.

## Quick Start

### Run Benchmarks Locally

```bash
cd itest
./run-benches.sh
```

This will:
1. Build the Rust library in `--release` mode
2. Run benchmarks in Godot headless mode
3. Display results with min/median times

### Compare Against a Base Branch

```bash
cd itest
./compare-benches.sh          # compare current branch vs main
./compare-benches.sh develop  # compare current branch vs develop
```

This mirrors what CI does on every PR:
1. Creates a git worktree for the base branch
2. Builds and runs benchmarks on the base branch
3. Builds and runs benchmarks on the current branch
4. Prints a comparison table with change percentages

Both builds share `target/` so dependency compilation only happens once.
Results are saved to `itest/.bench-results/` for further inspection.

### Example Output

```
📊 Benchmark Results:
                                                                    min       median
itest/rust/src/benchmarks.rs:14
  transform_update_individual_3d                                 6.32ms       6.70ms
  transform_update_bulk_3d                                       4.42ms       4.88ms
  transform_update_individual_2d                                 6.90ms       7.39ms
  transform_update_bulk_2d                                       5.19ms       5.57ms

Benchmarks completed in 52.54s.
```

## What We Benchmark

### Transform Synchronization (5000 entities)

These benchmarks measure the performance of our **PackedArray optimization** for bulk transform updates:

| Benchmark | What It Tests | Entity Count |
|-----------|---------------|--------------|
| `transform_update_individual_3d` | Individual FFI calls (5000 calls) | 5000 Node3D |
| `transform_update_bulk_3d` | Bulk PackedArray update (1 call) | 5000 Node3D |
| `transform_update_individual_2d` | Individual FFI calls (5000 calls) | 5000 Node2D |
| `transform_update_bulk_2d` | Bulk PackedArray update (1 call) | 5000 Node2D |

**Key Insight**: Bulk operations are ~25-30% faster, saving ~2ms per frame for 5000 entities.

## CI Integration

### Automated Regression Detection

Every PR automatically runs benchmarks and compares against the `main` branch baseline:

1. **PR opened/updated** → Benchmarks run automatically
2. **Compare** against baseline from `main`
3. **Comment on PR** with results table
4. **Fail if regression** > 10% slowdown detected

### Baseline Updates

When code is merged to `main`:
1. Benchmarks run on CI infrastructure
2. Results saved to `itest/baseline.json`
3. Baseline automatically committed with metadata
4. Future PRs compare against this baseline

## Adding New Benchmarks

### 1. Write the Benchmark

Add to `itest/rust/src/benchmarks.rs`:

```rust
use godot_bevy_itest_macros::bench;

#[bench(repeat = 3)]  // Optional: custom repetitions
fn my_new_benchmark() -> i32 {
    // Benchmark code - must return a value
    // Will be run 3 times internally, plus 200 warmup + 501 test runs

    let result = do_expensive_operation();
    result
}
```

**Requirements:**
- Must return a value (prevents compiler optimization)
- Should be deterministic (avoid randomness)
- Keep runtime reasonable (< 1 second per iteration)

### 2. Run Locally

```bash
cd itest
./run-benches.sh
```

### 3. Create PR

The CI will automatically:
- Run your new benchmark
- Compare against baseline (if it exists in `main`)
- Comment with results

## Benchmark Infrastructure

### Architecture

```
itest/
├── run-benches.sh              # Main entry point
├── parse-bench-results.py      # Parse output → JSON
├── baseline.json               # CI-generated baseline
└── rust/
    ├── src/
    │   ├── benchmarks.rs       # Benchmark definitions
    │   └── framework/
    │       └── bencher.rs      # Benchmark runner
    └── macros/
        └── src/lib.rs          # #[bench] macro
```

### How It Works

1. **`#[bench]` macro** expands to:
   - Wrapper function that runs code N times
   - Registration in plugin system
   - Uses `std::hint::black_box()` to prevent optimization

2. **Benchmark runner** (`bencher.rs`):
   - 200 warmup runs
   - 501 test runs (odd number for clean median)
   - Reports min and median (ignores outliers)

3. **CI workflows**:
   - **`benchmarks.yml`**: Runs benchmarks, checks regressions, saves artifacts
   - **`benchmark-comment.yml`**: Posts results as PR comments (runs with elevated permissions via `workflow_run`)
   - Two-workflow pattern needed to support PRs from forks (security requirement)

## Why itest Benchmarks vs perf-test?

### itest Benchmarks (THIS - Used in CI)

✅ **Fast**: ~1 minute total
✅ **Deterministic**: Measures exact µs/ms
✅ **Focused**: Tests specific optimizations
✅ **Headless**: No rendering overhead
✅ **CI-friendly**: Runs on every PR

### perf-test Example (Demo/Manual Testing)

✅ **End-to-end**: Full game loop with rendering
✅ **Visual**: Users can see and interact
✅ **Comparison**: GDScript vs Bevy implementations
❌ **Slow**: ~10+ minutes to run
❌ **Flaky**: FPS varies with hardware

**Recommendation**: Use itest benchmarks for CI regression detection, keep perf-test as user-facing demo.

## Baseline Management

### Format

```json
{
  "timestamp": "2025-10-10T10:00:00.000000",
  "benchmarks": {
    "transform_update_individual_3d": {
      "min_ns": 6250000.0,
      "median_ns": 6410000.0,
      "min_display": "6.25ms",
      "median_display": "6.41ms"
    }
  },
  "ci_metadata": {
    "commit": "abc123...",
    "timestamp": "2025-10-10T10:00:00Z",
    "runner": "GitHub Actions"
  }
}
```

### Manual Baseline Update

```bash
cd itest
./run-benches.sh 2>&1 | python3 parse-bench-results.py --output baseline.json
git add baseline.json
git commit -m "chore: update benchmark baseline"
```

## Regression Detection

### Threshold

Default: **90%** (10% slowdown allowed)

Configurable via `--threshold` parameter:
```bash
python3 parse-bench-results.py --baseline baseline.json --threshold 0.95  # 5% tolerance
```

### What Triggers Failure

A benchmark fails regression check if:
```
current_median / baseline_median > 1.11  # More than 11% slower (with 90% threshold)
```

### Example PR Comment

```markdown
## 📊 Benchmark Results

| Benchmark | Median | Min |
|-----------|--------|-----|
| transform_update_individual_3d | 6.70ms | 6.32ms |
| transform_update_bulk_3d | 4.88ms | 4.42ms |

### Regression Check

⚠️ Performance Regressions Detected:

**transform_update_bulk_3d**:
  - Baseline: 2.10ms
  - Current:  4.88ms
  - Regression: +132.4% slower

> ⚠️ **Warning**: Performance regressions detected. Please investigate before merging.
```

## Troubleshooting

### "No benchmarks found"

Check that:
- Benchmarks are in `itest/rust/src/benchmarks.rs`
- Using `#[bench]` macro from `godot_bevy_itest_macros`
- Function returns a value
- `mod benchmarks;` is in `lib.rs`

### "Benchmarks taking too long"

Reduce entity count in `BENCH_ENTITY_COUNT` constant or reduce `repeat` parameter:
```rust
const BENCH_ENTITY_COUNT: usize = 1000;  // Down from 5000

#[bench(repeat = 1)]  // Down from 3
fn my_benchmark() -> i32 { ... }
```

### "Godot crashes with RID limit"

Use `node.free()` instead of `node.queue_free()` for immediate cleanup:
```rust
for node in nodes {
    node.free();  // Immediate, not deferred
}
```

## Future Improvements

- [ ] Add benchmarks for scene tree event processing
- [ ] Benchmark asset loading performance
- [ ] Add audio system benchmarks
- [ ] Compare different entity counts (scaling)
- [ ] Add memory usage tracking
- [ ] Benchmark parallel ECS systems

## References

- [CLAUDE.md](../CLAUDE.md) - PackedArray optimization pattern
- [gdext benchmarking](https://github.com/godot-rust/gdext/tree/master/itest) - Inspiration for this system
- [Criterion.rs](https://github.com/bheisler/criterion.rs) - Statistical benchmarking (not used, but relevant)
