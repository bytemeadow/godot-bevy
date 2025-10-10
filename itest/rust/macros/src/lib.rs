use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, Lit, Meta, MetaNameValue, ReturnType, parse_macro_input};

/// Attribute macro for integration tests
///
/// Usage:
/// ```
/// #[itest]
/// fn my_sync_test(ctx: &TestContext) {
///     // test code
/// }
///
/// #[itest(async)]
/// fn my_async_test(ctx: &TestContext) -> godot::task::TaskHandle {
///     godot::task::spawn(async move {
///         // async test code
///     })
/// }
/// ```
#[proc_macro_attribute]
pub fn itest(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let attr_str = attr.to_string();
    let is_async = attr_str.contains("async");
    let is_skipped = attr_str.contains("skip");

    let test_name = &input.sig.ident;
    let test_name_str = test_name.to_string();
    let visibility = &input.vis;
    let body = &input.block;

    // Extract parameter or use default
    let param = if let Some(param) = input.sig.inputs.first() {
        quote! { #param }
    } else {
        quote! { _ctx: &crate::framework::TestContext }
    };

    if is_async {
        // Async test - returns TaskHandle
        let return_ty = match &input.sig.output {
            ReturnType::Type(_, ty) => quote! { -> #ty },
            ReturnType::Default => quote! { -> godot::task::TaskHandle },
        };

        TokenStream::from(quote! {
            #visibility fn #test_name(#param) #return_ty {
                #body
            }

            ::godot::sys::plugin_add!(
                crate::framework::__GODOT_ASYNC_ITEST;
                crate::framework::AsyncRustTestCase {
                    name: #test_name_str,
                    file: file!(),
                    skipped: #is_skipped,
                    focused: false,
                    line: line!(),
                    function: #test_name,
                }
            );
        })
    } else {
        // Sync test
        TokenStream::from(quote! {
            #visibility fn #test_name(#param) {
                #body
            }

            ::godot::sys::plugin_add!(
                crate::framework::__GODOT_ITEST;
                crate::framework::RustTestCase {
                    name: #test_name_str,
                    file: file!(),
                    skipped: false,
                    focused: false,
                    line: line!(),
                    function: #test_name,
                }
            );
        })
    }
}

/// Attribute macro for benchmarks
///
/// Usage:
/// ```
/// #[bench]
/// fn my_benchmark() -> ReturnType {
///     // benchmark code - must return a value
/// }
///
/// #[bench(repeat = 25)]
/// fn expensive_benchmark() -> ReturnType {
///     // custom repetition count
/// }
/// ```
#[proc_macro_attribute]
pub fn bench(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let bench_name = &input.sig.ident;
    let bench_name_str = bench_name.to_string();
    let visibility = &input.vis;
    let body = &input.block;

    let default_repetitions = 100;
    let mut repetitions = default_repetitions;

    if !attr.is_empty() {
        let attr_meta = parse_macro_input!(attr as Meta);
        if let Meta::NameValue(MetaNameValue { path, value, .. }) = attr_meta {
            if path.is_ident("repeat") {
                if let syn::Expr::Lit(expr_lit) = value {
                    if let Lit::Int(lit_int) = &expr_lit.lit {
                        repetitions = lit_int.base10_parse().unwrap_or(default_repetitions);
                    }
                }
            }
        }
    }

    let ret_ty = match &input.sig.output {
        ReturnType::Type(_, ty) => ty,
        ReturnType::Default => {
            return TokenStream::from(quote! {
                compile_error!("#[bench] function must return a value to prevent optimization");
            });
        }
    };

    let reps_literal = syn::Index::from(repetitions);

    TokenStream::from(quote! {
        #visibility fn #bench_name() {
            for _ in 0..#reps_literal {
                let __ret: #ret_ty = #body;
                std::hint::black_box(__ret);
            }
        }

        ::godot::sys::plugin_add!(
            crate::framework::__GODOT_BENCH;
            crate::framework::RustBenchmark {
                name: #bench_name_str,
                file: file!(),
                line: line!(),
                function: #bench_name,
                repetitions: #reps_literal,
            }
        );
    })
}
