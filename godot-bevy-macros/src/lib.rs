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

#[proc_macro_derive(BevyComponent, attributes(bevy_component))]
pub fn derive_bevy_component(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    let expanded = bevy_component(input).unwrap_or_else(Error::into_compile_error);

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

    // Create appropriate expression based on whether the field is optional
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

// Parse bevy_component attribute syntax
struct BevyComponentAttr {
    bundle_name: Ident,
    components: Vec<ComponentSpec>,
}

struct ComponentSpec {
    component_name: Ident,
    source_field: Option<Ident>,
}

impl Parse for BevyComponentAttr {
    fn parse(input: ParseStream) -> Result<Self> {
        let bundle_name: Ident = input.parse()?;
        let content;
        syn::parenthesized!(content in input);

        let mut components = Vec::new();
        while !content.is_empty() {
            let component_content;
            syn::parenthesized!(component_content in content);

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

            if !content.is_empty() {
                let _comma: Token![,] = content.parse()?;
            }
        }

        Ok(BevyComponentAttr {
            bundle_name,
            components,
        })
    }
}

fn bevy_component(input: DeriveInput) -> Result<TokenStream2> {
    let struct_name = &input.ident;

    // Find the bevy_component attribute
    let bevy_attr = input
        .attrs
        .iter()
        .find(|attr| attr.path().is_ident("bevy_component"))
        .ok_or_else(|| Error::new_spanned(&input, "Missing #[bevy_component(...)] attribute"))?;

    let attr_args: BevyComponentAttr = bevy_attr.parse_args()?;
    let bundle_name = &attr_args.bundle_name;

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

    // Generate the auto-sync plugin
    let plugin_name = syn::Ident::new(
        &format!("{}AutoSyncPlugin", bundle_name),
        bundle_name.span(),
    );
    let sync_system_name = syn::Ident::new(
        &format!("sync_{}_components", bundle_name.to_string().to_lowercase()),
        bundle_name.span(),
    );

    // Use the first component as a marker to check if the bundle is already added
    let first_component = &attr_args.components[0].component_name;

    let plugin_impl = quote! {
        pub struct #plugin_name;

        impl bevy::app::Plugin for #plugin_name {
            fn build(&self, app: &mut bevy::app::App) {
                app.add_systems(bevy::app::Update, #sync_system_name);
            }
        }

        fn #sync_system_name(
            mut commands: bevy::ecs::system::Commands,
            nodes: bevy::ecs::system::Query<(bevy::ecs::entity::Entity, &godot_bevy::bridge::GodotNodeHandle), (
                bevy::ecs::query::With<godot_bevy::bridge::GodotNodeHandle>,
                bevy::ecs::query::Without<#first_component>
            )>,
        ) {
            for (entity, handle) in nodes.iter() {
                if let Some(godot_node) = handle.clone().try_get::<#struct_name>() {
                    let bundle = #bundle_name::from_godot_node(&godot_node);
                    commands.entity(entity).insert(bundle);
                    bevy::log::debug!("Added {} bundle to entity {:?}", stringify!(#bundle_name), entity);
                }
            }
        }
    };

    let expanded = quote! {
        #bundle_struct

        #bundle_constructor

        #plugin_impl
    };

    Ok(expanded)
}
