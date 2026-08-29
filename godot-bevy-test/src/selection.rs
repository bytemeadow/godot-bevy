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
mod tests {
    use super::*;

    fn sync_test(_: &TestContext) {}

    fn async_test(_: &TestContext) -> godot::task::TaskHandle {
        unreachable!()
    }

    fn registered(
        name: &'static str,
        file: &'static str,
        line: u32,
        focused: bool,
        skipped: bool,
        function: TestFunction,
    ) -> RegisteredTest {
        RegisteredTest {
            name,
            file,
            skipped,
            focused,
            line,
            function,
        }
    }

    #[test]
    fn selection_contract() {
        let mixed = select_registered_tests(
            vec![
                registered(
                    "async_second",
                    "src/b.rs",
                    10,
                    false,
                    false,
                    TestFunction::Async(async_test),
                ),
                registered(
                    "sync_first",
                    "src/a.rs",
                    20,
                    false,
                    false,
                    TestFunction::Sync(sync_test),
                ),
            ],
            None,
        );
        assert_eq!(
            mixed.tests.iter().map(|test| test.name).collect::<Vec<_>>(),
            ["sync_first", "async_second"]
        );

        let all = vec![
            registered(
                "normal_sync",
                "src/z.rs",
                4,
                false,
                false,
                TestFunction::Sync(sync_test),
            ),
            registered(
                "focused_async_first",
                "src/a.rs",
                20,
                true,
                false,
                TestFunction::Async(async_test),
            ),
            registered(
                "focused_async_skipped",
                "src/a.rs",
                30,
                true,
                true,
                TestFunction::Async(async_test),
            ),
        ];
        let filter = Filter {
            normalized: "focused_async".to_string(),
            patterns: vec!["focused_async".to_string()],
        };
        let selected = select_registered_tests(all, Some(&filter));

        assert_eq!(selected.registered, 3);
        assert!(selected.focus_run);
        assert_eq!(selected.tests.len(), 2);
        assert_eq!(selected.tests[0].name, "focused_async_first");
        assert_eq!(selected.tests[1].name, "focused_async_skipped");
        assert!(selected.tests[1].skipped);

        let no_match = Filter {
            normalized: "FOCUSED".to_string(),
            patterns: vec!["FOCUSED".to_string()],
        };
        let selected = select_registered_tests(
            vec![registered(
                "focused_async_first",
                "src/a.rs",
                20,
                true,
                false,
                TestFunction::Async(async_test),
            )],
            Some(&no_match),
        );
        assert!(selected.tests.is_empty());
        assert!(selected.focus_run);
    }
}
