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
