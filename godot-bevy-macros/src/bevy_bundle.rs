use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Error, Token, braced};

// Parse bevy_bundle attribute syntax
struct BevyBundleAttr {
    components: Vec<ComponentSpec>,
}

impl Parse for BevyBundleAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut components = Vec::new();

        while !input.is_empty() {
            // Parse component specification
            let component_content;
            syn::parenthesized!(component_content in input);

            let component_name: syn::Path = component_content.parse()?;

            // Determine the mapping type
            let mapping = if component_content.peek(Token![:]) {
                // Single field mapping: (Component: field)
                let _colon: Token![:] = component_content.parse()?;
                let field: syn::Ident = component_content.parse()?;

                ComponentMapping::SingleField(field)
            } else if component_content.peek(syn::token::Brace) {
                // Multiple field mapping: (Component { bevy_field: godot_field, ... })
                let field_content;
                braced!(field_content in component_content);

                let mut field_mappings = Vec::new();

                while !field_content.is_empty() {
                    let bevy_field: syn::Ident = field_content.parse()?;
                    let _colon: Token![:] = field_content.parse()?;
                    let godot_field: syn::Ident = field_content.parse()?;

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

pub fn bevy_bundle(input: DeriveInput) -> syn::Result<TokenStream2> {
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
    let extract_transform_with = |field_name: &syn::Ident| -> Option<syn::Path> {
        for field in fields {
            if let Some(fname) = &field.ident
                && fname == field_name
            {
                for attr in &field.attrs {
                    if attr.path().is_ident("bundle") || attr.path().is_ident("bevy_bundle") {
                        // Parse the bundle attribute
                        if let Ok(syn::Meta::NameValue(name_value)) = attr.parse_args::<syn::Meta>()
                            && name_value.path.is_ident("transform_with")
                            && let syn::Expr::Lit(expr_lit) = &name_value.value
                            && let syn::Lit::Str(lit_str) = &expr_lit.lit
                        {
                            let transform_str = lit_str.value();
                            if let Ok(path) = syn::parse_str::<syn::Path>(&transform_str) {
                                return Some(path);
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
            let last_segment = component_name
                .segments
                .last()
                .expect("component to have at least one path segment");
            let field_name = last_segment.ident.to_string().to_lowercase();
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
            let last_segment = component_name.segments.last()
                .expect("component to have at least one path segment");
            let field_name = last_segment.ident.to_string().to_lowercase();
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

                    // Avoid Clippy warning: struct update has no effect,
                    // all the fields in the struct have already been specified
                    // https://rust-lang.github.io/rust-clippy/master/index.html#needless_update
                    // It's not possible to determine how many fields the component
                    // struct has from this macro, so we have to allow the warning.
                    quote! {
                        #[allow(clippy::needless_update)]
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

struct ComponentSpec {
    component_name: syn::Path,
    mapping: ComponentMapping,
}

#[derive(Debug, Clone)]
enum ComponentMapping {
    Default,                                       // (Component)
    SingleField(syn::Ident),                       // (Component: field)
    MultipleFields(Vec<(syn::Ident, syn::Ident)>), // (Component { bevy_field: godot_field })
}

#[cfg(test)]
mod tests {
    use crate::bevy_bundle::*;
    use syn::{DeriveInput, parse_quote};

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
            #[bevy_bundle((fully::qualified::path::to::TestComponent { name: test_name, value: test_value }))]
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
