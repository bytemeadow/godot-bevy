//! Benchmark infrastructure for godot-bevy integration benchmarks

use std::cell::Cell;
use std::time::{Duration, Instant};

pub(crate) const WARMUP_RUNS: usize = 5;
pub(crate) const TEST_RUNS: usize = 21;
const METRIC_COUNT: usize = 2;

thread_local! {
    static MEASURED_NS: Cell<Option<u128>> = const { Cell::new(None) };
    #[cfg(feature = "profile-tracy")]
    static PROFILE_CONTEXT: Cell<Option<ProfileContext>> = const { Cell::new(None) };
}

#[cfg(feature = "profile-tracy")]
#[derive(Clone, Copy)]
struct ProfileContext {
    benchmark: &'static str,
    phase: &'static str,
    iteration: usize,
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
    #[cfg(feature = "profile-tracy")]
    let _profile_span = PROFILE_CONTEXT.with(|context| {
        context.get().map(|context| {
            crate::profiling::measured_span(context.benchmark, context.phase, context.iteration)
        })
    });
    let start = Instant::now();
    let result = f();
    let elapsed = start.elapsed().as_nanos();
    MEASURED_NS.with(|m| m.set(Some(m.get().unwrap_or(0) + elapsed)));
    result
}

/// Run a benchmark function with warmup and multiple iterations
pub fn run_benchmark(code: fn(), inner_repetitions: usize) -> BenchResult {
    run_benchmark_named("unnamed", code, inner_repetitions)
}

pub(crate) fn run_benchmark_named(
    _benchmark: &'static str,
    code: fn(),
    inner_repetitions: usize,
) -> BenchResult {
    #[cfg(feature = "profile-tracy")]
    let _benchmark_span = crate::profiling::benchmark_span(_benchmark, inner_repetitions);

    #[cfg(feature = "profile-tracy")]
    let _warmup_phase = crate::profiling::phase_span(_benchmark, "warmup");
    for _iteration in 0..WARMUP_RUNS {
        MEASURED_NS.with(|m| m.set(None));
        #[cfg(feature = "profile-tracy")]
        set_profile_context(_benchmark, "warmup", _iteration);
        #[cfg(feature = "profile-tracy")]
        let _iteration_span =
            crate::profiling::iteration_span(_benchmark, "warmup", _iteration, inner_repetitions);
        code();
        #[cfg(feature = "profile-tracy")]
        clear_profile_context();
    }
    #[cfg(feature = "profile-tracy")]
    drop(_warmup_phase);

    let mut times = Vec::with_capacity(TEST_RUNS);
    #[cfg(feature = "profile-tracy")]
    let _sample_phase = crate::profiling::phase_span(_benchmark, "sample");
    for _iteration in 0..TEST_RUNS {
        MEASURED_NS.with(|m| m.set(None));
        #[cfg(feature = "profile-tracy")]
        set_profile_context(_benchmark, "sample", _iteration);
        #[cfg(feature = "profile-tracy")]
        let _iteration_span =
            crate::profiling::iteration_span(_benchmark, "sample", _iteration, inner_repetitions);
        let start = Instant::now();
        code();
        let wall = start.elapsed();
        #[cfg(feature = "profile-tracy")]
        clear_profile_context();

        // Prefer the measured() scope when the benchmark used one
        let duration = match MEASURED_NS.with(|m| m.get()) {
            Some(ns) => Duration::from_nanos(ns as u64),
            None => wall,
        };

        times.push(duration / inner_repetitions as u32);
    }
    #[cfg(feature = "profile-tracy")]
    drop(_sample_phase);
    times.sort();

    calculate_stats(times)
}

#[cfg(feature = "profile-tracy")]
fn set_profile_context(benchmark: &'static str, phase: &'static str, iteration: usize) {
    PROFILE_CONTEXT.with(|context| {
        context.set(Some(ProfileContext {
            benchmark,
            phase,
            iteration,
        }));
    });
}

#[cfg(feature = "profile-tracy")]
fn clear_profile_context() {
    PROFILE_CONTEXT.with(|context| context.set(None));
}

fn calculate_stats(times: Vec<Duration>) -> BenchResult {
    let min = times[0];
    let median = times[TEST_RUNS / 2];

    BenchResult {
        stats: [min, median],
    }
}
