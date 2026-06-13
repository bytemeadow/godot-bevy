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

### Run a Subset of Benchmarks

```bash
./run-benches.sh --filter transform_sync        # only transform sync benches
./run-benches.sh --filter "signal,collision"    # comma-separated, substring match
```

The filter is a comma-separated list of substrings; a benchmark runs if its
name contains any of them. It also works as an environment variable
(`BENCHMARK_FILTER=transform_sync`), including with `compare-benches.sh`.

### Compare Against a Base Branch

```bash
cd itest
./compare-benches.sh          # compare current branch vs main
./compare-benches.sh develop  # compare current branch vs develop
```

This mirrors what CI does on every PR:
1. Creates a git worktree for the base branch and builds both branches
2. Runs benchmarks interleaved (base, current, base, current, ...) so thermal
   drift and background load hit both sides equally
3. Merges each side's runs (median of medians)
4. Prints a comparison table with change percentages

Both builds share `target/` so dependency compilation only happens once.
Results are saved to `itest/.bench-results/` for further inspection.

`BENCH_ROUNDS` controls how many interleaved rounds run per side (default: 3).

The comparison table includes a `Noise` column: the **standard error of the
measured change**, derived from the spread of per-round medians on each side. A
change must exceed **2× that standard error** (~95% confidence) to count as a
regression or improvement; smaller changes are marked `~`. Because the standard
error divides by √(rounds), running more rounds *tightens* the band — so for a
noisy benchmark, bump `BENCH_ROUNDS` (e.g. `BENCH_ROUNDS=6`) to resolve a
borderline result, rather than re-rolling the dice. (This replaced an earlier
max−min spread metric, which wrongly *widened* with more rounds and could mask a
real change behind one outlier round.) µs-scale benchmarks routinely shift >10%
between processes, so never judge a change from two standalone `run-benches.sh`
runs — always use `compare-benches.sh`.

### Example Output

```
Benchmark Results:
                                                                    min       median
itest/rust/src/benchmarks.rs:146
  transform_sync_bevy_to_godot_3d                             118.58µs     120.64µs
  transform_sync_bevy_to_godot_3d_100                          16.82µs      18.07µs
  transform_sync_bevy_to_godot_3d_5000                        566.57µs     579.96µs
  transform_sync_godot_to_bevy_3d                             172.71µs     175.80µs
  transform_sync_godot_to_bevy_3d_5000                        829.90µs     847.15µs
  transform_sync_bevy_to_godot_2d                             119.32µs     122.35µs
  transform_sync_godot_to_bevy_2d                             131.79µs     134.21µs
  transform_sync_roundtrip_3d                                 225.57µs     239.72µs
  transform_sync_roundtrip_2d                                 184.85µs     200.83µs
  scene_tree_idle_no_messages                                   2.59ms       2.65ms
  scene_tree_process_node_added_optimized                     678.80µs     688.19µs
  scene_tree_process_node_added_optimized_2500                  2.70ms       3.11ms
  scene_tree_process_node_added_fallback                      917.07µs     924.35µs
  scene_tree_process_node_renamed_sparse_updates                1.66ms       2.03ms
  scene_tree_process_collision_bodies_optimized               426.56µs     474.18µs
  scene_tree_process_collision_bodies_fallback                490.19µs     498.86µs
  collisions_process_start_end_burst                            5.60ms       5.74ms
  collisions_process_start_end_burst_1000                       5.22ms       5.40ms
  input_action_checking_many_events_many_actions                1.13ms       1.14ms
  packed_scene_batch_spawn                                    157.26µs     158.67µs
  signal_dispatch_throughput                                   43.83µs      46.69µs
  signal_dispatch_throughput_1000                             152.29µs     154.74µs
  signal_connection_setup                                      64.47µs      66.33µs

Benchmarks completed in 3.92s.
```

Reported times cover only each benchmark's `measured(|| ...)` scope — setup
and teardown are excluded.

## What We Benchmark

The suite contains **24 benchmarks** across six categories. Every benchmark runs the real godot-bevy systems (plugins, schedules, ECS queries) rather than raw FFI calls, so regressions in actual user-facing code are caught.

### Scaling Variants

Size-suffixed benchmarks (`_100`, `_1000`, `_2500`, `_5000`) rerun a system at
a different entity count than its default-named sibling. Comparing per-size
times reveals scaling behavior a single size can't: 5x the entities should
cost roughly 5x the time — a much larger ratio means super-linear (e.g. O(n²))
growth, and an optimization that only helps at one size shows up as a skewed
ratio.

### Transform Synchronization (9 benchmarks, 1000 entities unless suffixed)

These benchmarks measure the real `GodotTransformSyncPlugin` systems that sync transforms between Bevy ECS and Godot nodes.

| Benchmark | What It Tests |
|-----------|---------------|
| `transform_sync_bevy_to_godot_3d` | Bevy->Godot 3D sync (Last schedule) |
| `transform_sync_bevy_to_godot_3d_100` | Scaling variant (100 nodes) |
| `transform_sync_bevy_to_godot_3d_5000` | Scaling variant (5000 nodes) |
| `transform_sync_godot_to_bevy_3d` | Godot->Bevy 3D sync (PreUpdate schedule) |
| `transform_sync_godot_to_bevy_3d_5000` | Scaling variant (5000 nodes) |
| `transform_sync_bevy_to_godot_2d` | Bevy->Godot 2D sync (Last schedule) |
| `transform_sync_godot_to_bevy_2d` | Godot->Bevy 2D sync (PreUpdate schedule) |
| `transform_sync_roundtrip_3d` | Full frame: PreUpdate -> game logic -> Last (3D) |
| `transform_sync_roundtrip_2d` | Full frame: PreUpdate -> game logic -> Last (2D) |

### Scene Tree Processing (7 benchmarks, 500 nodes unless suffixed)

These benchmarks measure the `GodotSceneTreePlugin` systems that process node-added, renamed, and collision-body messages from Godot.

| Benchmark | What It Tests |
|-----------|---------------|
| `scene_tree_idle_no_messages` | Per-frame overhead when stable (200 frames) |
| `scene_tree_process_node_added_optimized` | NodeAdded with pre-analyzed types |
| `scene_tree_process_node_added_optimized_2500` | Scaling variant (2500 nodes) |
| `scene_tree_process_node_added_fallback` | NodeAdded with FFI type detection |
| `scene_tree_process_node_added_populated_world` | Adding 10 nodes to a world with 10k existing entities (exposes per-batch costs that scale with world size, not batch size) |
| `scene_tree_process_node_renamed_sparse_updates` | Sparse rename messages (80 frames) |
| `scene_tree_process_collision_bodies_optimized` | Area3D with collision signals (optimized path, 100 nodes) |

### Collision Processing (3 benchmarks)

| Benchmark | What It Tests |
|-----------|---------------|
| `scene_tree_process_collision_bodies_fallback` | Area3D with collision signals (fallback FFI path, 100 nodes) |
| `collisions_process_start_end_burst` | Burst start/end events (200 nodes x 200 cycles) |
| `collisions_process_start_end_burst_1000` | Scaling variant: 5x concurrent pairs, same message volume (1000 x 40) |

### Input Action Checking (1 benchmark)

| Benchmark | What It Tests |
|-----------|---------------|
| `input_action_checking_many_events_many_actions` | 100 events x 50 actions |

### Packed Scene Spawning (1 benchmark, 100 scenes)

| Benchmark | What It Tests |
|-----------|---------------|
| `packed_scene_batch_spawn` | Batch spawn 100 instances (per-frame cache) |

### Signal System (3 benchmarks, 200 nodes unless suffixed)

| Benchmark | What It Tests |
|-----------|---------------|
| `signal_dispatch_throughput` | Full emit -> drain -> trigger pipeline |
| `signal_dispatch_throughput_1000` | Scaling variant (1000 nodes) |
| `signal_connection_setup` | FFI cost of 200 Callable+connect calls |

## CI Integration

### Automated Regression Detection

Every PR automatically runs benchmarks and compares against the `main` branch baseline:

1. **PR opened/updated** -> Benchmarks run automatically
2. **Compare** against baseline from `main`
3. **Comment on PR** with results table
4. **Fail if regression** > 10% slowdown detected

### Baseline Generation

There is no stored baseline. Each PR run builds the base branch (via a git
worktree) and benchmarks it on the same runner, right before benchmarking the
PR branch. This keeps comparisons hardware-consistent — a stored baseline from
a different runner or an older toolchain would produce false regressions.

Each side runs twice and the runs are merged (median of medians) to reduce
noise. The PR's benchmark definitions are used for the baseline build when
they compile against the base branch, so new benchmarks get baseline numbers
too.

## Adding New Benchmarks

### 1. Write the Benchmark

Add to `itest/rust/src/benchmarks.rs`:

```rust
use godot_bevy_test::{bench, measured};

#[bench(repeat = 3)]  // Optional: custom inner repetitions
fn my_new_benchmark() -> i32 {
    // Setup is NOT timed when you scope the hot section with measured()
    let mut app = setup_benchmark_app();

    // Only this section contributes to the reported time
    let result = measured(|| do_expensive_operation(&mut app));

    // Teardown is not timed either
    cleanup(app);
    result
}
```

**Requirements:**
- Must return a value (prevents compiler optimization)
- Wrap the system under test in `measured(|| ...)` so setup/teardown don't
  dilute the signal (without it, the whole function is timed)
- Should be deterministic (avoid randomness)
- Keep runtime reasonable (< 1 second per iteration)
- Clean up nodes with `node.free()`, not `queue_free()` — benchmarks run
  synchronously within one frame, so deferred frees accumulate in the scene
  tree across iterations and skew later benchmarks
- Test the real plugin/system, not raw FFI (see [CLAUDE.md](../CLAUDE.md) Benchmark Philosophy)

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
├── compare-benches.sh          # A/B comparison (current vs base branch)
└── rust/
    └── src/
        └── benchmarks.rs       # All benchmark definitions

godot-bevy-test/
└── src/
    ├── bencher.rs              # Benchmark runner (warmup, stats)
    └── lib.rs                  # #[bench] proc-macro re-export

.github/
├── workflows/
│   ├── benchmarks.yml          # Runs benchmarks, checks regressions, saves artifacts
│   └── benchmark-comment.yml   # Posts results as PR comments (elevated permissions)
└── scripts/
    └── benchmarks-compare.py   # JSON comparison and regression detection
```

### How It Works

1. **`#[bench]` macro** expands to:
   - Wrapper function that runs code N times
   - Registration in plugin system
   - Uses `std::hint::black_box()` to prevent optimization

2. **Benchmark runner** (`godot-bevy-test/src/bencher.rs`):
   - 5 warmup runs
   - 21 test runs (odd number for clean median)
   - Reports min and median (ignores outliers)
   - Times the `measured(|| ...)` scope when the benchmark uses one,
     otherwise the whole function

3. **CI workflows**:
   - **`benchmarks.yml`**: Runs benchmarks, checks regressions, saves artifacts
   - **`benchmark-comment.yml`**: Posts results as PR comments (runs with elevated permissions via `workflow_run`)
   - Two-workflow pattern needed to support PRs from forks (security requirement)

## Why itest Benchmarks vs perf-test?

### itest Benchmarks (THIS - Used in CI)

- **Fast**: ~2 minutes total
- **Deterministic**: Measures exact us/ms
- **Focused**: Tests specific systems and plugins
- **Headless**: No rendering overhead
- **CI-friendly**: Runs on every PR

### perf-test Example (Demo/Manual Testing)

- **End-to-end**: Full game loop with rendering
- **Visual**: Users can see and interact
- **Comparison**: GDScript vs Bevy implementations
- Slow: ~10+ minutes to run
- Flaky: FPS varies with hardware

**Recommendation**: Use itest benchmarks for CI regression detection, keep perf-test as user-facing demo.

## Result Format

Benchmark JSON (produced with `BENCHMARK_JSON=1 BENCHMARK_JSON_PATH=out.json`):

```json
{
  "timestamp": "2025-10-16T15:53:12.483786",
  "benchmarks": {
    "transform_sync_bevy_to_godot_3d": {
      "min_ns": 1580000.0,
      "median_ns": 1590000.0,
      "min_display": "1.58ms",
      "median_display": "1.59ms"
    }
  }
}
```

## Regression Detection

### Threshold

Default: **10%** slowdown triggers a regression warning.

The comparison script (`benchmarks-compare.py`) classifies each benchmark as:

| Change | Status |
|--------|--------|
| > +10% | `regression` (flagged in PR) |
| +5% to +10% | `slower` (noted, not flagged) |
| -5% to +5% | `neutral` |
| < -5% | `faster` (improvement) |

### What Triggers Failure

A benchmark is flagged as a regression if:
```
(current_median - baseline_median) / baseline_median > 0.10  # More than 10% slower
```

### Example PR Comment

```markdown
## Benchmark Results

| Benchmark | Median | Min |
|-----------|--------|-----|
| transform_sync_bevy_to_godot_3d | 1.62ms | 1.58ms |
| transform_sync_godot_to_bevy_3d | 1.44ms | 1.40ms |
| scene_tree_process_node_added_optimized | 1.92ms | 1.80ms |

### Regression Check

Performance Regressions Detected:

**scene_tree_process_node_added_optimized**:
  - Baseline: 0.95ms
  - Current:  1.92ms
  - Regression: +101.1% slower

> Warning: Performance regressions detected. Please investigate before merging.
```

## Troubleshooting

### "No benchmarks found"

Check that:
- Benchmarks are in `itest/rust/src/benchmarks.rs`
- Using `#[bench]` macro from `godot_bevy_test`
- Function returns a value
- `mod benchmarks;` is in `lib.rs`

### "Benchmarks taking too long"

Reduce entity count constants or reduce the `repeat` parameter:
```rust
const NODE_COUNT: usize = 500;  // Down from 1000

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

- [ ] Benchmark asset loading performance
- [ ] Add audio system benchmarks (deferred: <100 concurrent sounds is not a realistic bottleneck)
- [ ] Add memory usage tracking
- [ ] Benchmark parallel ECS systems

## References

- [CLAUDE.md](../CLAUDE.md) - Benchmark philosophy, PackedArray optimization pattern
- [gdext benchmarking](https://github.com/godot-rust/gdext/tree/master/itest) - Inspiration for this system
- [Criterion.rs](https://github.com/bheisler/criterion.rs) - Statistical benchmarking (not used, but relevant)
