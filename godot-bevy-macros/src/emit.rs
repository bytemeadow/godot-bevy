use crate::bevy_attr::{ClassPlan, ComponentInit, ComponentPlan, Mapping, PrimaryPlan};
use proc_macro2::{TokenStream as TokenStream2, TokenTree};
use quote::{ToTokens, format_ident, quote};
use std::collections::HashSet;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Attribute, Data, DeriveInput, Expr, Ident, Path, Token, Type};

/// Lower a `ClassPlan` to the Godot class, autosync registration, and required-components
/// registrar. `input` is threaded through for two things the IR doesn't carry: the primary
/// field's Rust type (export fallback when no `as`) and the sibling Bevy `#[require(...)]`.
pub fn emit(plan: &ClassPlan, input: &DeriveInput) -> TokenStream2 {
    let mut out = TokenStream2::new();
    if plan.emit_node_class {
        out.extend(emit_node_class(plan, input));
    }
    out.extend(emit_autosync(plan));
    if let Some(trigger) = &plan.trigger {
        let sibling = collect_require_idents(&input.attrs);
        out.extend(emit_required_registration(plan, trigger, &sibling));
    }
    out
}

/// The generated `#[derive(GodotClass)]` struct, one `#[export]` per primary field and per
/// generated companion export.
fn emit_node_class(plan: &ClassPlan, input: &DeriveInput) -> TokenStream2 {
    let class = &plan.godot_class;
    let base = &plan.base;

    let mut exports: Vec<TokenStream2> = Vec::new();
    for m in &plan.primary.fields {
        let ty = m
            .as_type
            .clone()
            .or_else(|| primary_field_type(input, &m.godot_prop));
        exports.push(export_field(m, ty));
    }
    for c in &plan.companions {
        if !c.generated_exports {
            continue;
        }
        match &c.init {
            ComponentInit::Marker => {}
            ComponentInit::Newtype(m) => exports.push(export_field(m, m.as_type.clone())),
            ComponentInit::Fields(ms) => {
                for m in ms {
                    exports.push(export_field(m, m.as_type.clone()));
                }
            }
        }
    }

    quote! {
        #[derive(godot::prelude::GodotClass)]
        #[class(base = #base, init)]
        pub struct #class {
            base: godot::prelude::Base<godot::classes::#base>,
            #(#exports,)*
        }
    }
}

fn export_field(m: &Mapping, ty: Option<Type>) -> TokenStream2 {
    let prop = &m.godot_prop;
    let init = m.default.as_ref().map(|d| {
        let val = paren_wrap(d);
        quote!(#[init(val = #val)])
    });
    quote! {
        #[export]
        #init
        #prop: #ty
    }
}

/// The autosync `create_bundle_fn` + its `inventory::submit!`. Reads the editor-authored
/// `#[export]` values off the node and inserts them as a direct component tuple.
fn emit_autosync(plan: &ClassPlan) -> TokenStream2 {
    let class = &plan.godot_class;
    let fn_name = format_ident!("__create_{}_bundle", class.to_string().to_lowercase());

    let mut values: Vec<TokenStream2> = Vec::new();
    if let Some(pv) = primary_value(&plan.primary) {
        values.push(pv);
    }
    for c in &plan.companions {
        values.push(companion_value(c));
    }

    quote! {
        #[allow(clippy::needless_update)]
        fn #fn_name(
            commands: &mut godot_bevy::bevy_ecs::system::Commands,
            entity: godot_bevy::bevy_ecs::entity::Entity,
            godot: &mut godot_bevy::interop::GodotAccess,
            handle: godot_bevy::interop::GodotNodeHandle,
        ) -> bool {
            if let Some(node) = godot.try_get::<#class>(handle) {
                commands.entity(entity).insert(( #(#values,)* ));
                return true;
            }
            false
        }

        godot_bevy::inventory::submit! {
            godot_bevy::prelude::AutoSyncBundleRegistry {
                godot_class_name: stringify!(#class),
                godot_class_id_fn: || <#class as godot::prelude::GodotClass>::class_id(),
                create_bundle_fn: #fn_name,
            }
        }
    }
}

fn primary_value(primary: &PrimaryPlan) -> Option<TokenStream2> {
    if primary.path.segments.is_empty() {
        return None;
    }
    let path = &primary.path;
    if primary.fields.is_empty() {
        return Some(quote!(#path::default()));
    }
    let inits = primary.fields.iter().map(field_init);
    Some(quote!(#path { #(#inits,)* ..Default::default() }))
}

fn companion_value(c: &ComponentPlan) -> TokenStream2 {
    let path = &c.path;
    match &c.init {
        ComponentInit::Marker => quote!(#path::default()),
        ComponentInit::Newtype(m) => {
            let read = read_prop(m);
            quote!(#path(#read))
        }
        ComponentInit::Fields(ms) => {
            let inits = ms.iter().map(field_init);
            quote!(#path { #(#inits,)* ..Default::default() })
        }
    }
}

fn field_init(m: &Mapping) -> TokenStream2 {
    let field = m.bevy_field.as_ref().unwrap_or(&m.godot_prop);
    let read = read_prop(m);
    quote!(#field: #read)
}

/// `node.bind().prop.clone()`, run through `with(...)` when present.
fn read_prop(m: &Mapping) -> TokenStream2 {
    let prop = &m.godot_prop;
    let read = quote!(node.bind().#prop.clone());
    match &m.with {
        Some(w) => quote!(#w(#read)),
        None => read,
    }
}

/// Register companions as Bevy required components so pure-Bevy spawns get the declared
/// defaults. Uses the non-panicking `try_*` forms and logs on failure; skips any companion
/// already named in a sibling `#[require(...)]` to avoid Bevy's double-registration panic.
fn emit_required_registration(
    plan: &ClassPlan,
    trigger: &Path,
    sibling: &HashSet<String>,
) -> TokenStream2 {
    let mut regs: Vec<TokenStream2> = Vec::new();
    for c in &plan.companions {
        let comp = &c.path;
        if let Some(last) = comp.segments.last()
            && sibling.contains(&last.ident.to_string())
        {
            continue;
        }
        let on_err = registration_warn(comp, trigger);
        regs.push(match &c.init {
            ComponentInit::Marker => quote! {
                if let Err(e) = world.try_register_required_components::<#trigger, #comp>() {
                    #on_err
                }
            },
            ComponentInit::Newtype(m) => {
                let value = companion_default_value(m);
                quote! {
                    if let Err(e) = world.try_register_required_components_with::<#trigger, #comp>(
                        || #comp(#value)
                    ) {
                        #on_err
                    }
                }
            }
            ComponentInit::Fields(ms) => {
                let inits = ms.iter().map(|m| {
                    let field = m.bevy_field.as_ref().unwrap_or(&m.godot_prop);
                    let value = companion_default_value(m);
                    quote!(#field: #value)
                });
                quote! {
                    if let Err(e) = world.try_register_required_components_with::<#trigger, #comp>(
                        || #comp { #(#inits,)* ..::core::default::Default::default() }
                    ) {
                        #on_err
                    }
                }
            }
        });
    }

    if regs.is_empty() {
        return quote!();
    }

    let trigger_ident = trigger
        .segments
        .last()
        .map(|s| s.ident.to_string().to_lowercase())
        .unwrap_or_default();
    let fn_name = format_ident!("__register_required_components_for_{}", trigger_ident);

    quote! {
        #[allow(clippy::needless_update)]
        fn #fn_name(world: &mut godot_bevy::bevy_ecs::world::World) {
            #(#regs)*
        }

        godot_bevy::inventory::submit! {
            godot_bevy::prelude::GodotRequiredComponents {
                component_name: stringify!(#trigger),
                registrar_fn: #fn_name,
            }
        }
    }
}

fn registration_warn(comp: &Path, trigger: &Path) -> TokenStream2 {
    quote! {
        godot_bevy::tracing::warn!(
            "godot-bevy: failed to register required component {} for {}: {}",
            stringify!(#comp), stringify!(#trigger), e
        );
    }
}

/// The Bevy-side default for a generated-export companion: its export default (or the export
/// type's `Default`), run through `with(...)` when set.
fn companion_default_value(m: &Mapping) -> TokenStream2 {
    let ty = m.as_type.as_ref().expect("generated export has `as`");
    let default = m
        .default
        .as_ref()
        .map(|e| quote!(#e))
        .unwrap_or_else(|| quote!(<#ty as ::core::default::Default>::default()));
    match &m.with {
        Some(w) => quote!(#w(#default)),
        None => default,
    }
}

fn primary_field_type(input: &DeriveInput, ident: &Ident) -> Option<Type> {
    let Data::Struct(s) = &input.data else {
        return None;
    };
    s.fields
        .iter()
        .find(|f| f.ident.as_ref() == Some(ident))
        .map(|f| f.ty.clone())
}

/// gdext parses `#[init(val = expr)]` as an attribute, so a top-level comma in `expr` would be
/// read as an attribute argument separator. Paren-wrap any such expr (commas inside `()`/`[]`/
/// `{}`/`<>`/`|...|` are already shielded).
fn paren_wrap(expr: &Expr) -> TokenStream2 {
    if has_top_level_comma(expr.to_token_stream()) {
        quote!((#expr))
    } else {
        quote!(#expr)
    }
}

fn has_top_level_comma(ts: TokenStream2) -> bool {
    let mut angle = 0i32;
    let mut in_pipe = false;
    for tt in ts {
        if let TokenTree::Punct(p) = tt {
            match p.as_char() {
                ',' if angle == 0 && !in_pipe => return true,
                '<' => angle += 1,
                '>' if angle > 0 => angle -= 1,
                '|' => in_pipe = !in_pipe,
                _ => {}
            }
        }
    }
    false
}

/// Collect the component idents named in sibling `#[require(...)]` attributes.
fn collect_require_idents(attrs: &[Attribute]) -> HashSet<String> {
    let mut set = HashSet::new();
    for attr in attrs {
        if !attr.path().is_ident("require") {
            continue;
        }
        if let Ok(entries) =
            attr.parse_args_with(Punctuated::<RequireEntry, Token![,]>::parse_terminated)
        {
            for entry in entries {
                if let Some(last) = entry.0.segments.last() {
                    set.insert(last.ident.to_string());
                }
            }
        }
    }
    set
}

/// One `#[require(...)]` entry: a component path, ignoring any trailing `= expr` / `(args)`.
struct RequireEntry(Path);

impl Parse for RequireEntry {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let path: Path = input.parse()?;
        while !input.is_empty() && !input.peek(Token![,]) {
            input.parse::<TokenTree>()?;
        }
        Ok(RequireEntry(path))
    }
}

#[cfg(test)]
mod tests {
    use syn::parse_quote;

    #[test]
    fn cf_generates_class_companions_and_required_registration() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(Component, GodotNode, Default)]
            #[gdbevy(base = CharacterBody2D, class_name = Player2D)]
            #[gdbevy(require(speed: Speed, as = f32, default = 250.0), require(Stunned))]
            struct Player;
        };
        let out = crate::godot_node::derive_godot_node_component(di)
            .unwrap()
            .to_string();
        assert!(out.contains("# [class (base = CharacterBody2D"));
        assert!(out.contains("pub struct Player2D"));
        assert!(out.contains("# [export]") && out.contains("speed : f32"));
        assert!(out.contains("# [init (val = 250.0"));
        assert!(out.contains("try_register_required_components_with"));
        assert!(
            out.contains("try_register_required_components ::")
                || out.contains("try_register_required_components <")
        );
        assert!(out.contains("GodotRequiredComponents"));
        assert!(out.contains("AutoSyncBundleRegistry"));
        assert!(out.contains("Stunned :: default ()"));
        assert!(!out.contains("bevy_bundle"));
    }

    #[test]
    fn gf_emits_insert_and_no_class() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(GodotClass, BevyComponents)]
            #[gdbevy(require(Player))]
            struct PlayerNode {
                base: Base<Node2D>,
                #[gdbevy(component = Speed, with = to_speed)]
                #[export] speed: f32,
            }
        };
        let out = crate::godot_node::derive_bevy_components(di)
            .unwrap()
            .to_string();
        assert!(!out.contains("# [class (base")); // user owns the class; we do NOT generate it
        assert!(out.contains("AutoSyncBundleRegistry"));
        assert!(out.contains("Speed (to_speed (node . bind () . speed . clone ()))"));
        assert!(out.contains("Player :: default ()"));
        assert!(!out.contains("GodotRequiredComponents")); // GF has no trigger
        assert!(!out.contains("bevy_bundle"));
    }

    #[test]
    fn cf_skips_companion_already_in_sibling_require() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(Component, GodotNode, Default)]
            #[require(Stunned)]
            #[gdbevy(require(Stunned), require(speed: Speed, as = f32))]
            struct Player;
        };
        let out = crate::godot_node::derive_godot_node_component(di)
            .unwrap()
            .to_string();
        assert!(
            !out.contains("try_register_required_components :: < Player , Stunned >")
                && !out.contains("< Player , Stunned >")
        );
        assert!(out.contains("try_register_required_components_with"));
    }
}
