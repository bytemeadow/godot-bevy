use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::parse::{Parse, ParseStream};
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

    // Generate tuple elements extracting component values from the Godot node
    let component_constructors: Vec<_> = attr_args
        .components
        .iter()
        .map(|spec| {
            let component_name = &spec.component_name;
            match &spec.mapping {
                ComponentMapping::Default => {
                    quote! { #component_name::default() }
                }
                ComponentMapping::SingleField(source_field) => {
                    if let Some(transformer) = extract_transform_with(source_field) {
                        quote! { #component_name(#transformer(node.bind().#source_field.clone())) }
                    } else {
                        quote! { #component_name(node.bind().#source_field.clone()) }
                    }
                }
                ComponentMapping::MultipleFields(field_mappings) => {
                    let field_inits: Vec<_> = field_mappings
                        .iter()
                        .map(|(bevy_field, godot_field)| {
                            if let Some(transformer) = extract_transform_with(godot_field) {
                                quote! { #bevy_field: #transformer(node.bind().#godot_field.clone()) }
                            } else {
                                quote! { #bevy_field: node.bind().#godot_field.clone() }
                            }
                        })
                        .collect();

                    // It's not possible to determine from this macro whether the
                    // component struct has unlisted fields, so the struct-update
                    // syntax may be redundant; allow the lint at the fn level.
                    quote! {
                        #component_name {
                            #(#field_inits),*,
                            ..Default::default()
                        }
                    }
                }
            }
        })
        .collect();

    let struct_name_lower = struct_name.to_string().to_lowercase();
    let create_bundle_fn_name = syn::Ident::new(
        &format!("__create_{struct_name_lower}_bundle"),
        struct_name.span(),
    );

    let bundle_impl = quote! {
        #[allow(clippy::needless_update)]
        fn #create_bundle_fn_name(
            commands: &mut godot_bevy::bevy_ecs::system::Commands,
            entity: godot_bevy::bevy_ecs::entity::Entity,
            godot: &mut godot_bevy::interop::GodotAccess,
            handle: godot_bevy::interop::GodotNodeHandle,
        ) -> bool {
            // Try to get the node as the correct type
            if let Some(node) = godot.try_get::<#struct_name>(handle) {
                commands.entity(entity).insert((
                    #(#component_constructors,)*
                ));
                return true;
            }
            false
        }

        // Auto-register this bundle using inventory
        godot_bevy::inventory::submit! {
            godot_bevy::prelude::AutoSyncBundleRegistry {
                godot_class_name: stringify!(#struct_name),
                godot_class_id_fn: || <#struct_name as godot::prelude::GodotClass>::class_id(),
                create_bundle_fn: #create_bundle_fn_name,
            }
        }
    };

    let expanded = quote! {
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
    fn test_no_bundle_struct_generated() {
        let input: DeriveInput = parse_quote! {
            #[bevy_bundle((TestComponent: test_field), (MarkerComponent))]
            struct TestNode {
                test_field: String,
            }
        };

        let output = bevy_bundle(input).unwrap().to_string();
        assert!(
            !output.contains("struct TestNodeBundle"),
            "Should not generate a bundle struct"
        );
        assert!(
            output.contains("insert (("),
            "Should insert a tuple of components"
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

    #[test]
    fn test_registers_godot_class_id_fn() {
        let input: DeriveInput = parse_quote! {
            #[bevy_bundle((MarkerComponent))]
            struct TestNode {
                test_field: String,
            }
        };

        let output = bevy_bundle(input).unwrap().to_string();
        assert!(
            output.contains("godot_class_id_fn"),
            "registry submit should set godot_class_id_fn"
        );
        assert!(
            output.contains("GodotClass") && output.contains("class_id ()"),
            "should emit the GodotClass::class_id() call, not just the field name"
        );
    }
}
