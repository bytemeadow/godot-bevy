use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{
    Data, DeriveInput, Error, Field, Fields, Ident, LitStr, Result, Token, braced,
    parse_macro_input,
};

/// Attribute macro that ensures a system runs on the main thread by adding a NonSend<MainThreadMarker> parameter.
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
) -> Result<TokenStream2> {
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
    mapping: ComponentMapping,
}

#[derive(Debug, Clone)]
enum ComponentMapping {
    Default,                             // (Component)
    SingleField(Ident),                  // (Component: field)
    MultipleFields(Vec<(Ident, Ident)>), // (Component { bevy_field: godot_field })
}

impl Parse for BevyBundleAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let mut components = Vec::new();

        while !input.is_empty() {
            // Parse component specification
            let component_content;
            syn::parenthesized!(component_content in input);

            let component_name: Ident = component_content.parse()?;

            // Determine the mapping type
            let mapping = if component_content.peek(Token![:]) {
                // Single field mapping: (Component: field)
                let _colon: Token![:] = component_content.parse()?;
                let field: Ident = component_content.parse()?;

                ComponentMapping::SingleField(field)
            } else if component_content.peek(syn::token::Brace) {
                // Multiple field mapping: (Component { bevy_field: godot_field, ... })
                let field_content;
                braced!(field_content in component_content);

                let mut field_mappings = Vec::new();

                while !field_content.is_empty() {
                    let bevy_field: Ident = field_content.parse()?;
                    let _colon: Token![:] = field_content.parse()?;
                    let godot_field: Ident = field_content.parse()?;

                    field_mappings.push((bevy_field, godot_field));

                    // Handle optional trailing comma
                    if field_content.peek(Token![,]) {
                        let _comma: Token![,] = field_content.parse()?;
                    }
                }

                ComponentMapping::MultipleFields(field_mappings)
            } else {
                // Default mapping: (Component)
                ComponentMapping::Default
            };

            components.push(ComponentSpec {
                component_name,
                mapping,
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

    // Get struct fields to check for transform_with attributes
    let fields = match &input.data {
        Data::Struct(data) => &data.fields,
        _ => {
            return Err(Error::new_spanned(
                &input,
                "BevyBundle can only be used on structs",
            ));
        }
    };

    // Helper function to extract transform_with from field attributes
    let extract_transform_with = |field_name: &Ident| -> Option<syn::Path> {
        for field in fields {
            if let Some(fname) = &field.ident {
                if fname == field_name {
                    for attr in &field.attrs {
                        if attr.path().is_ident("bundle") || attr.path().is_ident("bevy_bundle") {
                            // Parse the bundle attribute
                            if let Ok(syn::Meta::NameValue(name_value)) =
                                attr.parse_args::<syn::Meta>()
                            {
                                if name_value.path.is_ident("transform_with") {
                                    if let syn::Expr::Lit(expr_lit) = &name_value.value {
                                        if let syn::Lit::Str(lit_str) = &expr_lit.lit {
                                            let transform_str = lit_str.value();
                                            if let Ok(path) =
                                                syn::parse_str::<syn::Path>(&transform_str)
                                            {
                                                return Some(path);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        None
    };

    // Auto-generate bundle name from struct name
    let bundle_name = syn::Ident::new(&format!("{struct_name}Bundle"), struct_name.span());

    // Generate bundle struct
    let bundle_fields: Vec<_> = attr_args
        .components
        .iter()
        .map(|spec| {
            let component_name = &spec.component_name;
            let field_name = format!("{component_name}").to_lowercase();
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
            let field_name = format!("{component_name}").to_lowercase();
            let field_ident = syn::Ident::new(&field_name, component_name.span());

            match &spec.mapping {
                ComponentMapping::Default => {
                    // Marker component with no field mapping - use default
                    quote! {
                        #field_ident: #component_name::default()
                    }
                }
                ComponentMapping::SingleField(source_field) => {
                    // Component with single field mapping (tuple struct)
                    // Check if this field has a transform_with attribute
                    if let Some(transformer) = extract_transform_with(source_field) {
                        quote! {
                            #field_ident: #component_name(#transformer(node.bind().#source_field.clone()))
                        }
                    } else {
                        quote! {
                            #field_ident: #component_name(node.bind().#source_field.clone())
                        }
                    }
                }
                ComponentMapping::MultipleFields(field_mappings) => {
                    // Component with multiple field mappings (struct initialization)
                    let field_inits: Vec<_> = field_mappings
                        .iter()
                        .map(|(bevy_field, godot_field)| {
                            // Check if this field has a transform_with attribute
                            if let Some(transformer) = extract_transform_with(godot_field) {
                                quote! {
                                    #bevy_field: #transformer(node.bind().#godot_field.clone())
                                }
                            } else {
                                quote! {
                                    #bevy_field: node.bind().#godot_field.clone()
                                }
                            }
                        })
                        .collect();

                    quote! {
                        #field_ident: #component_name {
                            #(#field_inits),*,
                            ..Default::default()
                        }
                    }
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
    let bundle_name_lower = bundle_name.to_string().to_lowercase();
    let create_bundle_fn_name = syn::Ident::new(
        &format!("__create_{bundle_name_lower}_bundle"),
        bundle_name.span(),
    );

    // Generate the bundle registration (always enabled now)
    let bundle_impl = quote! {
        fn #create_bundle_fn_name(
            commands: &mut bevy::ecs::system::Commands,
            entity: bevy::ecs::entity::Entity,
            handle: &godot_bevy::interop::GodotNodeHandle,
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

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_bevy_bundle_basic_syntax() {
        let input: DeriveInput = parse_quote! {
            #[bevy_bundle((TestComponent: test_field))]
            struct TestNode {
                test_field: String,
            }
        };

        let result = bevy_bundle(input);
        assert!(result.is_ok(), "Basic syntax should parse successfully");
    }

    #[test]
    fn test_bevy_bundle_with_transform() {
        let input: DeriveInput = parse_quote! {
            #[bevy_bundle((TestComponent: test_field))]
            struct TestNode {
                #[bundle(transform_with = "String::from")]
                test_field: String,
            }
        };

        let result = bevy_bundle(input);
        assert!(result.is_ok(), "Transform syntax should parse successfully");

        let output = result.unwrap();
        let output_str = output.to_string();

        // Check that the transformer function is called in the generated code
        assert!(
            output_str.contains("String :: from"),
            "Should contain the transformer function"
        );
    }

    #[test]
    fn test_bevy_bundle_multiple_fields() {
        let input: DeriveInput = parse_quote! {
            #[bevy_bundle((TestComponent { name: test_name, value: test_value }))]
            struct TestNode {
                #[bundle(transform_with = "String::from")]
                test_name: String,
                test_value: i32,
            }
        };

        let result = bevy_bundle(input);
        assert!(
            result.is_ok(),
            "Multiple fields syntax should parse successfully"
        );

        let output = result.unwrap();
        let output_str = output.to_string();

        // Check that the transformer is only applied to the specified field
        assert!(
            output_str.contains("String :: from"),
            "Should contain the transformer function"
        );
        assert!(
            output_str.contains("test_name"),
            "Should contain the field name"
        );
        assert!(
            output_str.contains("test_value"),
            "Should contain the other field"
        );
    }

    #[test]
    fn test_bevy_bundle_default_component() {
        let input: DeriveInput = parse_quote! {
            #[bevy_bundle((MarkerComponent))]
            struct TestNode {
                test_field: String,
            }
        };

        let result = bevy_bundle(input);
        assert!(
            result.is_ok(),
            "Default component syntax should parse successfully"
        );

        let output = result.unwrap();
        let output_str = output.to_string();

        // Check that default() is called for marker components
        assert!(
            output_str.contains("MarkerComponent :: default ()"),
            "Should use default for marker components"
        );
    }

    #[test]
    fn test_extract_transform_with_function() {
        // Test the helper function directly by creating a more complex scenario
        let input: DeriveInput = parse_quote! {
            #[bevy_bundle((TestComponent: test_field))]
            struct TestNode {
                #[bundle(transform_with = "custom_transformer")]
                test_field: String,
                other_field: i32,
            }
        };

        let result = bevy_bundle(input);
        assert!(result.is_ok());

        let output = result.unwrap().to_string();
        assert!(
            output.contains("custom_transformer"),
            "Should call the custom transformer function"
        );
        assert!(
            output.contains("node . bind () . test_field . clone ()"),
            "Should access the field correctly"
        );
    }
}
