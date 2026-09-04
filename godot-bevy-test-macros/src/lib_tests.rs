#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;

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

    #[test]
    fn async_function_registers_an_async_test_case() {
        let input = syn::parse2(quote! {
            async fn test_name(ctx: TestContext) {
                let _ = ctx;
            }
        })
        .unwrap();
        let expanded = expand_itest(input, ITestOptions::default());

        assert!(expanded.to_string().contains("AsyncRustTestCase"));
    }

    #[test]
    fn async_function_rejects_borrowed_context() {
        let input = syn::parse2(quote! { async fn test_name(ctx: &TestContext) {} }).unwrap();
        let expanded = expand_itest(input, ITestOptions::default());

        assert!(expanded
            .to_string()
            .contains("must take TestContext by value"));
    }

    #[test]
    fn async_function_rejects_non_unit_return() {
        let input = syn::parse2(quote! { async fn test_name() -> bool { true } }).unwrap();
        let expanded = expand_itest(input, ITestOptions::default());

        assert!(expanded
            .to_string()
            .contains("async #[itest] functions must return ()"));
    }
}
