use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use tracing::span::EnteredSpan;
use tracing_subscriber::fmt::format::DefaultFields;
use tracing_subscriber::layer::SubscriberExt;

const CONNECT_TIMEOUT: Duration = Duration::from_secs(10);
const CONNECT_POLL_INTERVAL: Duration = Duration::from_millis(25);

static INSTALL_RESULT: OnceLock<Result<(), String>> = OnceLock::new();
static LAYER_ERRORS: Mutex<Vec<String>> = Mutex::new(Vec::new());

struct ProfileTracyConfig {
    formatter: DefaultFields,
}

impl tracing_tracy::Config for ProfileTracyConfig {
    type Formatter = DefaultFields;

    fn formatter(&self) -> &Self::Formatter {
        &self.formatter
    }

    fn stack_depth(&self, _: &tracing::Metadata<'_>) -> u16 {
        0
    }

    fn format_fields_in_zone_name(&self) -> bool {
        true
    }

    fn on_error(&self, client: &tracing_tracy::client::Client, error: &'static str) {
        if let Ok(mut errors) = LAYER_ERRORS.lock() {
            errors.push(error.to_string());
        }
        client.color_message(error, 0xFF000000, 0);
    }
}

pub fn install_profile_subscriber() {
    let _ = INSTALL_RESULT.get_or_init(|| {
        if tracing::dispatcher::has_been_set() {
            return Err("a global tracing subscriber was already installed".to_string());
        }
        let layer = tracing_tracy::TracyLayer::new(ProfileTracyConfig {
            formatter: DefaultFields::new(),
        });
        tracing::subscriber::set_global_default(tracing_subscriber::registry().with(layer))
            .map_err(|error| format!("failed to install profile tracing subscriber: {error}"))
    });
}

pub(crate) fn subscriber_status() -> Result<(), String> {
    match INSTALL_RESULT.get() {
        Some(result) => result.clone(),
        None => Err("profile tracing subscriber was not installed before the runner".to_string()),
    }
}

pub(crate) fn wait_for_connection() -> Result<(), String> {
    let deadline = Instant::now() + CONNECT_TIMEOUT;
    while Instant::now() < deadline {
        if tracing_tracy::client::Client::is_connected() {
            write_gate_status("connected")?;
            return Ok(());
        }
        std::thread::sleep(CONNECT_POLL_INTERVAL);
    }
    write_gate_status("timeout")?;
    Err("Tracy did not connect within 10 seconds".to_string())
}

fn write_gate_status(status: &str) -> Result<(), String> {
    let path = std::env::var("GBPROF_GATE_PATH")
        .map_err(|_| "GBPROF_GATE_PATH is required".to_string())?;
    if path.trim().is_empty() {
        return Err("GBPROF_GATE_PATH must not be empty".to_string());
    }
    std::fs::write(&path, status)
        .map_err(|error| format!("failed to write Tracy gate status to {path}: {error}"))
}

pub(crate) fn layer_errors() -> Vec<String> {
    LAYER_ERRORS
        .lock()
        .map(|errors| errors.clone())
        .unwrap_or_else(|_| vec!["profile layer error state was poisoned".to_string()])
}

pub(crate) fn mark_run_begin(run_id: &str) {
    let _marker = tracing::info_span!("__gbprof::run_begin", run_id = %run_id).entered();
}

pub(crate) fn mark_run_end(run_id: &str) {
    let _marker = tracing::info_span!("__gbprof::run_end", run_id = %run_id).entered();
}

pub(crate) fn benchmark_span(benchmark: &str, inner_repetitions: usize) -> EnteredSpan {
    tracing::info_span!(
        "__gbprof::benchmark",
        benchmark = %benchmark,
        inner_repetitions
    )
    .entered()
}

pub(crate) fn phase_span(benchmark: &str, phase: &str) -> EnteredSpan {
    tracing::info_span!(
        "__gbprof::phase",
        benchmark = %benchmark,
        phase = %phase
    )
    .entered()
}

pub(crate) fn iteration_span(
    benchmark: &str,
    phase: &str,
    iteration: usize,
    inner_repetitions: usize,
) -> EnteredSpan {
    tracing::info_span!(
        "__gbprof::iteration",
        benchmark = %benchmark,
        phase = %phase,
        iteration,
        inner_repetitions
    )
    .entered()
}

pub(crate) fn measured_span(benchmark: &str, phase: &str, iteration: usize) -> EnteredSpan {
    tracing::info_span!(
        "__gbprof::measured",
        benchmark = %benchmark,
        phase = %phase,
        iteration
    )
    .entered()
}
