use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{
    Data, DeriveInput, Error, Field, Fields, Ident, LitStr, Result, Token, parse_macro_input,
};

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
                    let mut app_builder_func = godot_bevy::app::BEVY_INIT_FUNC.lock().unwrap();
                    if app_builder_func.is_none() {
                        *app_builder_func = Some(Box::new(#name));
                    }
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

    let expanded = node_tree_view(view).unwrap_or_else(Error::into_compile_error);

    TokenStream::from(expanded)
}

#[proc_macro_derive(BevyBundle, attributes(bevy_bundle))]
pub fn derive_bevy_bundle(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    let expanded = bevy_bundle(input).unwrap_or_else(Error::into_compile_error);

    TokenStream::from(expanded)
}

fn node_tree_view(input: DeriveInput) -> Result<TokenStream2> {
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

fn create_get_node_expr(field: &Field) -> Result<TokenStream2> {
    let node_path: LitStr = field
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
    node_path: &LitStr,
    is_optional: bool,
    span: proc_macro2::Span,
) -> Result<TokenStream2> {
    let expr = if is_optional {
        quote_spanned! { span =>
            {
                let base_node = &node;
                base_node.has_node(#node_path)
                    .then(|| {
                        let node_ref = base_node.get_node_as::<godot::classes::Node>(#node_path);
                        godot_bevy::bridge::GodotNodeHandle::new(node_ref)
                    })
            }
        }
    } else {
        quote_spanned! { span =>
            {
                let base_node = &node;
                let node_ref = base_node.get_node_as::<godot::classes::Node>(#node_path);
                godot_bevy::bridge::GodotNodeHandle::new(node_ref)
            }
        }
    };
    Ok(expr)
}

fn create_pattern_matching_expr(
    path_pattern: &str,
    is_optional: bool,
    span: proc_macro2::Span,
) -> Result<TokenStream2> {
    let expr = if is_optional {
        quote_spanned! { span =>
            {
                let base_node = &node;
                godot_bevy::node_tree_view::find_node_by_pattern(base_node, #path_pattern)
                    .map(|node_ref| godot_bevy::bridge::GodotNodeHandle::new(node_ref))
            }
        }
    } else {
        quote_spanned! { span =>
            {
                let base_node = &node;
                let node_ref = godot_bevy::node_tree_view::find_node_by_pattern(base_node, #path_pattern)
                    .expect(&format!("Could not find node matching pattern: {}", #path_pattern));
                godot_bevy::bridge::GodotNodeHandle::new(node_ref)
            }
        }
    };
    Ok(expr)
}

// Helper function to extract the inner type of an Option<T>
fn get_option_inner_type(ty: &syn::Type) -> Option<&syn::Type> {
    if let syn::Type::Path(type_path) = ty {
        if type_path.path.segments.len() == 1 && type_path.path.segments[0].ident == "Option" {
            if let syn::PathArguments::AngleBracketed(ref args) =
                type_path.path.segments[0].arguments
            {
                if args.args.len() == 1 {
                    if let syn::GenericArgument::Type(ref inner_type) = args.args[0] {
                        return Some(inner_type);
                    }
                }
            }
        }
    }
    None
}

// Parse bevy_bundle attribute syntax
struct BevyBundleAttr {
    components: Vec<ComponentSpec>,
}

struct ComponentSpec {
    component_name: Ident,
    source_field: Option<Ident>,
}

impl Parse for BevyBundleAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut components = Vec::new();

        while !input.is_empty() {
            // Parse component specification
            let component_content;
            syn::parenthesized!(component_content in input);

            let component_name: Ident = component_content.parse()?;

            // Check if there's a colon and source field mapping
            let source_field = if component_content.peek(Token![:]) {
                let _colon: Token![:] = component_content.parse()?;
                Some(component_content.parse()?)
            } else {
                None
            };

            components.push(ComponentSpec {
                component_name,
                source_field,
            });

            if !input.is_empty() {
                let _comma: Token![,] = input.parse()?;
            }
        }

        Ok(BevyBundleAttr { components })
    }
}

fn bevy_bundle(input: DeriveInput) -> Result<TokenStream2> {
    let struct_name = &input.ident;

    // Find the bevy_bundle attribute
    let bevy_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("bevy_bundle"))
        .ok_or_else(|| Error::new_spanned(&input, "Missing #[bevy_bundle(...)] attribute"))?;

    let attr_args: BevyBundleAttr = bevy_attr.parse_args()?;

    // Auto-generate bundle name from struct name
    let bundle_name = syn::Ident::new(&format!("{}Bundle", struct_name), struct_name.span());

    // Generate bundle struct
    let bundle_fields: Vec<_> = attr_args
        .components
        .iter()
        .map(|spec| {
            let component_name = &spec.component_name;
            let field_name = format!("{}", component_name).to_lowercase();
            let field_ident = syn::Ident::new(&field_name, component_name.span());
            quote! {
                pub #field_ident: #component_name
            }
        })
        .collect();

    let bundle_struct = quote! {
        #[derive(bevy::prelude::Bundle)]
        pub struct #bundle_name {
            #(#bundle_fields),*
        }
    };

    // Generate implementation for extracting values from the Godot node
    let bundle_constructor_fields: Vec<_> = attr_args
        .components
        .iter()
        .map(|spec| {
            let component_name = &spec.component_name;
            let field_name = format!("{}", component_name).to_lowercase();
            let field_ident = syn::Ident::new(&field_name, component_name.span());

            if let Some(source_field) = &spec.source_field {
                // Component with field mapping
                quote! {
                    #field_ident: #component_name(node.bind().#source_field)
                }
            } else {
                // Marker component with no field mapping - use default
                quote! {
                    #field_ident: #component_name::default()
                }
            }
        })
        .collect();

    let bundle_constructor = quote! {
        impl #bundle_name {
            pub fn from_godot_node(node: &godot::obj::Gd<#struct_name>) -> Self {
                Self {
                    #(#bundle_constructor_fields),*
                }
            }
        }
    };

    // Use the first component as a marker to check if the bundle is already added
    let _first_component = &attr_args.components[0].component_name;

    // Generate the bundle creation function
    let create_bundle_fn_name = syn::Ident::new(
        &format!("__create_{}_bundle", bundle_name.to_string().to_lowercase()),
        bundle_name.span(),
    );

    // Generate the bundle registration (always enabled now)
    let bundle_impl = quote! {
        fn #create_bundle_fn_name(
            commands: &mut bevy::ecs::system::Commands,
            entity: bevy::ecs::entity::Entity,
            handle: &godot_bevy::bridge::GodotNodeHandle,
        ) -> bool {
            // Try to get the node as the correct type
            if let Some(godot_node) = handle.clone().try_get::<#struct_name>() {
                let bundle = #bundle_name::from_godot_node(&godot_node);
                commands.entity(entity).insert(bundle);
                return true;
            }
            false
        }

        // Auto-register this bundle using inventory
        godot_bevy::inventory::submit! {
            godot_bevy::prelude::AutoSyncBundleRegistry {
                godot_class_name: stringify!(#struct_name),
                create_bundle_fn: #create_bundle_fn_name,
            }
        }
    };

    let expanded = quote! {
        #bundle_struct

        #bundle_constructor

        #bundle_impl
    };

    Ok(expanded)
}
