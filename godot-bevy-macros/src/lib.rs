use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, quote_spanned};
use syn::spanned::Spanned;
use syn::{
    Attribute, Data, DeriveInput, Error, Field, Fields, LitStr, Result, Type, parse_macro_input,
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

#[proc_macro_derive(BevyComponent, attributes(bevy_component, sync))]
pub fn derive_bevy_component(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    let expanded = bevy_component(input).unwrap_or_else(Error::into_compile_error);
    TokenStream::from(expanded)
}

fn bevy_component(input: DeriveInput) -> Result<TokenStream2> {
    let godot_struct_name = &input.ident;
    let data_struct = match &input.data {
        Data::Struct(data_struct) => data_struct,
        _ => {
            return Err(Error::new_spanned(
                input,
                "BevyComponent must be used on structs",
            ));
        }
    };

    // Get the component name from attribute or default to <GodotStruct>Component
    let component_name = get_component_name(&input.attrs, godot_struct_name)?;

    // Find exported fields that should be synced to Bevy
    let synced_fields = data_struct
        .fields
        .iter()
        .filter(|field| has_export_attr(field) && should_sync_field(field))
        .collect::<Vec<_>>();

    if synced_fields.is_empty() {
        return Err(Error::new_spanned(
            input,
            "BevyComponent: No exported fields found to sync. Use #[export] and optionally #[sync] on fields.",
        ));
    }

    // Generate the component struct
    let component_fields = synced_fields.iter().map(|field| {
        let name = &field.ident;
        let ty = &field.ty;
        quote! { pub #name: #ty }
    });

    // Generate the sync function
    let sync_assignments = synced_fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let getter_name = syn::Ident::new(&format!("get_{}", field_name), field_name.span());
        quote! {
            component.#field_name = godot_node.bind().#getter_name();
        }
    });

    // Generate default values for the component
    let default_assignments = synced_fields.iter().map(|field| {
        let field_name = field.ident.as_ref().unwrap();
        let default_value = get_field_default_value(&field.ty);
        quote! {
            #field_name: #default_value
        }
    });

    let plugin_name = syn::Ident::new(
        &format!("{}AutoSyncPlugin", component_name),
        component_name.span(),
    );

    let expanded = quote! {
        #[derive(bevy::prelude::Component, Debug, Clone)]
        pub struct #component_name {
            #(#component_fields,)*
        }

        impl Default for #component_name {
            fn default() -> Self {
                Self {
                    #(#default_assignments,)*
                }
            }
        }

        impl #component_name {
            /// Synchronize this component's values from the corresponding Godot node
            pub fn sync_from_godot(&mut self, godot_node: &mut godot_bevy::bridge::GodotNodeHandle) {
                let mut component = self;
                let mut godot_node = godot_node.get::<#godot_struct_name>();
                #(#sync_assignments)*
            }

            /// Create a new component instance synchronized from a Godot node
            pub fn from_godot(godot_node: &mut godot_bevy::bridge::GodotNodeHandle) -> Self {
                let mut component = Self::default();
                component.sync_from_godot(godot_node);
                component
            }
        }

        // Implement AutoSyncComponent trait for automatic syncing
        impl godot_bevy::prelude::AutoSyncComponent for #component_name {
            type GodotType = #godot_struct_name;

            fn auto_sync(&mut self, godot_node: &mut godot_bevy::bridge::GodotNodeHandle) {
                self.sync_from_godot(godot_node);
            }
        }

        // Auto-generated plugin that includes the sync system!
        pub struct #plugin_name;

        impl bevy::prelude::Plugin for #plugin_name {
            fn build(&self, app: &mut bevy::prelude::App) {
                app.add_systems(
                    bevy::prelude::Update,
                    |mut query: bevy::prelude::Query<
                        (&mut #component_name, &mut godot_bevy::bridge::GodotNodeHandle),
                        bevy::prelude::Added<godot_bevy::bridge::GodotNodeHandle>
                    >| {
                        for (mut component, mut godot_handle) in query.iter_mut() {
                            component.auto_sync(&mut godot_handle);
                        }
                    }
                );
            }
        }

        impl Default for #plugin_name {
            fn default() -> Self {
                Self
            }
        }
    };

    Ok(expanded)
}

fn get_component_name(attrs: &[Attribute], godot_struct_name: &syn::Ident) -> Result<syn::Ident> {
    // Look for #[bevy_component(name = "CustomName")]
    for attr in attrs {
        if attr.path().is_ident("bevy_component") {
            if let Ok(name_lit) = attr.parse_args::<LitStr>() {
                return Ok(syn::Ident::new(&name_lit.value(), name_lit.span()));
            }
        }
    }

    // Default: <GodotStruct>Component
    let component_name = format!("{}Component", godot_struct_name);
    Ok(syn::Ident::new(&component_name, godot_struct_name.span()))
}

fn has_export_attr(field: &Field) -> bool {
    field
        .attrs
        .iter()
        .any(|attr| attr.path().is_ident("export"))
}

fn should_sync_field(field: &Field) -> bool {
    // If no #[sync] attribute is found, default to syncing all exported fields
    // If #[sync] is found, only sync those explicitly marked
    let has_sync_anywhere = field.attrs.iter().any(|attr| attr.path().is_ident("sync"));

    if has_sync_anywhere {
        field.attrs.iter().any(|attr| attr.path().is_ident("sync"))
    } else {
        true // Default: sync all exported fields
    }
}

fn get_field_default_value(ty: &Type) -> TokenStream2 {
    match ty {
        Type::Path(type_path) if type_path.path.is_ident("f32") => quote! { 0.0 },
        Type::Path(type_path) if type_path.path.is_ident("f64") => quote! { 0.0 },
        Type::Path(type_path) if type_path.path.is_ident("i32") => quote! { 0 },
        Type::Path(type_path) if type_path.path.is_ident("i64") => quote! { 0 },
        Type::Path(type_path) if type_path.path.is_ident("u32") => quote! { 0 },
        Type::Path(type_path) if type_path.path.is_ident("u64") => quote! { 0 },
        Type::Path(type_path) if type_path.path.is_ident("bool") => quote! { false },
        Type::Path(type_path) if type_path.path.segments.last().unwrap().ident == "String" => {
            quote! { String::new() }
        }
        Type::Path(type_path) if type_path.path.segments.last().unwrap().ident == "Vector2" => {
            quote! { godot::builtin::Vector2::ZERO }
        }
        Type::Path(type_path) if type_path.path.segments.last().unwrap().ident == "Vector3" => {
            quote! { godot::builtin::Vector3::ZERO }
        }
        _ => quote! { Default::default() },
    }
}
