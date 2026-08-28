use std::env::VarError;
use std::path::PathBuf;

pub(crate) const DEFAULT_REPEAT: u32 = 1;
pub(crate) const DEFAULT_TIMEOUT_FRAMES: u32 = 600;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct Filter {
    pub(crate) normalized: String,
    pub(crate) patterns: Vec<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct TestConfig {
    pub(crate) filter: Option<Filter>,
    pub(crate) repeat: u32,
    pub(crate) timeout_frames: u32,
    pub(crate) json_path: Option<PathBuf>,
    pub(crate) deny_focus: bool,
    pub(crate) build_profile: String,
}

impl TestConfig {
    pub(crate) fn from_env() -> Result<Self, String> {
        Ok(Self {
            filter: filter_from_env("ITEST_FILTER")?,
            repeat: positive_u32_from_env("ITEST_REPEAT", DEFAULT_REPEAT)?,
            timeout_frames: positive_u32_from_env("ITEST_TIMEOUT_FRAMES", DEFAULT_TIMEOUT_FRAMES)?,
            json_path: report_path_from_env()?,
            deny_focus: boolean_from_env("ITEST_DENY_FOCUS", false)?,
            build_profile: build_profile_from_env()?,
        })
    }

    pub(crate) fn fallback() -> Self {
        Self {
            filter: None,
            repeat: DEFAULT_REPEAT,
            timeout_frames: DEFAULT_TIMEOUT_FRAMES,
            json_path: None,
            deny_focus: false,
            build_profile: default_build_profile().to_string(),
        }
    }
}

pub(crate) fn filter_from_env(name: &str) -> Result<Option<Filter>, String> {
    match std::env::var(name) {
        Ok(value) => parse_filter(name, &value).map(Some),
        Err(VarError::NotPresent) => Ok(None),
        Err(VarError::NotUnicode(_)) => Err(format!("{name} must be valid Unicode")),
    }
}

pub(crate) fn report_path_from_env() -> Result<Option<PathBuf>, String> {
    let path = match std::env::var("ITEST_JSON_PATH") {
        Ok(value) if value.trim().is_empty() => {
            return Err("ITEST_JSON_PATH must not be empty".to_string());
        }
        Ok(value) => PathBuf::from(value),
        Err(VarError::NotPresent) => return Ok(None),
        Err(VarError::NotUnicode(_)) => {
            return Err("ITEST_JSON_PATH must be valid Unicode".to_string());
        }
    };

    if path.is_absolute() {
        Ok(Some(path))
    } else {
        std::env::current_dir()
            .map(|current_dir| Some(current_dir.join(path)))
            .map_err(|error| format!("failed to resolve ITEST_JSON_PATH: {error}"))
    }
}

fn parse_filter(name: &str, value: &str) -> Result<Filter, String> {
    let patterns: Vec<String> = value
        .split(',')
        .map(str::trim)
        .filter(|pattern| !pattern.is_empty())
        .map(str::to_string)
        .collect();

    if patterns.is_empty() {
        return Err(format!("{name} must contain at least one nonempty pattern"));
    }

    Ok(Filter {
        normalized: patterns.join(","),
        patterns,
    })
}

fn positive_u32_from_env(name: &str, default: u32) -> Result<u32, String> {
    match std::env::var(name) {
        Ok(value) => parse_positive_u32(name, &value),
        Err(VarError::NotPresent) => Ok(default),
        Err(VarError::NotUnicode(_)) => Err(format!("{name} must be valid Unicode")),
    }
}

fn parse_positive_u32(name: &str, value: &str) -> Result<u32, String> {
    if value.is_empty() || !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err(format!("{name} must be a positive integer"));
    }
    let parsed = value
        .parse::<u32>()
        .map_err(|_| format!("{name} must be a positive integer"))?;
    if parsed == 0 {
        return Err(format!("{name} must be a positive integer"));
    }
    Ok(parsed)
}

fn boolean_from_env(name: &str, default: bool) -> Result<bool, String> {
    match std::env::var(name) {
        Ok(value) => parse_boolean(name, &value),
        Err(VarError::NotPresent) => Ok(default),
        Err(VarError::NotUnicode(_)) => Err(format!("{name} must be valid Unicode")),
    }
}

fn parse_boolean(name: &str, value: &str) -> Result<bool, String> {
    match value {
        "1" | "true" => Ok(true),
        "0" | "false" => Ok(false),
        _ => Err(format!("{name} must be one of 0, 1, false, or true")),
    }
}

fn build_profile_from_env() -> Result<String, String> {
    match std::env::var("ITEST_BUILD_PROFILE") {
        Ok(value) if matches!(value.as_str(), "debug" | "release") => Ok(value),
        Ok(_) => Err("ITEST_BUILD_PROFILE must be debug or release".to_string()),
        Err(VarError::NotPresent) => Ok(default_build_profile().to_string()),
        Err(VarError::NotUnicode(_)) => {
            Err("ITEST_BUILD_PROFILE must be valid Unicode".to_string())
        }
    }
}

fn default_build_profile() -> &'static str {
    if cfg!(debug_assertions) {
        "debug"
    } else {
        "release"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
