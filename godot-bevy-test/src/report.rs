use serde::Serialize;
use serde_json::Value;
use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use crate::config::TestConfig;

const SCHEMA_NAME: &str = "itest-report-v1.schema.json";
static NEXT_FILE_ID: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum RunOutcome {
    Incomplete,
    Pass,
    Fail,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum TestOutcome {
    Pass,
    Fail,
    Flaky,
    Skip,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum AttemptOutcome {
    Pass,
    Fail,
}

#[derive(Debug, Serialize)]
pub(crate) struct TestReport {
    #[serde(rename = "$schema")]
    schema: &'static str,
    schema_version: u32,
    pub(crate) run_id: String,
    runner_version: &'static str,
    pub(crate) complete: bool,
    pub(crate) outcome: RunOutcome,
    environment: Environment,
    pub(crate) selection: Selection,
    pub(crate) repeat: u32,
    pub(crate) timeout_frames: u32,
    pub(crate) summary: Summary,
    pub(crate) tests: Vec<LogicalTest>,
    pub(crate) errors: Vec<ReportError>,
    artifacts: Vec<Artifact>,
    metadata: BTreeMap<String, Value>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Environment {
    build_profile: String,
    debug_assertions: bool,
    godot_version: String,
    os: &'static str,
    arch: &'static str,
}

#[derive(Debug, Serialize)]
pub(crate) struct Selection {
    pub(crate) registered: usize,
    pub(crate) selected: usize,
    pub(crate) focus_run: bool,
    pub(crate) filter: Option<String>,
    pub(crate) patterns: Vec<String>,
}

#[derive(Debug, Default, Serialize)]
pub(crate) struct Summary {
    pub(crate) passed: usize,
    pub(crate) failed: usize,
    pub(crate) flaky: usize,
    pub(crate) skipped: usize,
    pub(crate) total: usize,
    pub(crate) attempts_passed: usize,
    pub(crate) attempts_failed: usize,
    pub(crate) total_duration_ms: f64,
}

#[derive(Debug, Serialize)]
pub(crate) struct LogicalTest {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) file: String,
    pub(crate) line: u32,
    pub(crate) outcome: TestOutcome,
    pub(crate) duration_ms: f64,
    pub(crate) attempts: Vec<Attempt>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Attempt {
    pub(crate) id: String,
    pub(crate) index: u32,
    pub(crate) outcome: AttemptOutcome,
    pub(crate) duration_ms: f64,
    pub(crate) failures: Vec<Failure>,
    artifacts: Vec<Artifact>,
}

#[derive(Debug, Serialize)]
pub(crate) struct Failure {
    pub(crate) kind: String,
    pub(crate) message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) gdext_context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) callback: Option<String>,
}

#[derive(Debug, Serialize)]
pub(crate) struct ReportError {
    pub(crate) kind: String,
    pub(crate) message: String,
}

#[derive(Debug, Serialize)]
struct Artifact {
    kind: String,
    path: String,
    metadata: BTreeMap<String, Value>,
}

impl TestReport {
    pub(crate) fn new(config: &TestConfig, selection: Selection, godot_version: String) -> Self {
        Self {
            schema: SCHEMA_NAME,
            schema_version: 1,
            run_id: new_run_id(),
            runner_version: env!("CARGO_PKG_VERSION"),
            complete: false,
            outcome: RunOutcome::Incomplete,
            environment: Environment {
                build_profile: config.build_profile.clone(),
                debug_assertions: cfg!(debug_assertions),
                godot_version,
                os: std::env::consts::OS,
                arch: std::env::consts::ARCH,
            },
            selection,
            repeat: config.repeat,
            timeout_frames: config.timeout_frames,
            summary: Summary::default(),
            tests: Vec::new(),
            errors: Vec::new(),
            artifacts: Vec::new(),
            metadata: BTreeMap::new(),
        }
    }

    pub(crate) fn push_test(&mut self, test: LogicalTest, elapsed: Duration) {
        match test.outcome {
            TestOutcome::Pass => self.summary.passed += 1,
            TestOutcome::Fail => self.summary.failed += 1,
            TestOutcome::Flaky => self.summary.flaky += 1,
            TestOutcome::Skip => self.summary.skipped += 1,
        }
        for attempt in &test.attempts {
            match attempt.outcome {
                AttemptOutcome::Pass => self.summary.attempts_passed += 1,
                AttemptOutcome::Fail => self.summary.attempts_failed += 1,
            }
        }
        self.tests.push(test);
        self.summary.total = self.tests.len();
        self.summary.total_duration_ms = duration_ms(elapsed);
    }

    pub(crate) fn finish(&mut self, elapsed: Duration) {
        self.complete = true;
        self.outcome = if self.summary.failed > 0 || self.summary.flaky > 0 {
            RunOutcome::Fail
        } else {
            RunOutcome::Pass
        };
        self.summary.total_duration_ms = duration_ms(elapsed);
    }

    pub(crate) fn finish_error(&mut self, kind: &str, message: String) {
        self.complete = true;
        self.outcome = RunOutcome::Error;
        self.errors.push(ReportError {
            kind: kind.to_string(),
            message,
        });
    }
}

impl LogicalTest {
    pub(crate) fn skipped(name: &str, file: &str, line: u32) -> Self {
        Self {
            id: test_id(file, name),
            name: name.to_string(),
            file: file.to_string(),
            line,
            outcome: TestOutcome::Skip,
            duration_ms: 0.0,
            attempts: Vec::new(),
        }
    }

    pub(crate) fn from_attempts(name: &str, file: &str, line: u32, attempts: Vec<Attempt>) -> Self {
        let passed = attempts
            .iter()
            .filter(|attempt| attempt.outcome == AttemptOutcome::Pass)
            .count();
        let outcome = if passed == attempts.len() {
            TestOutcome::Pass
        } else if passed == 0 {
            TestOutcome::Fail
        } else {
            TestOutcome::Flaky
        };
        let duration_ms = attempts.iter().map(|attempt| attempt.duration_ms).sum();

        Self {
            id: test_id(file, name),
            name: name.to_string(),
            file: file.to_string(),
            line,
            outcome,
            duration_ms,
            attempts,
        }
    }
}

impl Attempt {
    pub(crate) fn new(
        test_id: &str,
        index: u32,
        duration: Duration,
        failures: Vec<Failure>,
    ) -> Self {
        Self {
            id: format!("{test_id}#{index}"),
            index,
            outcome: if failures.is_empty() {
                AttemptOutcome::Pass
            } else {
                AttemptOutcome::Fail
            },
            duration_ms: duration_ms(duration),
            failures,
            artifacts: Vec::new(),
        }
    }
}

pub(crate) struct ReportWriter {
    path: Option<PathBuf>,
}

impl ReportWriter {
    pub(crate) fn new(path: Option<PathBuf>) -> Self {
        Self { path }
    }

    pub(crate) fn write(&self, report: &TestReport) -> io::Result<Option<String>> {
        let Some(path) = &self.path else {
            return Ok(None);
        };
        let json = serde_json::to_string_pretty(report).map_err(io::Error::other)?;
        let parent = path.parent().ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidInput, "report path has no parent")
        })?;
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| {
                io::Error::new(io::ErrorKind::InvalidInput, "report path has no file name")
            })?;
        let file_id = NEXT_FILE_ID.fetch_add(1, Ordering::Relaxed);
        let temporary = parent.join(format!(
            ".{file_name}.{}.{}.tmp",
            std::process::id(),
            file_id
        ));

        if let Err(error) = fs::write(&temporary, json.as_bytes()) {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        if let Err(error) = fs::rename(&temporary, path) {
            let _ = fs::remove_file(&temporary);
            return Err(error);
        }
        Ok(Some(json))
    }
}

pub(crate) fn test_id(file: &str, name: &str) -> String {
    format!("{file}::{name}")
}

fn duration_ms(duration: Duration) -> f64 {
    duration.as_secs_f64() * 1000.0
}

fn new_run_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    format!("{nanos}-{}", std::process::id())
}

#[cfg(test)]
include!("report_tests.rs");
