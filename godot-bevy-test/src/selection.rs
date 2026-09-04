use std::collections::HashSet;

use crate::TestContext;
use crate::config::Filter;

#[derive(Copy, Clone)]
pub(crate) enum TestFunction {
    Sync(fn(&TestContext)),
    Async(fn(&TestContext) -> godot::task::TaskHandle),
}

#[derive(Copy, Clone)]
pub(crate) struct RegisteredTest {
    pub(crate) name: &'static str,
    pub(crate) file: &'static str,
    pub(crate) skipped: bool,
    pub(crate) focused: bool,
    pub(crate) line: u32,
    pub(crate) function: TestFunction,
}

pub(crate) struct CollectedTests {
    pub(crate) tests: Vec<RegisteredTest>,
    pub(crate) registered: usize,
    pub(crate) file_count: usize,
    pub(crate) focus_run: bool,
}

pub(crate) fn select_registered_tests(
    mut tests: Vec<RegisteredTest>,
    filter: Option<&Filter>,
) -> CollectedTests {
    let registered = tests.len();
    let focus_run = tests.iter().any(|test| test.focused);

    tests.retain(|test| {
        (!focus_run || test.focused)
            && filter.is_none_or(|filter| {
                filter
                    .patterns
                    .iter()
                    .any(|pattern| test.name.contains(pattern))
            })
    });
    tests.sort_by_key(|test| (test.file, test.line));

    let file_count = tests
        .iter()
        .map(|test| test.file)
        .collect::<HashSet<_>>()
        .len();

    CollectedTests {
        tests,
        registered,
        file_count,
        focus_run,
    }
}

#[cfg(test)]
include!("selection_tests.rs");
