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
Benchmark Results:
                                                                    min       median
itest/rust/src/benchmarks.rs:14
  transform_sync_bevy_to_godot_3d                                1.58ms       1.62ms
  transform_sync_godot_to_bevy_3d                                1.40ms       1.44ms
  transform_sync_bevy_to_godot_2d                                1.52ms       1.56ms
  transform_sync_godot_to_bevy_2d                                1.36ms       1.40ms
  transform_sync_roundtrip_3d                                    3.10ms       3.20ms
  transform_sync_roundtrip_2d                                    2.98ms       3.06ms
  scene_tree_idle_no_messages                                    0.02ms       0.03ms
  scene_tree_process_node_added_optimized                        1.80ms       1.92ms
  scene_tree_process_node_added_fallback                         3.40ms       3.55ms
  scene_tree_process_node_renamed_sparse_updates                 0.45ms       0.50ms
  scene_tree_process_collision_bodies_optimized                   1.10ms       1.18ms
  scene_tree_process_collision_bodies_fallback                    2.20ms       2.35ms
  collisions_process_start_end_burst                             5.80ms       6.10ms
  input_action_checking_many_events_many_actions                 0.90ms       0.95ms
  packed_scene_batch_spawn                                       4.20ms       4.40ms
  packed_scene_spawn_with_transforms                             4.50ms       4.70ms
  signal_dispatch_throughput                                      0.60ms       0.65ms
  signal_idle_no_signals                                         0.01ms       0.02ms
  signal_connection_setup                                        1.20ms       1.30ms

Benchmarks completed in 85.32s.
```

## What We Benchmark

The suite contains **20 benchmarks** across six categories. Every benchmark runs the real godot-bevy systems (plugins, schedules, ECS queries) rather than raw FFI calls, so regressions in actual user-facing code are caught.

### Transform Synchronization (6 benchmarks, 1000 entities)

These benchmarks measure the real `GodotTransformSyncPlugin` systems that sync transforms between Bevy ECS and Godot nodes.

| Benchmark | What It Tests |
|-----------|---------------|
| `transform_sync_bevy_to_godot_3d` | Bevy->Godot 3D sync (Last schedule) |
| `transform_sync_godot_to_bevy_3d` | Godot->Bevy 3D sync (PreUpdate schedule) |
| `transform_sync_bevy_to_godot_2d` | Bevy->Godot 2D sync (Last schedule) |
| `transform_sync_godot_to_bevy_2d` | Godot->Bevy 2D sync (PreUpdate schedule) |
| `transform_sync_roundtrip_3d` | Full frame: PreUpdate -> game logic -> Last (3D) |
| `transform_sync_roundtrip_2d` | Full frame: PreUpdate -> game logic -> Last (2D) |

### Scene Tree Processing (5 benchmarks, 500 nodes)

These benchmarks measure the `GodotSceneTreePlugin` systems that process node-added, renamed, and collision-body messages from Godot.

| Benchmark | What It Tests |
|-----------|---------------|
| `scene_tree_idle_no_messages` | Per-frame overhead when stable (200 frames) |
| `scene_tree_process_node_added_optimized` | NodeAdded with pre-analyzed types |
| `scene_tree_process_node_added_fallback` | NodeAdded with FFI type detection |
| `scene_tree_process_node_renamed_sparse_updates` | Sparse rename messages (80 frames) |
| `scene_tree_process_collision_bodies_optimized` | Area3D with collision signals (optimized path, 100 nodes) |

### Collision Processing (2 benchmarks)

| Benchmark | What It Tests |
|-----------|---------------|
| `scene_tree_process_collision_bodies_fallback` | Area3D with collision signals (fallback FFI path, 100 nodes) |
| `collisions_process_start_end_burst` | Burst start/end events (200 nodes x 200 cycles) |

### Input Action Checking (1 benchmark)

| Benchmark | What It Tests |
|-----------|---------------|
| `input_action_checking_many_events_many_actions` | 100 events x 50 actions |

### Packed Scene Spawning (2 benchmarks, 100 scenes)

| Benchmark | What It Tests |
|-----------|---------------|
| `packed_scene_batch_spawn` | Batch spawn 100 instances (per-frame cache) |
| `packed_scene_spawn_with_transforms` | Batch spawn with transform application |

### Signal System (3 benchmarks, 200 nodes)

| Benchmark | What It Tests |
|-----------|---------------|
| `signal_dispatch_throughput` | Full emit -> drain -> trigger pipeline |
| `signal_idle_no_signals` | Per-frame overhead when idle (200 frames) |
| `signal_connection_setup` | FFI cost of 200 Callable+connect calls |

## CI Integration

### Automated Regression Detection

Every PR automatically runs benchmarks and compares against the `main` branch baseline:

1. **PR opened/updated** -> Benchmarks run automatically
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
use godot_bevy_test::bench;

#[bench(repeat = 3)]  // Optional: custom inner repetitions
fn my_new_benchmark() -> i32 {
    // Benchmark code - must return a value to prevent dead-code elimination.
    // Will be run 3 times internally per iteration,
    // plus 5 warmup + 21 test runs by the harness.

    let result = do_expensive_operation();
    result
}
```

**Requirements:**
- Must return a value (prevents compiler optimization)
- Should be deterministic (avoid randomness)
- Keep runtime reasonable (< 1 second per iteration)
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
├── baseline.json               # CI-generated baseline
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

## Baseline Management

### Format

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
  },
  "ci_metadata": {
    "commit": "abc123...",
    "timestamp": "2025-10-16T15:53:12Z",
    "runner": "GitHub Actions"
  }
}
```

### Manual Baseline Update

```bash
cd itest
# Run benchmarks with JSON output, then copy the result to baseline.json
BENCHMARK_JSON=1 BENCHMARK_JSON_PATH=baseline.json ./run-benches.sh
git add baseline.json
git commit -m "chore: update benchmark baseline"
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
- [ ] Compare different entity counts (scaling)
- [ ] Add memory usage tracking
- [ ] Benchmark parallel ECS systems

## References

- [CLAUDE.md](../CLAUDE.md) - Benchmark philosophy, PackedArray optimization pattern
- [gdext benchmarking](https://github.com/godot-rust/gdext/tree/master/itest) - Inspiration for this system
- [Criterion.rs](https://github.com/bheisler/criterion.rs) - Statistical benchmarking (not used, but relevant)
