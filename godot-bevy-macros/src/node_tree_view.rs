use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Error, Field, Fields};

pub fn node_tree_view(input: DeriveInput) -> syn::Result<TokenStream2> {
    let item = &input.ident;
    let data_struct = match &input.data {
        Data::Struct(data_struct) => data_struct,
        _ => {
            return Err(Error::new_spanned(
                input,
                "NodeTreeView must be used on structs",
            ));
        }
    };

    if matches!(data_struct.fields, Fields::Unit) {
        return Err(Error::new_spanned(
            input,
            "NodeTreeView must be used on structs with fields",
        ));
    }

    let mut field_errors = vec![];
    let field_exprs = data_struct
        .fields
        .iter()
        .map(|field| match create_get_node_expr(field) {
            Ok(expr) => {
                if let Some(name) = &field.ident {
                    quote! { #name : #expr, }
                } else {
                    quote! { #expr, }
                }
            }
            Err(e) => {
                field_errors.push(e);
                TokenStream2::new()
            }
        })
        .collect::<TokenStream2>();

    if !field_errors.is_empty() {
        let mut error = field_errors[0].clone();
        error.extend(field_errors[1..].iter().cloned());

        return Err(error);
    }

    let self_expr = if matches!(data_struct.fields, Fields::Named(_)) {
        quote! { Self { #field_exprs } }
    } else {
        quote! { Self ( #field_exprs ) }
    };

    let node_tree_view = quote! { godot_bevy::prelude::NodeTreeView };
    let inherits = quote! { godot::obj::Inherits };
    let node = quote! { godot::classes::Node };
    let gd = quote! { godot::obj::Gd };

    let expanded = quote! {
       impl #node_tree_view for #item {
           fn from_node<T: #inherits<#node>>(node: #gd<T>) -> Self {
               let node = node.upcast::<#node>();
               #self_expr
           }
       }
    };

    Ok(expanded)
}

fn create_get_node_expr(field: &Field) -> syn::Result<TokenStream2> {
    let node_path: syn::LitStr = field
        .attrs
        .iter()
        .find_map(|attr| {
            if attr.path().is_ident("node") {
                attr.parse_args().ok()
            } else {
                None
            }
        })
        .ok_or_else(|| {
            Error::new_spanned(field, "NodeTreeView: every field must have a #[node(..)]")
        })?;

    let field_ty = &field.ty;
    let span = field_ty.span();

    // Check if the type is GodotNodeHandle or Option<GodotNodeHandle>
    let (is_optional, _inner_type) = match get_option_inner_type(field_ty) {
        Some(inner) => (true, inner),
        None => (false, field_ty),
    };

    let path_value = node_path.value();

    // Check if the path contains wildcards for pattern matching
    if path_value.contains('*') {
        create_pattern_matching_expr(&path_value, is_optional, span)
    } else {
        // Use existing direct path logic for non-pattern paths
        create_direct_path_expr(&node_path, is_optional, span)
    }
}

fn create_direct_path_expr(
    node_path: &syn::LitStr,
    is_optional: bool,
    span: proc_macro2::Span,
) -> syn::Result<TokenStream2> {
    let expr = if is_optional {
        quote_spanned! { span =>
            {
                let base_node = &node;
                base_node.has_node(#node_path)
                    .then(|| {
                        let node_ref = base_node.get_node_as::<godot::classes::Node>(#node_path);
                        godot_bevy::interop::GodotNodeHandle::new(node_ref)
                    })
            }
        }
    } else {
        quote_spanned! { span =>
            {
                let base_node = &node;
                let node_ref = base_node.get_node_as::<godot::classes::Node>(#node_path);
                godot_bevy::interop::GodotNodeHandle::new(node_ref)
            }
        }
    };
    Ok(expr)
}

fn create_pattern_matching_expr(
    path_pattern: &str,
    is_optional: bool,
    span: proc_macro2::Span,
) -> syn::Result<TokenStream2> {
    let expr = if is_optional {
        quote_spanned! { span =>
            {
                let base_node = &node;
                godot_bevy::node_tree_view::find_node_by_pattern(base_node, #path_pattern)
                    .map(|node_ref| godot_bevy::interop::GodotNodeHandle::new(node_ref))
            }
        }
    } else {
        quote_spanned! { span =>
            {
                let base_node = &node;
                let pattern = #path_pattern;
                let node_ref = godot_bevy::node_tree_view::find_node_by_pattern(base_node, pattern)
                    .unwrap_or_else(|| panic!("Could not find node matching pattern: {pattern}"));
                godot_bevy::interop::GodotNodeHandle::new(node_ref)
            }
        }
    };
    Ok(expr)
}

// Helper function to extract the inner type of an Option<T>
fn get_option_inner_type(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(type_path) = ty
        && type_path.path.segments.len() == 1
        && type_path.path.segments[0].ident == "Option"
        && let syn::PathArguments::AngleBracketed(ref args) = type_path.path.segments[0].arguments
        && args.args.len() == 1
        && let syn::GenericArgument::Type(ref inner_type) = args.args[0]
    {
        return Some(inner_type);
    }
    None
}
