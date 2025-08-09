use proc_macro2::TokenStream as TokenStream2;
use quote::{ToTokens, format_ident, quote, quote_spanned};
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{Data, DeriveInput, Error, Expr, Fields, Ident, Meta, Path, Token, Type, parse2};

// ----------------------------
// Godot node attributes parser
// ----------------------------

struct KeyValueArg {
    key: Ident,
    value: syn::Expr,
}

impl Parse for KeyValueArg {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let key: Ident = input.parse()?;
        let content;
        syn::parenthesized!(content in input);
        let value: syn::Expr = content.parse()?;
        Ok(KeyValueArg { key, value })
    }
}

#[derive(Clone)]
struct GodotNodeAttrArgs {
    base: Option<Ident>,
    class_name: Option<Ident>,
}

impl Parse for GodotNodeAttrArgs {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let args = Punctuated::<KeyValueArg, Token![,]>::parse_terminated(input)?;
        let mut base = None;
        let mut class_name = None;

        for kv in args {
            if kv.key == "base" {
                base = Some(parse2::<Ident>(kv.value.to_token_stream())?);
            } else if kv.key == "class_name" {
                class_name = Some(parse2::<Ident>(kv.value.to_token_stream())?);
            } else {
                return Err(Error::new(
                    kv.key.span(),
                    format!(
                        "Unknown parameter: `{}`. Expected `base` or `class_name`.",
                        kv.key
                    ),
                ));
            }
        }

        Ok(GodotNodeAttrArgs { base, class_name })
    }
}

// ----------------------------
// godot_props(...) parser
// ----------------------------

#[derive(Clone)]
enum PropKind {
    // Tuple/newtype component – property name is the bundle field name
    Tuple,
    // Struct component field – property name is the Bevy field name
    StructField(Ident),
}

#[derive(Clone)]
struct GodotPropEntry {
    kind: PropKind,
    export_type: Type,
    transform_with: Option<Path>,
    default_expr: Option<Expr>,
}

struct GodotPropsAttr {
    entries: Vec<GodotPropEntry>,
}

impl Parse for GodotPropsAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // godot_props((...), (...), ...)
        let entries_paren = Punctuated::<PropEntryParen, Token![,]>::parse_terminated(input)?;
        let mut entries = Vec::with_capacity(entries_paren.len());
        for entry in entries_paren {
            entries.push(entry.0);
        }
        Ok(GodotPropsAttr { entries })
    }
}

// A single parenthesized entry: (field?, export_type(..)?, transform_with(..)?, default(..)?)
struct PropEntryParen(GodotPropEntry);

impl Parse for PropEntryParen {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let content;
        syn::parenthesized!(content in input);

        // First token: either ':' for tuple or an Ident for struct field
        let kind = if content.peek(Token![:]) {
            let _colon: Token![:] = content.parse()?;
            PropKind::Tuple
        } else {
            let field_ident: Ident = content.parse()?;
            PropKind::StructField(field_ident)
        };

        // Optional trailing config: comma-separated key(value) entries
        let mut export_type: Option<Type> = None;
        let mut transform_with: Option<Path> = None;
        let mut default_expr: Option<Expr> = None;

        while !content.is_empty() {
            // Consume optional comma
            if content.peek(Token![,]) {
                let _comma: Token![,] = content.parse()?;
                if content.is_empty() {
                    break;
                }
            }

            let key: Ident = content.parse()?;
            let args;
            syn::parenthesized!(args in content);

            if key == "export_type" {
                if export_type.is_some() {
                    return Err(Error::new(key.span(), "Duplicate export_type(..)"));
                }
                let ty: Type = args.parse()?;
                export_type = Some(ty);
            } else if key == "transform_with" {
                if transform_with.is_some() {
                    return Err(Error::new(key.span(), "Duplicate transform_with(..)"));
                }
                let path: Path = args.parse()?;
                transform_with = Some(path);
            } else if key == "default" {
                if default_expr.is_some() {
                    return Err(Error::new(key.span(), "Duplicate default(..)"));
                }
                let expr: Expr = args.parse()?;
                default_expr = Some(expr);
            } else {
                return Err(Error::new(
                    key.span(),
                    "Unknown key. Expected export_type(..), transform_with(..), or default(..)",
                ));
            }
        }

        // export_type(..) is required because we cannot infer Bevy field types here
        let export_type = export_type.ok_or_else(|| {
            Error::new(
                match &kind {
                    PropKind::Tuple => content.span(),
                    PropKind::StructField(ident) => ident.span(),
                },
                "Missing export_type(..) – required for GodotNode on Bundles",
            )
        })?;

        Ok(PropEntryParen(GodotPropEntry {
            kind,
            export_type,
            transform_with,
            default_expr,
        }))
    }
}

fn parse_godot_props_attr(attr: &syn::Attribute) -> syn::Result<Option<GodotPropsAttr>> {
    if !attr.path().is_ident("godot_props") {
        return Ok(None);
    }
    match &attr.meta {
        Meta::List(list) => parse2::<GodotPropsAttr>(list.tokens.clone()).map(Some),
        _ => Err(Error::new(
            attr.span(),
            "Expected a list of entries: #[godot_props((...), (...))]",
        )),
    }
}

// ----------------------------
// Implementation
// ----------------------------

pub fn godot_node_bundle_impl(input: DeriveInput) -> syn::Result<TokenStream2> {
    let struct_name = &input.ident;

    // Ensure we are working on a struct with fields
    let data_struct = match &input.data {
        Data::Struct(data) => data,
        _ => {
            return Err(Error::new_spanned(
                &input,
                "GodotNode (bundle mode) can only be used on structs",
            ));
        }
    };

    if matches!(data_struct.fields, Fields::Unit) {
        return Err(Error::new_spanned(
            &input,
            "GodotNode (bundle mode) must be used on structs with fields",
        ));
    }

    // Parse struct-level godot_node(base(..), class_name(..))
    let mut godot_node_attr: Option<GodotNodeAttrArgs> = None;
    for attr in &input.attrs {
        if attr.path().is_ident("godot_node") {
            match &attr.meta {
                Meta::List(meta_list) => {
                    godot_node_attr = Some(parse2::<GodotNodeAttrArgs>(meta_list.tokens.clone())?);
                }
                _ => {
                    return Err(Error::new(
                        attr.span(),
                        "Expected a list of arguments for #[godot_node(..)]",
                    ));
                }
            }
        }
    }

    let godot_node_name: Ident = godot_node_attr
        .as_ref()
        .and_then(|a| a.class_name.clone())
        .unwrap_or_else(|| format_ident!("{}Node", struct_name));

    if struct_name == &godot_node_name {
        return Err(Error::new(
            godot_node_name.span(),
            "Cannot use the same name for the Godot Node as the Bundle struct name.",
        ));
    }

    let godot_node_type: Ident = godot_node_attr
        .as_ref()
        .and_then(|a| a.base.clone())
        .unwrap_or_else(|| format_ident!("Node"));
    let godot_inode_type = format_ident!("I{}", godot_node_type);

    // Collect exported properties from all fields
    // Also construct tokens for building each component from the node
    let mut exported_props: Vec<(Ident, Type, Option<Expr>)> = Vec::new();
    let mut bundle_field_constructors: Vec<TokenStream2> = Vec::new();

    // Note: We intentionally allow nested bundles. Bevy will flatten nested bundles
    // at insertion time. Detecting nested bundles reliably at macro time is not possible
    // without unstable negative trait bounds. Components without `#[godot_props]` must
    // implement `Default` so nested bundles can be constructed.

    // Track property name collisions
    use std::collections::HashSet;
    let mut seen_prop_names: HashSet<String> = HashSet::new();

    for field in data_struct.fields.iter() {
        let field_ident = field
            .ident
            .clone()
            .ok_or_else(|| Error::new(field.span(), "Bundle fields must be named"))?;
        let field_ty = field.ty.clone();

        // Parse optional godot_props on this field
        let mut entries: Vec<GodotPropEntry> = Vec::new();
        for attr in &field.attrs {
            if let Some(parsed) = parse_godot_props_attr(attr)? {
                entries.extend(parsed.entries.into_iter());
            }
        }

        // Generate exported properties for this component field
        // and the constructor for the component value.
        if entries.is_empty() {
            // No exported properties – require Default via construction
            bundle_field_constructors.push(quote! {
                #field_ident: <#field_ty as ::core::default::Default>::default()
            });
            continue;
        }

        // Separate entries kinds to detect invalid mixes
        let has_tuple = entries.iter().any(|e| matches!(e.kind, PropKind::Tuple));
        let has_struct = entries
            .iter()
            .any(|e| matches!(e.kind, PropKind::StructField(_)));
        if has_tuple && has_struct {
            return Err(Error::new(
                field.span(),
                "Cannot mix tuple (:) and struct-field entries in one #[godot_props(..)]",
            ));
        }

        if has_tuple {
            // Only one tuple entry is allowed
            if entries.len() != 1 {
                return Err(Error::new(
                    field.span(),
                    "Tuple/newtype mapping must have exactly one entry",
                ));
            }
            let entry = entries.into_iter().next().unwrap();

            // Property name is the bundle field name
            let prop_ident = field_ident.clone();
            let prop_name_str = prop_ident.to_string();
            if !seen_prop_names.insert(prop_name_str.clone()) {
                return Err(Error::new(
                    field.span(),
                    format!("Duplicate exported property `{prop_name_str}`"),
                ));
            }

            // Exported property declaration
            let export_ty = entry.export_type.clone();
            let default_expr = entry.default_expr.clone().unwrap_or_else(|| {
                parse2::<Expr>(quote_spanned! {export_ty.span()=> #export_ty :: default()}).unwrap()
            });
            exported_props.push((prop_ident.clone(), export_ty.clone(), Some(default_expr)));

            // Component constructor – apply transform if provided
            let value_tokens = if let Some(transform) = entry.transform_with.clone() {
                quote! { #transform(node.bind().#prop_ident.clone()) }
            } else {
                quote! { node.bind().#prop_ident.clone() }
            };

            bundle_field_constructors.push(quote! {
                #field_ident: #field_ty( #value_tokens )
            });
        } else {
            // Struct-field entries
            let mut field_inits: Vec<TokenStream2> = Vec::new();
            for entry in entries.iter() {
                let bevy_field_ident = match &entry.kind {
                    PropKind::StructField(id) => id.clone(),
                    PropKind::Tuple => unreachable!(),
                };

                // Property name equals the Bevy field ident
                let prop_ident = bevy_field_ident.clone();
                let prop_name_str = prop_ident.to_string();
                if !seen_prop_names.insert(prop_name_str.clone()) {
                    return Err(Error::new(
                        field.span(),
                        format!("Duplicate exported property `{prop_name_str}`"),
                    ));
                }

                let export_ty = entry.export_type.clone();
                let default_expr = entry.default_expr.clone().unwrap_or_else(|| {
                    parse2::<Expr>(quote_spanned! {export_ty.span()=> #export_ty :: default()})
                        .unwrap()
                });
                exported_props.push((prop_ident.clone(), export_ty.clone(), Some(default_expr)));

                let value_tokens = if let Some(transform) = entry.transform_with.clone() {
                    quote! { #transform(node.bind().#prop_ident.clone()) }
                } else {
                    quote! { node.bind().#prop_ident.clone() }
                };
                field_inits.push(quote! { #bevy_field_ident: #value_tokens });
            }

            // Construct the struct with Default for the rest of the fields.
            bundle_field_constructors.push(quote! {
                #field_ident: #field_ty {
                    #(#field_inits,)*
                    ..::core::default::Default::default()
                }
            });
        }
    }

    // Build Godot class fields and their defaults
    let godot_node_fields: Vec<TokenStream2> = exported_props
        .iter()
        .map(|(name, ty, _)| {
            quote_spanned! {ty.span()=>
                #[export]
                #name: #ty
            }
        })
        .collect();

    let default_export_fields: Vec<TokenStream2> = exported_props
        .iter()
        .map(|(name, ty, default)| {
            let default_expr = default.clone().unwrap_or_else(|| {
                parse2::<Expr>(quote_spanned! {ty.span()=> #ty :: default()}).unwrap()
            });
            quote! { #name: #default_expr }
        })
        .collect();

    // Bundle constructor from Godot node
    let bundle_constructor = quote! {
        impl #struct_name {
            pub fn from_godot_node(node: &godot::obj::Gd<#godot_node_name>) -> Self {
                Self {
                    #(#bundle_field_constructors,)*
                }
            }
        }
    };

    // Registration function and inventory submit
    let bundle_name_lower = struct_name.to_string().to_lowercase();
    let create_bundle_fn_name = Ident::new(
        &format!("__create_{bundle_name_lower}_bundle"),
        struct_name.span(),
    );

    let bundle_impl = quote! {
        fn #create_bundle_fn_name(
            commands: &mut bevy::ecs::system::Commands,
            entity: bevy::ecs::entity::Entity,
            handle: &godot_bevy::interop::GodotNodeHandle,
        ) -> bool {
            if let Some(godot_node) = handle.clone().try_get::<#godot_node_name>() {
                let bundle = #struct_name::from_godot_node(&godot_node);
                commands.entity(entity).insert(bundle);
                return true;
            }
            false
        }

        godot_bevy::inventory::submit! {
            godot_bevy::prelude::AutoSyncBundleRegistry {
                godot_class_name: stringify!(#godot_node_name),
                create_bundle_fn: #create_bundle_fn_name,
            }
        }
    };

    // Generate the Godot node class
    let godot_node_struct = quote! {
        #[derive(godot::prelude::GodotClass)]
        #[class(base=#godot_node_type)]
        pub struct #godot_node_name {
            base: godot::prelude::Base<godot::classes::#godot_node_type>,
            #(#godot_node_fields,)*
        }

        #[godot::prelude::godot_api]
        impl godot::classes::#godot_inode_type for #godot_node_name {
            fn init(base: godot::prelude::Base<godot::classes::#godot_node_type>) -> Self {
                Self {
                    base,
                    #(#default_export_fields,)*
                }
            }
        }
    };

    let expanded = quote! {
        // Ensure this type implements Bevy's Bundle trait
        const _: fn() = || {
            fn assert_impl_bundle<T: bevy::prelude::Bundle>() {}
            assert_impl_bundle::<#struct_name>();
        };

        #godot_node_struct
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
    fn tuple_entry_parses_and_generates() {
        let input: DeriveInput = parse_quote! {
            #[derive(Bundle, GodotNode)]
            #[godot_node(base(Node2D), class_name(PlayerNode))]
            struct PlayerBundle {
                #[godot_props((:, export_type(f32), default(5.0)))]
                speed: Speed,
            }
        };

        let result = godot_node_bundle_impl(input);
        assert!(result.is_ok(), "Tuple entry should parse");
        let tokens = result.unwrap().to_string();
        assert!(tokens.contains("pub struct PlayerNode"));
        assert!(tokens.contains("# [export] speed : f32"));
        assert!(tokens.contains("speed : 5.0"));
        assert!(tokens.contains("PlayerBundle :: from_godot_node"));
    }

    #[test]
    fn struct_entries_parses_and_generates() {
        let input: DeriveInput = parse_quote! {
            #[derive(Bundle, GodotNode)]
            #[godot_node(base(Node2D), class_name(PlayerNode))]
            struct PlayerBundle {
                #[godot_props(
                    (current, export_type(i32), default(100)),
                    (max, export_type(i32))
                )]
                health: Health,
            }
        };

        let result = godot_node_bundle_impl(input);
        assert!(result.is_ok(), "Struct entries should parse");
        let tokens = result.unwrap().to_string();
        assert!(tokens.contains("# [export] current : i32"));
        assert!(tokens.contains("# [export] max : i32"));
        // default(100) appears in init
        assert!(tokens.contains("current : 100"));
    }

    #[test]
    fn transform_and_default_handling() {
        let input: DeriveInput = parse_quote! {
            #[derive(Bundle, GodotNode)]
            #[godot_node(base(Node2D), class_name(PlayerNode))]
            struct PlayerBundle {
                #[godot_props(
                    (pos, export_type(Vector2), transform_with(to_vec2), default(Vector2::ZERO))
                )]
                physics: Physics,
            }
        };

        let result = godot_node_bundle_impl(input).unwrap();
        let tokens = result.to_string();
        assert!(tokens.contains("# [export] pos : Vector2"));
        assert!(tokens.contains("pos : Vector2 :: ZERO"));
        // Ensure transform function name is present in constructor path
        assert!(tokens.contains("to_vec2"));
    }

    #[test]
    fn mixed_tuple_and_struct_is_error() {
        let input: DeriveInput = parse_quote! {
            #[derive(Bundle, GodotNode)]
            #[godot_node(base(Node2D), class_name(PlayerNode))]
            struct PlayerBundle {
                #[godot_props((:, export_type(f32)), (value, export_type(f32)))]
                comp: Comp,
            }
        };

        let err = godot_node_bundle_impl(input).unwrap_err();
        let msg = err.to_string();
        assert!(msg.contains("Cannot mix tuple (:) and struct-field entries"));
    }

    #[test]
    fn missing_export_type_is_error() {
        let input: DeriveInput = parse_quote! {
            #[derive(Bundle, GodotNode)]
            #[godot_node(base(Node2D), class_name(PlayerNode))]
            struct PlayerBundle {
                #[godot_props((value))]
                comp: Comp,
            }
        };

        let err = godot_node_bundle_impl(input).unwrap_err();
        assert!(err.to_string().contains("Missing export_type(..)"));
    }

    #[test]
    fn duplicate_property_across_fields_is_error() {
        let input: DeriveInput = parse_quote! {
            #[derive(Bundle, GodotNodeBundle)]
            #[godot_node(base(Node2D), class_name(PlayerNode))]
            struct PlayerBundle {
                #[godot_props((hp, export_type(i32)))]
                health: Health,
                #[godot_props((hp, export_type(i32)))]
                stats: Stats,
            }
        };

        let err = godot_node_bundle_impl(input).unwrap_err();
        assert!(err.to_string().contains("Duplicate exported property"));
    }
}
