mod bevy_bundle;
mod component_as_godot_node;
mod node_tree_view;

use crate::component_as_godot_node::component_as_godot_node_impl;
use proc_macro::TokenStream;
use quote::quote;
use syn::{DeriveInput, Error, parse_macro_input};

/// Attribute macro that ensures a system runs on the main thread by adding a `NonSend<MainThreadMarker>` parameter.
/// This is required for systems that need to access Godot APIs.
#[proc_macro_attribute]
pub fn main_thread_system(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let mut input_fn = parse_macro_input!(item as syn::ItemFn);
    let fn_name = &input_fn.sig.ident;

    // Create a unique type alias name for this function
    let type_alias_name = syn::Ident::new(
        &format!("__MainThreadSystemMarker_{fn_name}"),
        fn_name.span(),
    );

    // Add a NonSend resource parameter that forces main thread execution
    let main_thread_param: syn::FnArg = syn::parse_quote! {
        _main_thread: bevy::ecs::system::NonSend<#type_alias_name>
    };
    input_fn.sig.inputs.push(main_thread_param);

    // Return the modified function with a unique type alias
    let expanded = quote! {
        #[allow(non_camel_case_types)]
        type #type_alias_name = godot_bevy::plugins::core::MainThreadMarker;

        #input_fn
    };

    expanded.into()
}

#[proc_macro_attribute]
pub fn bevy_app(_attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as syn::ItemFn);
    let name = &input_fn.sig.ident;
    let expanded = quote! {
        struct BevyExtensionLibrary;

        #[gdextension]
        unsafe impl ExtensionLibrary for BevyExtensionLibrary {
            fn on_level_init(level: godot::prelude::InitLevel) {
                if level == godot::prelude::InitLevel::Core {
                    godot::private::class_macros::registry::class::auto_register_classes(level);

                    // Stores the client's entrypoint, which we'll call shortly when our `BevyApp`
                    // Godot Node has its `ready()` invoked
                    let _ = godot_bevy::app::BEVY_INIT_FUNC.get_or_init(|| Box::new(#name));

                    #[cfg(feature = "trace_tracy")]
                    // Start Tracy manually (manual‑lifetime feature enabled)
                    let _ = &godot_bevy::utils::TRACY_CLIENT;
                }
            }


            fn on_level_deinit(_level: godot::prelude::InitLevel) {
                #[cfg(feature = "trace_tracy")]
                if _level == godot::prelude::InitLevel::Core {
                    // Explicitly shut Tracy down; required with `manual-lifetime`.
                    unsafe {
                        tracing_tracy::client::sys::___tracy_shutdown_profiler();
                    }
                    // TRACY_CLIENT stays filled, but the library is about to be unloaded,
                    // so its memory will disappear immediately afterwards.
                }
            }
        }

        #input_fn
    };

    expanded.into()
}

#[proc_macro_derive(NodeTreeView, attributes(node))]
pub fn derive_node_tree_view(item: TokenStream) -> TokenStream {
    let view = parse_macro_input!(item as DeriveInput);

    let expanded = node_tree_view::node_tree_view(view).unwrap_or_else(Error::into_compile_error);

    TokenStream::from(expanded)
}

#[proc_macro_derive(BevyBundle, attributes(bevy_bundle))]
pub fn derive_bevy_bundle(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    let expanded = bevy_bundle::bevy_bundle(input).unwrap_or_else(Error::into_compile_error);

    TokenStream::from(expanded)
}

/// Automatically registers a Godot node based on the annotated Component struct.
/// This macro has two parts:
/// - Struct level `godot_node` attribute
/// - Field level `godot_export` attribute.
///
/// ---
///
/// A struct level attribute can be used to specify the Godot class to extend, and the class name:
///
/// ```ignore
/// #[godot_node(base(<godot_node_type>), class_name(<custom_class_name>))]
/// ```
///
/// - `base` (Default: `Node`) Godot node to extend.
/// - `class_name` (Default: `<struct_name>BevyComponent`) Name of generated Godot class.
///
/// ---
///
/// Fields can be exposed to Godot as node properties using the `#[godot_export]` attribute.
/// The attribute syntax is:
///
/// ```ignore
/// #[godot_export(export_type(<godot_type>), transform_with(<conversion_function>), default(<value>))]
/// ```
///
/// For fields with types incompatible with Godot-Rust's `#[export]` macro:
/// - Use `export_type` to specify an alternate Godot-compatible type
/// - Use `transform_with` to provide a conversion function from the Godot type to the field type
/// - Use `default` to provide an initial value to the exported Godot field.
///
/// ---
///
/// Uses the `inventory` crate
#[proc_macro_derive(GodotNode, attributes(godot_export, godot_node))]
pub fn component_as_godot_node(input: TokenStream) -> TokenStream {
    component_as_godot_node_impl(input.into())
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
