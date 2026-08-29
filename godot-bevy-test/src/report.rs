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
mod tests {
    use super::*;

    fn failure(message: &str) -> Failure {
        Failure {
            kind: "assertion".to_string(),
            message: message.to_string(),
            location: Some("src/test.rs:7".to_string()),
            gdext_context: None,
            callback: None,
        }
    }

    fn report() -> TestReport {
        TestReport::new(
            &TestConfig::fallback(),
            Selection {
                registered: 4,
                selected: 4,
                focus_run: false,
                filter: None,
                patterns: Vec::new(),
            },
            "4.6.2.stable".to_string(),
        )
    }

    #[test]
    fn report_v1_contract() {
        let config = TestConfig::fallback();
        let mut report = TestReport::new(
            &config,
            Selection {
                registered: 1,
                selected: 1,
                focus_run: false,
                filter: Some("contract".to_string()),
                patterns: vec!["contract".to_string()],
            },
            "4.6.2.stable".to_string(),
        );
        let path = std::env::temp_dir().join(format!("itest-report-{}.json", report.run_id));
        let writer = ReportWriter::new(Some(path.clone()));
        let initial = writer.write(&report).unwrap().unwrap();
        assert_eq!(
            serde_json::from_str::<Value>(&initial).unwrap()["complete"],
            false
        );

        let id = test_id("src/contract.rs", "contract_test");
        report.push_test(
            LogicalTest::from_attempts(
                "contract_test",
                "src/contract.rs",
                12,
                vec![Attempt::new(&id, 1, Duration::from_millis(2), Vec::new())],
            ),
            Duration::from_millis(2),
        );
        report.finish(Duration::from_millis(2));
        let final_json = writer.write(&report).unwrap().unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), final_json);
        std::fs::remove_file(path).unwrap();

        let value = serde_json::to_value(&report).unwrap();
        assert_eq!(value["$schema"], SCHEMA_NAME);
        assert_eq!(value["schema_version"], 1);
        assert_eq!(value["runner_version"], env!("CARGO_PKG_VERSION"));
        assert_eq!(value["complete"], true);
        assert_eq!(value["outcome"], "pass");
        assert_eq!(value["tests"][0]["id"], "src/contract.rs::contract_test");
        assert_eq!(value["tests"][0]["attempts"][0]["index"], 1);
        assert!(value["metadata"].is_object());
        assert!(value["artifacts"].is_array());

        let schema: Value =
            serde_json::from_str(include_str!("../schema/itest-report-v1.schema.json")).unwrap();
        assert_eq!(schema["properties"]["schema_version"]["const"], 1);
        assert_eq!(
            schema["$id"],
            "https://github.com/bytemeadow/godot-bevy/itest-report-v1.schema.json"
        );
    }

    #[test]
    fn logical_outcomes_and_durations_are_exact() {
        let id = test_id("src/test.rs", "matrix");
        let pass = LogicalTest::from_attempts(
            "pass",
            "src/test.rs",
            1,
            vec![
                Attempt::new(&id, 1, Duration::from_millis(2), Vec::new()),
                Attempt::new(&id, 2, Duration::from_millis(3), Vec::new()),
            ],
        );
        assert_eq!(pass.outcome, TestOutcome::Pass);
        assert_eq!(pass.duration_ms, 5.0);

        let fail = LogicalTest::from_attempts(
            "fail",
            "src/test.rs",
            2,
            vec![
                Attempt::new(&id, 1, Duration::from_millis(7), vec![failure("one")]),
                Attempt::new(&id, 2, Duration::from_millis(11), vec![failure("two")]),
            ],
        );
        assert_eq!(fail.outcome, TestOutcome::Fail);
        assert_eq!(fail.duration_ms, 18.0);

        let flaky = LogicalTest::from_attempts(
            "flaky",
            "src/test.rs",
            3,
            vec![
                Attempt::new(&id, 1, Duration::from_millis(13), Vec::new()),
                Attempt::new(&id, 2, Duration::from_millis(17), vec![failure("three")]),
            ],
        );
        assert_eq!(flaky.outcome, TestOutcome::Flaky);
        assert_eq!(flaky.duration_ms, 30.0);
    }

    #[test]
    fn push_test_counts_every_outcome_and_attempt() {
        let id = test_id("src/test.rs", "counts");
        let mut report = report();
        report.push_test(
            LogicalTest::from_attempts(
                "pass",
                "src/test.rs",
                1,
                vec![Attempt::new(&id, 1, Duration::from_millis(2), Vec::new())],
            ),
            Duration::from_millis(2),
        );
        report.push_test(
            LogicalTest::from_attempts(
                "fail",
                "src/test.rs",
                2,
                vec![Attempt::new(
                    &id,
                    1,
                    Duration::from_millis(3),
                    vec![failure("fail")],
                )],
            ),
            Duration::from_millis(5),
        );
        report.push_test(
            LogicalTest::from_attempts(
                "flaky",
                "src/test.rs",
                3,
                vec![
                    Attempt::new(&id, 1, Duration::from_millis(5), Vec::new()),
                    Attempt::new(&id, 2, Duration::from_millis(7), vec![failure("flaky")]),
                ],
            ),
            Duration::from_millis(17),
        );
        report.push_test(
            LogicalTest::skipped("skip", "src/test.rs", 4),
            Duration::from_millis(17),
        );

        assert_eq!(report.summary.passed, 1);
        assert_eq!(report.summary.failed, 1);
        assert_eq!(report.summary.flaky, 1);
        assert_eq!(report.summary.skipped, 1);
        assert_eq!(report.summary.total, 4);
        assert_eq!(report.summary.attempts_passed, 2);
        assert_eq!(report.summary.attempts_failed, 2);
        assert_eq!(report.summary.total_duration_ms, 17.0);

        let value = serde_json::to_value(&report).unwrap();
        assert_eq!(value["summary"]["passed"], 1);
        assert_eq!(value["summary"]["failed"], 1);
        assert_eq!(value["summary"]["flaky"], 1);
        assert_eq!(value["summary"]["skipped"], 1);
        assert_eq!(value["summary"]["attempts_passed"], 2);
        assert_eq!(value["summary"]["attempts_failed"], 2);
        assert_eq!(value["summary"]["total_duration_ms"], 17.0);
    }

    #[test]
    fn finish_fails_for_either_failed_or_flaky_tests() {
        let mut failed = report();
        failed.summary.failed = 1;
        failed.finish(Duration::from_millis(23));
        assert!(failed.complete);
        assert_eq!(failed.outcome, RunOutcome::Fail);
        assert_eq!(failed.summary.total_duration_ms, 23.0);

        let mut flaky = report();
        flaky.summary.flaky = 1;
        flaky.finish(Duration::from_millis(29));
        assert!(flaky.complete);
        assert_eq!(flaky.outcome, RunOutcome::Fail);
        assert_eq!(flaky.summary.total_duration_ms, 29.0);

        let mut passing = report();
        passing.finish(Duration::from_millis(31));
        assert_eq!(passing.outcome, RunOutcome::Pass);
    }

    #[test]
    fn finish_error_records_the_exact_error() {
        let mut report = report();
        report.finish_error("startup", "Godot did not start".to_string());

        assert!(report.complete);
        assert_eq!(report.outcome, RunOutcome::Error);
        assert_eq!(report.errors.len(), 1);
        assert_eq!(report.errors[0].kind, "startup");
        assert_eq!(report.errors[0].message, "Godot did not start");
    }

    #[test]
    fn duration_and_run_id_encoding_are_exact() {
        assert_eq!(duration_ms(Duration::from_millis(1_250)), 1_250.0);

        let run_id = new_run_id();
        let (nanos, pid) = run_id.rsplit_once('-').unwrap();
        assert!(nanos.parse::<u128>().unwrap() > 0);
        assert_eq!(pid, std::process::id().to_string());
    }
}
