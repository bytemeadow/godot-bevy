use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use syn::{DeriveInput, Meta};

mod bundle;
mod component;

pub fn derive_godot_node(input: DeriveInput) -> syn::Result<TokenStream2> {
    // Prefer explicit derives when available
    let mut derives_bundle = false;
    let mut derives_component = false;
    for attr in &input.attrs {
        if attr.path().is_ident("derive") {
            if let Meta::List(list) = &attr.meta {
                // The tokens are a comma-separated list of paths: e.g. (Bundle, Component)
                let mut tokens = list.tokens.clone().into_iter();
                while let Some(tt) = tokens.next() {
                    if let proc_macro2::TokenTree::Ident(ident) = tt {
                        if ident == "Bundle" {
                            derives_bundle = true;
                        }
                        if ident == "Component" {
                            derives_component = true;
                        }
                    }
                }
            }
        }
    }

    // Fallback: detect bundle mode by presence of any #[godot_props]
    let has_godot_props = match &input.data {
        syn::Data::Struct(data) => data
            .fields
            .iter()
            .flat_map(|f| f.attrs.iter())
            .any(|a| a.path().is_ident("godot_props")),
        _ => false,
    };

    if derives_bundle || (!derives_component && has_godot_props) {
        bundle::godot_node_bundle_impl(input)
    } else {
        // Component flow expects TokenStream2 of DeriveInput
        component::component_as_godot_node_impl(input.to_token_stream())
    }
}
