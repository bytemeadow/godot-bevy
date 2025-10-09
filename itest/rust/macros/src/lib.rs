use proc_macro::TokenStream;
use quote::quote;
use syn::{ItemFn, ReturnType, parse_macro_input};

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
