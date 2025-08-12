use super::attr::{GodotNodeAttrArgs, KeyValue};
use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, format_ident, quote, quote_spanned};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Meta, Token, parse_quote, parse2};

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

/// Parses the following format:
/// ```ignore
/// export_type(<godot_type>), transform_with(<conversion_function>), default(<default_value>)
/// ```
impl Parse for GodotExportAttrArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let arguments = Punctuated::<KeyValue, Token![,]>::parse_terminated(input)?;
        let mut export_type = None;
        let mut transform_with = None;
        let mut default = None;

        for argument in arguments {
            if argument.key == "export_type" {
                export_type = Some(
                    parse2::<syn::Type>(argument.value.to_token_stream()).map_err(|err| {
                        syn::Error::new(
                            argument.value.span(),
                            format!("Failed to parse `export_type` parameter: {err}"),
                        )
                    })?,
                );
            } else if argument.key == "transform_with" {
                transform_with = Some(
                    parse2::<syn::Type>(argument.value.to_token_stream()).map_err(|err| {
                        syn::Error::new(
                            argument.value.span(),
                            format!("Failed to parse `transform_with` parameter: {err}"),
                        )
                    })?,
                );
            } else if argument.key == "default" {
                default = Some(argument.value);
            } else {
                return Err(syn::Error::new(
                    argument.key.span(),
                    format!(
                        "Unknown parameter: `{}`. Expected `export_type`, `transform_with`, or `default`.",
                        argument.key
                    ),
                ));
            }
        }

        Ok(GodotExportAttrArgs {
            export_type,
            transform_with,
            default,
        })
    }
}

fn get_godot_export_type(field: &ComponentField) -> TokenStream2 {
    field
        .export_attribute
        .as_ref()
        .and_then(|args| {
            args.export_type
                .as_ref()
                .map(|ty| quote_spanned! {ty.span()=>#ty})
        })
        .unwrap_or_else(|| {
            let ty = &field.field_type;
            quote_spanned! {field.field_type.span()=>#ty}
        })
}

/// Parses the following format:
/// ```ignore
/// export_type(<godot_type>), transform_with(<conversion_function>), default(<default_value>)
/// ```
fn parse_godot_export_args(attr: &syn::Attribute) -> syn::Result<Option<GodotExportAttrArgs>> {
    match &attr.meta {
        Meta::List(meta_list) => match parse2::<GodotExportAttrArgs>(meta_list.tokens.clone()) {
            Ok(export_type_transform) => Ok(Some(export_type_transform)),
            Err(error) => Err(syn::Error::new(
                error.span(),
                format!("Failed to parse export parameters: {error}"),
            )),
        },
        Meta::NameValue(_) => Err(syn::Error::new(
            attr.span(),
            "Unexpected named value attribute.",
        )),
        // #[godot_export] without attributes is allowed.
        Meta::Path(_) => Ok(Some(GodotExportAttrArgs {
            export_type: None,
            transform_with: None,
            default: None,
        })),
    }
}

/// Parses the following format:
/// ```ignore
/// #[godot_export(export_type(<godot_type>), transform_with(<conversion_function>), default(<default_value>))]
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
        Data::Struct(data_struct) => data_struct
            .fields
            .iter()
            .map(parse_field)
            .collect::<syn::Result<Vec<ComponentField>>>()?,
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
                    quote_spanned! {transform_with.span()=>
                        #[export]
                        #[bevy_bundle(transform_with=#transform_with_str_lit)]
                        #field_name: #export_type
                    }
                } else {
                    quote_spanned! {export_type.span()=>
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
            let export_type = get_godot_export_type(field);
            let default = field
                .export_attribute
                .as_ref()
                .and_then(|attr| attr.default.as_ref())
                .map(|default_expr| quote!(#default_expr))
                .unwrap_or(quote!(#export_type::default()));
            quote! {
                #name: #default
            }
        })
        .collect::<Vec<TokenStream2>>();

    let bevy_bundle_init = if field_names.is_empty() {
        quote! {
            #[bevy_bundle( (#struct_name) )]
        }
    } else {
        quote! {
            #[bevy_bundle( (#struct_name{ #(#field_names: #field_names),* }) )]
        }
    };

    let godot_node_struct = quote! {
        #[derive(godot::prelude::GodotClass, godot_bevy::prelude::BevyBundle)]
        #[class(base=#godot_node_type)]
        #bevy_bundle_init
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

    Ok(godot_node_struct)
}

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn test_godot_node_base() {
        let input: DeriveInput = parse_quote! {
            #[derive(Component, GodotNode)]
            #[godot_node(base(Sprite2D))]
            pub struct Sprite;
        };

        let result = component_as_godot_node_impl(input.into_token_stream());
        assert!(result.is_ok(), "Syntax should parse successfully");

        let result = result.unwrap();
        assert!(result.to_string().contains("# [class (base = Sprite2D)]"));
        assert!(
            result
                .to_string()
                .contains("base : godot :: prelude :: Base < godot :: classes :: Sprite2D >")
        );
    }

    #[test]
    fn test_godot_node_class_name() {
        let input: DeriveInput = parse_quote! {
            #[derive(Component, GodotNode)]
            #[godot_node(class_name(MyNode))]
            pub struct MyComponent;
        };

        let result = component_as_godot_node_impl(input.into_token_stream());
        assert!(result.is_ok(), "Syntax should parse successfully");

        let result = result.unwrap();
        assert!(result.to_string().contains("pub struct MyNode"));
        assert!(
            result
                .to_string()
                .contains("impl godot :: classes :: INode for MyNode")
        );
    }

    #[test]
    fn test_simple_export_field() {
        let input: DeriveInput = parse_quote! {
            #[derive(Component, GodotNode)]
            pub struct Player {
                #[godot_export]
                pub position: f32,
            }
        };

        let result = component_as_godot_node_impl(input.into_token_stream());
        assert!(result.is_ok(), "Syntax should parse successfully");

        let result = result.unwrap();
        assert!(
            result
                .to_string()
                .contains("Player { position : position }")
        );
        assert!(result.to_string().contains("# [export] position : f32"));
        assert!(result.to_string().contains("position : f32 :: default ()"));
    }

    #[test]
    fn test_advanced_godot_export() {
        let input: DeriveInput = parse_quote! {
            #[derive(Component, GodotNode)]
            pub struct Player {
                #[godot_export(
                    export_type(Vector2),
                    transform_with(transform_to_vec2),
                    default(Vector2::new(5.0, 15.0)),
                )]
                pub position: Vec2,
            }
        };

        let result = component_as_godot_node_impl(input.into_token_stream());
        assert!(result.is_ok(), "Syntax should parse successfully");

        let result = result.unwrap();
        assert!(result.to_string().contains("position : Vector2"));
        assert!(
            result
                .to_string()
                .contains("# [bevy_bundle (transform_with = \"transform_to_vec2\")]")
        );
        assert!(
            result
                .to_string()
                .contains("position : Vector2 :: new (5.0 , 15.0)")
        );
    }

    #[test]
    fn test_all_parameters() {
        let input: DeriveInput = parse_quote! {
            #[derive(Component, GodotNode)]
            #[godot_node(base(Node2D), class_name(PlayerNode))]
            pub struct Player {
                #[godot_export(
                    export_type(Vector2),
                    transform_with(transform_to_vec2),
                    default(Vector2::new(5.0, 15.0))
                )]
                pub position: Vec2,
            }
        };

        let result = component_as_godot_node_impl(input.into_token_stream());
        assert!(result.is_ok(), "Syntax should parse successfully");
    }
}
