//! Benchmark infrastructure for godot-bevy integration benchmarks

use std::cell::Cell;
use std::time::{Duration, Instant};

const WARMUP_RUNS: usize = 5;
const TEST_RUNS: usize = 21;
const METRIC_COUNT: usize = 2;

thread_local! {
    static MEASURED_NS: Cell<Option<u128>> = const { Cell::new(None) };
}

/// Result of running a benchmark
pub struct BenchResult {
    pub stats: [Duration; METRIC_COUNT],
}

/// Get the metric names for benchmark output
pub fn metrics() -> [&'static str; METRIC_COUNT] {
    ["min", "median"]
}

/// Scope timing to the hot section of a benchmark.
///
/// By default the whole benchmark function is timed, including setup and
/// teardown. Wrapping the section under test in `measured(|| ...)` excludes
/// everything else from the reported time. Multiple calls within one run
/// accumulate.
pub fn measured<R>(f: impl FnOnce() -> R) -> R {
    let start = Instant::now();
    let result = f();
    let elapsed = start.elapsed().as_nanos();
    MEASURED_NS.with(|m| m.set(Some(m.get().unwrap_or(0) + elapsed)));
    result
}

/// Run a benchmark function with warmup and multiple iterations
pub fn run_benchmark(code: fn(), inner_repetitions: usize) -> BenchResult {
    for _ in 0..WARMUP_RUNS {
        MEASURED_NS.with(|m| m.set(None));
        code();
    }

    let mut times = Vec::with_capacity(TEST_RUNS);
    for _ in 0..TEST_RUNS {
        MEASURED_NS.with(|m| m.set(None));
        let start = Instant::now();
        code();
        let wall = start.elapsed();

        // Prefer the measured() scope when the benchmark used one
        let duration = match MEASURED_NS.with(|m| m.get()) {
            Some(ns) => Duration::from_nanos(ns as u64),
            None => wall,
        };

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
