use proc_macro::TokenStream;
use quote::quote;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{Ident, ItemFn, Lit, Meta, MetaNameValue, ReturnType, Token, parse_macro_input};

#[derive(Default)]
struct ITestOptions {
    is_async: bool,
    is_skipped: bool,
    is_focused: bool,
}

impl Parse for ITestOptions {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut options = Self::default();

        while !input.is_empty() {
            let option = input.call(Ident::parse_any)?;
            let (name, selected) = match option.to_string().as_str() {
                "async" => ("async", &mut options.is_async),
                "skip" => ("skip", &mut options.is_skipped),
                "focus" => ("focus", &mut options.is_focused),
                _ => {
                    return Err(syn::Error::new(
                        option.span(),
                        "unknown #[itest] option; expected async, skip, or focus",
                    ));
                }
            };
            if *selected {
                return Err(syn::Error::new(
                    option.span(),
                    format!("duplicate #[itest] option `{name}`"),
                ));
            }
            *selected = true;

            if !input.is_empty() {
                input.parse::<Token![,]>()?;
            }
        }

        Ok(options)
    }
}

/// Attribute macro for integration tests
///
/// Usage:
/// <!-- qualification-doctest: scaffold=book-tests/src/doctest_scaffolds.rs#itest -->
/// ```ignore
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
///
/// #[itest(skip)]
/// fn skipped_test(ctx: &TestContext) {
///     // this test will be skipped
/// }
///
/// #[itest(focus)]
/// fn focused_test(ctx: &TestContext) {
///     // only focused tests will run when any test has focus
/// }
/// ```
///
/// Unknown options are rejected at compile time:
/// ```compile_fail
/// use godot_bevy_test_macros::itest;
///
/// #[itest(foucs)]
/// fn misspelled_option() {}
/// ```
#[proc_macro_attribute]
pub fn itest(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let options = parse_macro_input!(attr as ITestOptions);
    let is_async = options.is_async;
    let is_skipped = options.is_skipped;
    let is_focused = options.is_focused;

    let test_name = &input.sig.ident;
    let test_name_str = test_name.to_string();
    let visibility = &input.vis;
    let body = &input.block;

    // Extract parameter or use default - use absolute path to godot_bevy_test
    let param = if let Some(param) = input.sig.inputs.first() {
        quote! { #param }
    } else {
        quote! { _ctx: &::godot_bevy_test::TestContext }
    };

    if is_async {
        // Async test - returns TaskHandle
        let return_ty = match &input.sig.output {
            ReturnType::Type(_, ty) => quote! { -> #ty },
            ReturnType::Default => quote! { -> ::godot::task::TaskHandle },
        };

        TokenStream::from(quote! {
            #visibility fn #test_name(#param) #return_ty {
                #body
            }

            ::godot::sys::shard_add!(
                ::godot_bevy_test::__GODOT_ASYNC_ITEST;
                ::godot_bevy_test::AsyncRustTestCase {
                    name: #test_name_str,
                    file: file!(),
                    skipped: #is_skipped,
                    focused: #is_focused,
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

            ::godot::sys::shard_add!(
                ::godot_bevy_test::__GODOT_ITEST;
                ::godot_bevy_test::RustTestCase {
                    name: #test_name_str,
                    file: file!(),
                    skipped: #is_skipped,
                    focused: #is_focused,
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
/// <!-- qualification-doctest: scaffold=book-tests/src/doctest_scaffolds.rs#bench -->
/// ```ignore
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
        if let Meta::NameValue(MetaNameValue { path, value, .. }) = attr_meta
            && path.is_ident("repeat")
            && let syn::Expr::Lit(expr_lit) = value
            && let Lit::Int(lit_int) = &expr_lit.lit
        {
            repetitions = lit_int.base10_parse().unwrap_or(default_repetitions);
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
                ::std::hint::black_box(__ret);
            }
        }

        ::godot::sys::shard_add!(
            ::godot_bevy_test::__GODOT_BENCH;
            ::godot_bevy_test::RustBenchmark {
                name: #bench_name_str,
                file: file!(),
                line: line!(),
                function: #bench_name,
                repetitions: #reps_literal,
            }
        );
    })
}

#[cfg(test)]
include!("lib_tests.rs");
