//! Benchmark infrastructure for godot-bevy integration benchmarks

use std::time::{Duration, Instant};

const WARMUP_RUNS: usize = 5;
const TEST_RUNS: usize = 21;
const METRIC_COUNT: usize = 2;

/// Result of running a benchmark
pub struct BenchResult {
    pub stats: [Duration; METRIC_COUNT],
}

/// Get the metric names for benchmark output
pub fn metrics() -> [&'static str; METRIC_COUNT] {
    ["min", "median"]
}

/// Run a benchmark function with warmup and multiple iterations
pub fn run_benchmark(code: fn(), inner_repetitions: usize) -> BenchResult {
    for _ in 0..WARMUP_RUNS {
        code();
    }

    let mut times = Vec::with_capacity(TEST_RUNS);
    for _ in 0..TEST_RUNS {
        let start = Instant::now();
        code();
        let duration = start.elapsed();

        times.push(duration / inner_repetitions as u32);
    }
    times.sort();

    calculate_stats(times)
}

fn calculate_stats(times: Vec<Duration>) -> BenchResult {
    let min = times[0];
    let median = times[TEST_RUNS / 2];

    BenchResult {
        stats: [min, median],
    }
}
