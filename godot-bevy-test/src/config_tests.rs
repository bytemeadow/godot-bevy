#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    const ENVIRONMENT_KEYS: &[&str] = &[
        "GBV_FILTER",
        "GBPROF_NATIVE_SECONDS",
        "GBV_REQUIRED",
        "GBV_OPTIONAL",
        "ITEST_JSON_PATH",
        "GBV_POSITIVE",
        "GBV_BOOLEAN_FALSE",
        "GBV_BOOLEAN_TRUE",
        "ITEST_BUILD_PROFILE",
    ];

    fn run_environment_probe(case: &str, values: &[(&str, &str)]) {
        let mut command = Command::new(std::env::current_exe().unwrap());
        command.arg("config_environment_probe").arg("--nocapture");
        command.env("GBV_CONFIG_PROBE", case);
        for key in ENVIRONMENT_KEYS {
            command.env_remove(key);
        }
        for (key, value) in values {
            command.env(key, value);
        }
        assert!(command.status().unwrap().success(), "probe case {case}");
    }

    #[test]
    fn filter_tokens_are_trimmed_and_empty_tokens_are_dropped() {
        assert_eq!(
            parse_filter("FILTER", " alpha, ,beta, ").unwrap(),
            Filter {
                normalized: "alpha,beta".to_string(),
                patterns: vec!["alpha".to_string(), "beta".to_string()],
            }
        );
        assert!(parse_filter("FILTER", " , ").is_err());
    }

    #[test]
    fn numeric_and_boolean_values_are_strict() {
        assert_eq!(parse_positive_u32("COUNT", "1").unwrap(), 1);
        assert!(parse_positive_u32("COUNT", "0").is_err());
        assert!(parse_positive_u32("COUNT", "-1").is_err());
        assert!(parse_positive_u32("COUNT", "4294967296").is_err());
        assert!(parse_boolean("BOOLEAN", "1").unwrap());
        assert!(!parse_boolean("BOOLEAN", "false").unwrap());
        assert!(parse_boolean("BOOLEAN", "yes").is_err());
    }

    #[test]
    fn benchmark_selectors_match_exactly_or_by_substring() {
        let exact = BenchmarkSelector::Exact("alpha".to_string());
        assert!(exact.matches("alpha"));
        assert!(!exact.matches("alpha_2"));

        let filter = BenchmarkSelector::Filter(parse_filter("FILTER", "alpha,beta").unwrap());
        assert!(filter.matches("prefix_alpha_suffix"));
        assert!(filter.matches("beta_2"));
        assert!(!filter.matches("gamma"));
    }

    #[test]
    fn benchmark_selector_all_detection_is_exact() {
        assert!(BenchmarkSelector::All.is_all());
        assert!(!BenchmarkSelector::Exact("alpha".to_string()).is_all());
        assert!(!BenchmarkSelector::Filter(parse_filter("FILTER", "alpha").unwrap()).is_all());
    }

    #[test]
    fn environment_helpers_parse_valid_missing_invalid_and_boundary_values() {
        run_environment_probe(
            "valid",
            &[
                ("GBV_FILTER", " alpha, beta "),
                ("GBPROF_NATIVE_SECONDS", "7"),
                ("GBV_REQUIRED", " required "),
                ("GBV_OPTIONAL", " optional "),
                ("ITEST_JSON_PATH", "gbv-report.json"),
                ("GBV_POSITIVE", "37"),
                ("GBV_BOOLEAN_FALSE", "false"),
                ("GBV_BOOLEAN_TRUE", "1"),
                ("ITEST_BUILD_PROFILE", "release"),
            ],
        );
        run_environment_probe("missing", &[]);
        run_environment_probe(
            "invalid",
            &[
                ("GBV_FILTER", " , "),
                ("GBPROF_NATIVE_SECONDS", "4"),
                ("GBV_REQUIRED", " "),
                ("GBV_OPTIONAL", " "),
                ("ITEST_JSON_PATH", " "),
                ("GBV_POSITIVE", "-1"),
                ("GBV_BOOLEAN_FALSE", "yes"),
                ("ITEST_BUILD_PROFILE", "other"),
            ],
        );
        run_environment_probe("boundary", &[("GBPROF_NATIVE_SECONDS", "5")]);
    }

    #[test]
    fn config_environment_probe() {
        let Ok(case) = std::env::var("GBV_CONFIG_PROBE") else {
            return;
        };
        match case.as_str() {
            "valid" => {
                assert_eq!(
                    filter_from_env("GBV_FILTER").unwrap(),
                    Some(Filter {
                        normalized: "alpha,beta".to_string(),
                        patterns: vec!["alpha".to_string(), "beta".to_string()],
                    })
                );
                assert_eq!(native_profile_seconds_from_env().unwrap(), Some(7));
                assert_eq!(
                    required_nonempty_from_env("GBV_REQUIRED").unwrap(),
                    "required"
                );
                assert_eq!(
                    optional_nonempty_from_env("GBV_OPTIONAL").unwrap(),
                    Some("optional".to_string())
                );
                assert_eq!(
                    report_path_from_env().unwrap(),
                    Some(std::env::current_dir().unwrap().join("gbv-report.json"))
                );
                assert_eq!(positive_u32_from_env("GBV_POSITIVE", 23).unwrap(), 37);
                assert!(!boolean_from_env("GBV_BOOLEAN_FALSE", true).unwrap());
                assert!(boolean_from_env("GBV_BOOLEAN_TRUE", false).unwrap());
                assert_eq!(build_profile_from_env().unwrap(), "release");
            }
            "missing" => {
                assert_eq!(filter_from_env("GBV_FILTER").unwrap(), None);
                assert_eq!(native_profile_seconds_from_env().unwrap(), None);
                assert!(required_nonempty_from_env("GBV_REQUIRED").is_err());
                assert_eq!(optional_nonempty_from_env("GBV_OPTIONAL").unwrap(), None);
                assert_eq!(report_path_from_env().unwrap(), None);
                assert_eq!(positive_u32_from_env("GBV_POSITIVE", 23).unwrap(), 23);
                assert!(boolean_from_env("GBV_BOOLEAN_FALSE", true).unwrap());
                let expected = if cfg!(debug_assertions) {
                    "debug"
                } else {
                    "release"
                };
                assert_eq!(default_build_profile(), expected);
                assert_eq!(build_profile_from_env().unwrap(), expected);
            }
            "invalid" => {
                assert!(filter_from_env("GBV_FILTER").is_err());
                assert!(native_profile_seconds_from_env().is_err());
                assert!(required_nonempty_from_env("GBV_REQUIRED").is_err());
                assert!(optional_nonempty_from_env("GBV_OPTIONAL").is_err());
                assert!(report_path_from_env().is_err());
                assert!(positive_u32_from_env("GBV_POSITIVE", 23).is_err());
                assert!(boolean_from_env("GBV_BOOLEAN_FALSE", true).is_err());
                assert!(build_profile_from_env().is_err());
            }
            "boundary" => {
                assert_eq!(native_profile_seconds_from_env().unwrap(), Some(5));
            }
            other => panic!("unknown probe case {other}"),
        }
    }
}
