use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use syn::DeriveInput;

mod bundle;
mod component;

pub fn derive_godot_node(input: DeriveInput) -> syn::Result<TokenStream2> {
    let is_bundle_style = match &input.data {
        syn::Data::Struct(data) => data
            .fields
            .iter()
            .flat_map(|f| f.attrs.iter())
            .any(|a| a.path().is_ident("godot_props")),
        _ => false,
    };

    if is_bundle_style {
        bundle::godot_node_bundle_impl(input)
    } else {
        // Existing component flow expects TokenStream2 of DeriveInput
        component::component_as_godot_node_impl(input.to_token_stream())
    }
}


