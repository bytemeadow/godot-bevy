use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Token, parse_macro_input, parse2};

#[derive(Debug)]
enum GodotExportParams {
    ExportType,
    TransformWith,
}

impl GodotExportParams {
    fn as_str(&self) -> &str {
        match self {
            GodotExportParams::ExportType => "export_type",
            GodotExportParams::TransformWith => "transform_with",
        }
    }

    fn all_as_str() -> String {
        [
            GodotExportParams::ExportType.as_str(),
            GodotExportParams::TransformWith.as_str(),
        ]
        .join(", ")
    }
}

impl TryFrom<&syn::Ident> for GodotExportParams {
    type Error = syn::Error;

    fn try_from(ident: &syn::Ident) -> Result<Self, Self::Error> {
        match ident.to_string().as_str() {
            "export_type" => Ok(GodotExportParams::ExportType),
            "transform_with" => Ok(GodotExportParams::TransformWith),
            _ => Err(syn::Error::new(
                ident.span(),
                format!(
                    "Unknown parameter: {}. Valid parameters are: {}",
                    ident,
                    GodotExportParams::all_as_str()
                ),
            )),
        }
    }
}

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
    transform_with: syn::Type,
}

impl Parse for ExportTypeTransform {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut export_type = None;
        let mut transform_with = None;

        // Parse comma-separated name-value pairs
        let arguments =
            Punctuated::<KeyValue, Token![,]>::parse_terminated(input).map_err(|error| {
                syn::Error::new(
                    input.span(),
                    format!(
                        "Failed to parse comma separated list of arguments: {}",
                        error
                    ),
                )
            })?;

        for argument in arguments {
            match GodotExportParams::try_from(&argument.key) {
                Ok(GodotExportParams::ExportType) => {
                    let value = &argument.value;
                    export_type = Some(parse2::<syn::Type>(quote!(#value)).map_err(|error| {
                        syn::Error::new(
                            error.span(),
                            format!("Failed to parse export type: {}", error),
                        )
                    })?);
                }
                Ok(GodotExportParams::TransformWith) => {
                    let value = &argument.value;
                    transform_with =
                        Some(parse2::<syn::Type>(quote!(#value)).map_err(|error| {
                            syn::Error::new(
                                error.span(),
                                format!("Failed to parse transform_with type: {}", error),
                            )
                        })?);
                }
                Err(error) => {
                    return Err(error);
                }
            }
        }

        if let (Some(export_type), Some(transform_with)) = (export_type, transform_with) {
            Ok(ExportTypeTransform {
                export_type,
                transform_with,
            })
        } else {
            Err(syn::Error::new(
                input.span(),
                format!(
                    "Both {} and {} must be provided",
                    GodotExportParams::ExportType.as_str(),
                    GodotExportParams::TransformWith.as_str()
                ),
            ))
        }
    }
}

struct KeyValue {
    key: syn::Ident,
    value: syn::Expr,
}

impl Parse for KeyValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: syn::Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let value: syn::Expr = input.parse()?;
        Ok(KeyValue { key: name, value })
    }
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
                    let transform_with = syn::LitStr::new(
                        alternate_type
                            .transform_with
                            .to_token_stream()
                            .to_string()
                            .as_str(),
                        alternate_type.transform_with.span(),
                    );
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
/// #[godot_export(export_type = <godot_type>, transform_with = <conversion_function>)]
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
        .find(|attr| attr.path().is_ident("godot_export"))
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
/// export_type = <godot_type>, transform_with = <conversion_function>
/// ```
fn parse_export_parameters(attr: &syn::Attribute) -> syn::Result<Option<ExportTypeTransform>> {
    if let syn::Meta::List(meta_list) = &attr.meta {
        match parse2::<ExportTypeTransform>(meta_list.tokens.clone()) {
            Ok(export_type_transform) => Ok(Some(export_type_transform)),
            Err(error) => Err(syn::Error::new(
                error.span(),
                format!("Failed to parse export parameters: {}", error),
            )),
        }
    } else {
        Ok(None)
    }
}
