use proc_macro2::TokenStream as TokenStream2;
use quote::ToTokens;
use syn::{DeriveInput, Meta};

mod attr;
mod bundle;
mod component;
mod components_attr;

pub fn derive_godot_node(input: DeriveInput) -> syn::Result<TokenStream2> {
    // Prefer explicit derives when available
    let mut derives_bundle = false;
    let mut derives_component = false;
    for attr in &input.attrs {
        if attr.path().is_ident("derive")
            && let Meta::List(list) = &attr.meta
        {
            // The tokens are a comma-separated list of paths: e.g. (Bundle, Component)
            let tokens = list.tokens.clone().into_iter();
            for tt in tokens {
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

    // Fallback: detect bundle mode by presence of any #[export_fields]
    let has_export_fields = match &input.data {
        syn::Data::Struct(data) => data
            .fields
            .iter()
            .flat_map(|f| f.attrs.iter())
            .any(|a| a.path().is_ident("export_fields")),
        _ => false,
    };

    let is_bundle_mode = derives_bundle || (!derives_component && has_export_fields);

    let has_godot_components = input
        .attrs
        .iter()
        .any(|a| a.path().is_ident("godot_components"));

    if is_bundle_mode && has_godot_components {
        return Err(syn::Error::new_spanned(
            &input,
            "`#[godot_components]` requires deriving GodotNode on a Component, not a Bundle. \
             Bundle mode is deprecated; see the `#[godot_components]` documentation for the migration.",
        ));
    }

    if is_bundle_mode {
        bundle::godot_node_bundle_impl(input)
    } else {
        // Component flow expects TokenStream2 of DeriveInput
        component::component_as_godot_node_impl(input.to_token_stream())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn godot_components_on_bundle_mode_is_error() {
        let input: DeriveInput = parse_quote! {
            #[derive(Bundle, GodotNode)]
            #[godot_node(base(Node2D), class_name(PlayerNode))]
            #[godot_components(speed(Speed, export_type(f32)))]
            struct PlayerBundle {
                #[export_fields(value(export_type(f32)))]
                speed: Speed,
            }
        };

        let err = derive_godot_node(input).unwrap_err();
        assert!(
            err.to_string()
                .contains("`#[godot_components]` requires deriving GodotNode on a Component")
        );
    }
}
