use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, format_ident, quote};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Meta, Token, parse_quote, parse2};

struct KeyValue {
    key: syn::Ident,
    value: syn::Expr,
}

struct GodotNodeAttrArgs {
    base: Option<syn::Ident>,
    class_name: Option<syn::Ident>,
}

#[derive(Clone)]
struct GodotExportAttrArgs {
    export_type: Option<syn::Type>,
    transform_with: Option<syn::Type>,
    default: Option<syn::Expr>,
}

#[derive(Clone)]
struct ComponentField {
    name: syn::Ident,
    field_type: syn::Type,
    export_attribute: Option<GodotExportAttrArgs>,
}

impl Parse for KeyValue {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let name: syn::Ident = input.parse()?;
        input.parse::<Token![=]>()?;
        let value: syn::Expr = input.parse()?;
        Ok(KeyValue { key: name, value })
    }
}

/// Parses the following format:
/// ```ignore
/// base = <godot_type>, class_name = <identifier>
/// ```
impl Parse for GodotNodeAttrArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let arguments = Punctuated::<KeyValue, Token![,]>::parse_terminated(&input)?;
        let mut base = None;
        let mut class_name = None;

        for argument in arguments {
            if argument.key == "base" {
                base = Some(parse2::<syn::Ident>(argument.value.to_token_stream())?);
            } else if argument.key == "class_name" {
                class_name = Some(parse2::<syn::Ident>(argument.value.to_token_stream())?);
            } else {
                return Err(syn::Error::new(
                    argument.key.span(),
                    format!(
                        "Unknown parameter: `{}`. Expected `base` or `class_name`.",
                        argument.key
                    ),
                ));
            }
        }

        Ok(GodotNodeAttrArgs { base, class_name })
    }
}

/// Parses the following format:
/// ```ignore
/// export_type = <godot_type>, transform_with = <conversion_function>, default = <default_value>
/// ```
impl Parse for GodotExportAttrArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let arguments = Punctuated::<KeyValue, Token![,]>::parse_terminated(input)?;
        let mut export_type = None;
        let mut transform_with = None;
        let mut default = None;

        for argument in arguments {
            if argument.key == "export_type" {
                export_type = Some(parse2::<syn::Type>(argument.value.to_token_stream())?);
            } else if argument.key == "transform_with" {
                transform_with = Some(parse2::<syn::Type>(argument.value.to_token_stream())?);
            } else if argument.key == "default" {
                default = Some(argument.value);
            } else {
                return Err(syn::Error::new(
                    argument.key.span(),
                    format!(
                        "Unknown parameter: `{}`. Expected `export_type` or `transform_with`.",
                        argument.key
                    ),
                ));
            }
        }

        if export_type.is_some() && transform_with.is_some() {
            Ok(GodotExportAttrArgs {
                export_type,
                transform_with,
                default,
            })
        } else {
            Err(syn::Error::new(
                input.span(),
                "Both `export_type` and `transform_with` must be provided".to_string(),
            ))
        }
    }
}

fn get_godot_export_type(field: &ComponentField) -> &syn::Type {
    field
        .export_attribute
        .as_ref()
        .and_then(|args| args.export_type.as_ref())
        .unwrap_or(&field.field_type)
}

/// Parses the following format:
/// ```ignore
/// export_type = <godot_type>, transform_with = <conversion_function>, default = <default_value>
/// ```
fn parse_godot_export_args(attr: &syn::Attribute) -> syn::Result<Option<GodotExportAttrArgs>> {
    match &attr.meta {
        Meta::List(meta_list) => match parse2::<GodotExportAttrArgs>(meta_list.tokens.clone()) {
            Ok(export_type_transform) => Ok(Some(export_type_transform)),
            Err(error) => Err(syn::Error::new(
                error.span(),
                format!("Failed to parse export parameters: {}", error),
            )),
        },
        Meta::NameValue(_) => Err(syn::Error::new(
            attr.span(),
            "Unexpected named value attribute.",
        )),
        Meta::Path(_) => Ok(None), // Plain #[export] attribute is allowed.
    }
}

/// Parses the following format:
/// ```ignore
/// #[godot_export(export_type = <godot_type>, transform_with = <conversion_function>, default = <default_value>)]
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
        .map(parse_godot_export_args)
        .transpose()?
        .flatten();
    Ok(ComponentField {
        name: field_name,
        field_type,
        export_attribute,
    })
}

pub fn component_as_godot_node_impl(input: TokenStream2) -> syn::Result<TokenStream2> {
    let input = parse2::<DeriveInput>(input)?;

    let struct_name: &syn::Ident = &input.ident;

    let struct_fields: Vec<ComponentField> = match &input.data {
        Data::Struct(data_struct) => {
            match data_struct
                .fields
                .iter()
                .map(|field| parse_field(field))
                .collect::<syn::Result<Vec<ComponentField>>>()
            {
                Ok(fields) => fields,
                Err(err) => return Err(err),
            }
        }
        _ => return Err(syn::Error::new(input.span(), "Only works on structs")),
    };

    let mut godot_node_attr: Option<GodotNodeAttrArgs> = None;
    for attr in &input.attrs {
        if attr.path().is_ident("godot_node") {
            match &attr.meta {
                Meta::List(meta_list) => {
                    godot_node_attr = Some(parse2::<GodotNodeAttrArgs>(meta_list.tokens.clone())?);
                }
                _ => return Err(syn::Error::new(attr.span(), "Expected a list of arguments")),
            }
        }
    }

    let godot_node_name = godot_node_attr
        .as_ref()
        .and_then(|attr| attr.class_name.clone())
        .unwrap_or(format_ident!("{}BevyComponent", struct_name));
    if struct_name == &godot_node_name {
        return Err(syn::Error::new(
            godot_node_name.span(),
            "Cannot use the same name for the Godot Node name as the Bevy Component struct name.",
        ));
    }

    let godot_node_type = godot_node_attr
        .as_ref()
        .and_then(|attr| attr.base.clone())
        .unwrap_or(parse_quote!(Node));
    let godot_inode_type = format_ident!("I{}", godot_node_type);

    let field_names = struct_fields
        .iter()
        .filter(|field| field.export_attribute.is_some())
        .map(|attr| attr.name.clone())
        .collect::<Vec<syn::Ident>>();

    let godot_node_fields = struct_fields
        .iter()
        .filter(|field| field.export_attribute.is_some())
        .map(|field| {
            let field_name = &field.name;
            let export_type = get_godot_export_type(field);
            if let Some(export_attribute) = &field.export_attribute {
                if let Some(transform_with) = &export_attribute.transform_with {
                    let transform_with_str_lit = syn::LitStr::new(
                        transform_with.to_token_stream().to_string().as_str(),
                        transform_with.span(),
                    );
                    quote! {
                        #[export]
                        #[bevy_bundle(transform_with=#transform_with_str_lit)]
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

    let default_export_fields = struct_fields
        .iter()
        .filter(|field| field.export_attribute.is_some())
        .map(|field| {
            let name = &field.name;
            let ty = get_godot_export_type(field);
            let default = field
                .export_attribute
                .as_ref()
                .and_then(|attr| attr.default.as_ref())
                .map(|default_expr| quote!(#default_expr))
                .unwrap_or(quote!(#ty::default()));
            quote! {
                #name: #default
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
        #[class(base=#godot_node_type)]
        #[bevy_bundle(
            (godot_bevy::plugins::component_as_godot_node_child::UninitializedBevyComponentNode),
            (#struct_name #bundle_init)
        )]
        pub struct #godot_node_name {
            base: godot::prelude::Base<godot::classes::#godot_node_type>,
            #(#godot_node_fields),*
        }
        #[godot::prelude::godot_api]
        impl godot::classes::#godot_inode_type for #godot_node_name {
            fn init(base: godot::prelude::Base<godot::classes::#godot_node_type>) -> Self {
                Self {
                    base,
                    #(#default_export_fields),*
                }
            }
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

    // println!("{}", final_output.to_string());
    // println!("count: {}", default_export_fields.len());

    Ok(final_output)
}
