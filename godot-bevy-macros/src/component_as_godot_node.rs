use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};
use std::collections::HashMap;
use syn::parse::ParseStream;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Token, parse_macro_input, parse_str};

const EXPORT_TYPE_KEY: &str = "export_type";
const TRANSFORM_WITH_KEY: &str = "transform_with";

#[derive(Clone)]
struct ComponentField {
    field_name: syn::Ident,
    field_type: syn::Type,
    export: Option<ExportAttribute>,
}

#[derive(Clone)]
struct ExportAttribute {
    alternate_type: Option<ExportTypeTransform>,
}

#[derive(Clone)]
struct ExportTypeTransform {
    export_type: syn::Type,
    transform_with: syn::LitStr,
}

pub fn component_as_godot_node_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    let struct_name: &syn::Ident = &input.ident;
    // TODO: I'm open to name prefix/suffix suggestions for node names
    let godot_node_name = format_ident!("{}BevyComponent", struct_name);

    let struct_fields: Vec<ComponentField> = match &input.data {
        Data::Struct(data_struct) => {
            match data_struct
                .fields
                .iter()
                .map(|field| parse_field(field))
                .collect::<syn::Result<Vec<ComponentField>>>()
            {
                Ok(fields) => fields,
                Err(err) => return err.to_compile_error().into(),
            }
        }
        _ => {
            return syn::Error::new(input.span(), "Only works on structs")
                .to_compile_error()
                .into();
        }
    };

    let field_names = struct_fields
        .iter()
        .map(|attr| attr.field_name.clone())
        .collect::<Vec<syn::Ident>>();
    let godot_node_fields = struct_fields
        .iter()
        .map(|attr| {
            let field_name = &attr.field_name;
            let export_type = attr
                .export
                .as_ref()
                .and_then(|x| x.alternate_type.as_ref())
                .map(|t| &t.export_type)
                .unwrap_or(&attr.field_type);
            if let Some(export_attribute) = attr.export.as_ref() {
                if let Some(alternate_type) = &export_attribute.alternate_type {
                    let transform_with = &alternate_type.transform_with;
                    // TODO: Default values don't show up in Godot editor
                    quote! {
                        #[export]
                        #[bevy_bundle(transform_with=#transform_with)]
                        #field_name: #export_type
                    }
                } else {
                    quote! {
                        #[export]
                        #field_name: #export_type
                    }
                }
            } else {
                quote!(#field_name: #export_type)
            }
        })
        .collect::<Vec<TokenStream2>>();

    let bundle_init = if field_names.len() == 0 {
        quote!()
    } else {
        quote! {
            { #(#field_names: #field_names),* }
        }
    };

    let godot_node_struct = quote! {
        #[derive(godot::prelude::GodotClass, godot_bevy::prelude::BevyBundle)]
        #[class(base=Node, init)]
        #[bevy_bundle(
            (godot_bevy::plugins::component_as_godot_node_child::UninitializedBevyComponentNode),
            (#struct_name #bundle_init)
        )]
        pub struct #godot_node_name {
            base: godot::prelude::Base<godot::classes::Node>,
            #(#godot_node_fields),*
        }
    };

    let inventory_registration = quote! {
        ::godot_bevy::inventory::submit! {
            ::godot_bevy::plugins::component_as_godot_node_child::ChildComponentRegistry {
                create_system_fn: |app| {
                    app.add_systems(
                        bevy::app::PreUpdate,
                        ::godot_bevy::plugins::component_as_godot_node_child::move_component_from_child_to_parent::<#struct_name>
                    );
                }
            }
        }
    };

    let final_output = quote! {
        #inventory_registration
        #godot_node_struct
    };

    TokenStream::from(final_output)
}

/// Parses the following format:
/// ```ignore
/// #[export(export_type = "<godot_type>", transform_with = "<conversion_function>")]
/// <field_name>: <field_type>,
/// ```
fn parse_field(field: &syn::Field) -> syn::Result<ComponentField> {
    let field_type = field.ty.clone();
    let field_name = field
        .ident
        .clone()
        .ok_or(syn::Error::new(field.span(), "Field must be named"))?;
    let export_attribute = field
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("export"))
        .map(parse_export_parameters)
        .transpose()?
        .map(|params| ExportAttribute {
            alternate_type: params,
        });
    Ok(ComponentField {
        field_name,
        field_type,
        export: export_attribute,
    })
}

/// Parses the following format:
/// ```ignore
/// export_type = "<godot_type>", transform_with = "<conversion_function>"
/// ```
fn parse_export_parameters(attr: &syn::Attribute) -> syn::Result<Option<ExportTypeTransform>> {
    if let syn::Meta::List(meta_list) = &attr.meta {
        let parameter_map = meta_list.parse_args_with(|input: ParseStream| {
            let mut parameter_map: HashMap<String, syn::LitStr> = HashMap::new();
            loop {
                let key = input.parse::<syn::Ident>()?;
                let key_str = key.to_string();
                if key_str != EXPORT_TYPE_KEY && key_str != TRANSFORM_WITH_KEY {
                    return Err(syn::Error::new(
                        key.span(),
                        format!("Invalid parameter: {}. Only '{EXPORT_TYPE_KEY}' and '{TRANSFORM_WITH_KEY}' are allowed", key_str),
                    ));
                }

                let _eq = input.parse::<Token![=]>()?;
                let parameter = input.parse::<syn::LitStr>()?;
                parameter_map.insert(key_str, parameter);

                if input.is_empty() {
                    break;
                }
                let _comma = input.parse::<Token![,]>()?;
                if input.is_empty() {
                    break;
                }
            }

            Ok(parameter_map)
        })?;

        if parameter_map.is_empty() {
            Ok(None)
        } else if parameter_map.contains_key(EXPORT_TYPE_KEY)
            && parameter_map.contains_key(TRANSFORM_WITH_KEY)
        {
            let export_type = parameter_map.get(EXPORT_TYPE_KEY).unwrap();
            let export_type = parse_str::<syn::Type>(export_type.value().as_str())?;
            let transform_with = parameter_map.get(TRANSFORM_WITH_KEY).unwrap();
            Ok(Some(ExportTypeTransform {
                export_type,
                transform_with: transform_with.clone(),
            }))
        } else {
            Err(syn::Error::new(
                meta_list.span(),
                "Both export_type and transform_with must be provided",
            ))
        }
    } else {
        Ok(None)
    }
}
