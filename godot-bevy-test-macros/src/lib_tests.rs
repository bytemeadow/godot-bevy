#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_exact_itest_options() {
        let options = syn::parse_str::<ITestOptions>("async, skip, focus").unwrap();
        assert!(options.is_async);
        assert!(options.is_skipped);
        assert!(options.is_focused);
    }

    #[test]
    fn rejects_unknown_and_duplicate_itest_options() {
        assert!(syn::parse_str::<ITestOptions>("asyncish").is_err());
        assert!(syn::parse_str::<ITestOptions>("focus, focus").is_err());
    }
}
