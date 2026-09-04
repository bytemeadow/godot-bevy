use proc_macro::TokenStream;
use quote::quote;
use syn::ext::IdentExt;
use syn::parse::{Parse, ParseStream};
use syn::{
    FnArg, Ident, ItemFn, Lit, Meta, MetaNameValue, ReturnType, Token, Type, parse_macro_input,
};

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

/// Attribute macro for integration tests.
///
/// Usage:
/// <!-- qualification-doctest: scaffold=book-tests/src/doctest_scaffolds.rs#itest -->
/// ```ignore
/// #[itest]
/// async fn my_test(ctx: TestContext) {
///     let mut app = TestApp::new(&ctx, |_| {}).await;
///     app.update().await;
/// }
///
/// #[itest(async)]
/// fn my_async_test(ctx: &TestContext) -> godot::task::TaskHandle {
///     godot::task::spawn(async move {
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
///
/// Async functions take `TestContext` by value. References cannot outlive the
/// wrapper that starts the task:
/// ```compile_fail
/// use godot_bevy_test_macros::itest;
///
/// struct TestContext;
///
/// #[itest]
/// async fn borrowed_context(_ctx: &TestContext) {}
/// ```
#[proc_macro_attribute]
pub fn itest(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as ItemFn);
    let options = parse_macro_input!(attr as ITestOptions);
    expand_itest(input, options).into()
}

fn expand_itest(input: ItemFn, options: ITestOptions) -> proc_macro2::TokenStream {
    let is_async = options.is_async;
    let is_skipped = options.is_skipped;
    let is_focused = options.is_focused;

    let test_name = &input.sig.ident;
    let test_name_str = test_name.to_string();
    let visibility = &input.vis;
    let body = &input.block;

    // Absolute paths keep the expansion independent of caller imports.
    let param = if let Some(param) = input.sig.inputs.first() {
        quote! { #param }
    } else {
        quote! { _ctx: &::godot_bevy_test::TestContext }
    };

    if input.sig.asyncness.is_some() {
        if !matches!(input.sig.output, ReturnType::Default) {
            return syn::Error::new_spanned(
                &input.sig.output,
                "async #[itest] functions must return ()",
            )
            .into_compile_error();
        }

        if input.sig.inputs.len() > 1 {
            return syn::Error::new_spanned(
                &input.sig.inputs,
                "async #[itest] functions accept at most one TestContext parameter",
            )
            .into_compile_error();
        }

        let (wrapper_param, context) = match input.sig.inputs.first() {
            Some(FnArg::Typed(param)) => {
                if matches!(&*param.ty, Type::Reference(_)) {
                    return syn::Error::new_spanned(
                        &param.ty,
                        "async #[itest] functions must take TestContext by value",
                    )
                    .into_compile_error();
                }
                let pattern = &param.pat;
                (
                    quote! { ctx: &::godot_bevy_test::TestContext },
                    quote! { let #pattern = ctx.clone(); },
                )
            }
            Some(FnArg::Receiver(receiver)) => {
                return syn::Error::new_spanned(
                    receiver,
                    "async #[itest] functions accept TestContext by value",
                )
                .into_compile_error();
            }
            None => (quote! { _ctx: &::godot_bevy_test::TestContext }, quote! {}),
        };

        quote! {
            #visibility fn #test_name(#wrapper_param) -> ::godot::task::TaskHandle {
                #context
                ::godot::task::spawn(async move #body)
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
        }
    } else if is_async {
        let return_ty = match &input.sig.output {
            ReturnType::Type(_, ty) => quote! { -> #ty },
            ReturnType::Default => quote! { -> ::godot::task::TaskHandle },
        };

        quote! {
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
        }
    } else {
        quote! {
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
        }
    }
}

/// Attribute macro for benchmarks
///
/// Usage:
/// <!-- qualification-doctest: scaffold=book-tests/src/doctest_scaffolds.rs#bench -->
/// ```ignore
/// #[bench]
/// fn my_benchmark() -> ReturnType {
/// }
///
/// #[bench(repeat = 25)]
/// fn expensive_benchmark() -> ReturnType {
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
