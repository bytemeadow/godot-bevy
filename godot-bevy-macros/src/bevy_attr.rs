use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::HashSet;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    Attribute, Data, DeriveInput, Error, Expr, Field, Fields, Ident, LitStr, Meta, Path, Token,
    Type, braced, parenthesized, parse_quote,
};

/// The Godot class + Bevy components a single derive expands to.
///
/// Two front-ends share this IR: component-first (`GodotNode`, which generates the
/// Godot class) and Godot-first (`BevyComponents`, which annotates the user's class).
pub struct ClassPlan {
    pub godot_class: syn::Ident,
    pub base: syn::Ident,
    pub emit_node_class: bool,
    pub trigger: Option<syn::Path>,
    pub primary: PrimaryPlan,
    pub companions: Vec<ComponentPlan>,
}

pub struct PrimaryPlan {
    pub path: syn::Path,
    pub fields: Vec<Mapping>,
}

pub struct ComponentPlan {
    pub path: syn::Path,
    pub generated_exports: bool,
    pub init: ComponentInit,
}

// Variant sizes differ (Mapping is wide), but boxing would change the IR shape.
#[allow(clippy::large_enum_variant)]
pub enum ComponentInit {
    Marker,
    Newtype(Mapping),
    Fields(Vec<Mapping>),
}

pub struct Mapping {
    pub godot_prop: syn::Ident,
    pub bevy_field: Option<syn::Ident>,
    pub tuple_index: Option<usize>,
    pub as_type: Option<syn::Type>,
    pub default: Option<syn::Expr>,
    pub with: Option<syn::Path>,
    pub docs: Vec<Attribute>,
    pub description: Option<LitStr>,
    pub hint: Option<Ident>,
    pub hint_string: Option<Expr>,
}

// Summary Debug so tests can `.unwrap_err()` on `Result<ClassPlan, _>`;
// syn types only implement Debug under the `extra-traits` feature.
impl std::fmt::Debug for ClassPlan {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "ClassPlan({}, emit_node_class={}, {} companions)",
            self.godot_class,
            self.emit_node_class,
            self.companions.len()
        )
    }
}

/// `key = value` directives shared by `require(...)` entries and field attributes.
/// Which keys are legal depends on the front-end; the parser fills every key it
/// understands and the per-front-end validators reject illegal combinations.
#[derive(Default)]
struct Directives {
    as_type: Option<Type>,
    default: Option<Expr>,
    with: Option<Path>,
    component: Option<Path>,
    export: bool,
    description: Option<LitStr>,
    hint: Option<Ident>,
    hint_string: Option<Expr>,
}

fn parse_directives(input: ParseStream) -> syn::Result<Directives> {
    let mut d = Directives::default();
    while !input.is_empty() {
        if input.peek(Token![as]) {
            let kw: Token![as] = input.parse()?;
            input.parse::<Token![=]>()?;
            if d.as_type.is_some() {
                return Err(Error::new(kw.span(), "duplicate `as`"));
            }
            d.as_type = Some(input.parse()?);
        } else {
            let key: Ident = input.parse()?;
            let name = key.to_string();
            match name.as_str() {
                // Reserved for the deferred sync minor; reject explicitly.
                "sync" | "into" => {
                    return Err(Error::new(
                        key.span(),
                        format!("`{name}` is reserved and not yet available"),
                    ));
                }
                "default" => {
                    input.parse::<Token![=]>()?;
                    if d.default.is_some() {
                        return Err(Error::new(key.span(), "duplicate `default`"));
                    }
                    d.default = Some(input.parse()?);
                }
                "with" => {
                    input.parse::<Token![=]>()?;
                    if d.with.is_some() {
                        return Err(Error::new(key.span(), "duplicate `with`"));
                    }
                    d.with = Some(input.parse()?);
                }
                "description" => {
                    input.parse::<Token![=]>()?;
                    if d.description.is_some() {
                        return Err(Error::new(key.span(), "duplicate `description`"));
                    }
                    d.description = Some(input.parse()?);
                }
                "hint" => {
                    input.parse::<Token![=]>()?;
                    if d.hint.is_some() {
                        return Err(Error::new(key.span(), "duplicate `hint`"));
                    }
                    d.hint = Some(input.parse()?);
                }
                "hint_string" => {
                    input.parse::<Token![=]>()?;
                    if d.hint_string.is_some() {
                        return Err(Error::new(key.span(), "duplicate `hint_string`"));
                    }
                    d.hint_string = Some(input.parse()?);
                }
                "component" => {
                    input.parse::<Token![=]>()?;
                    if d.component.is_some() {
                        return Err(Error::new(key.span(), "duplicate `component`"));
                    }
                    d.component = Some(input.parse()?);
                }
                "export" => {
                    if d.export {
                        return Err(Error::new(key.span(), "duplicate `export`"));
                    }
                    d.export = true;
                }
                _ => {
                    return Err(Error::new(
                        key.span(),
                        format!(
                            "unknown key `{name}`; expected `as`, `default`, `with`, `description`, `hint`, `hint_string`, `component`, or `export`"
                        ),
                    ));
                }
            }
        }
        if input.peek(Token![,]) {
            input.parse::<Token![,]>()?;
        } else {
            break;
        }
    }
    if d.hint_string.is_some() && d.hint.is_none() {
        return Err(Error::new(
            input.span(),
            "`hint_string` requires `hint` to also be provided",
        ));
    }
    Ok(d)
}

fn doc_attributes(field: &Field) -> Vec<Attribute> {
    field
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("doc"))
        .cloned()
        .collect()
}

/// The syntactic shape of one `require(...)` entry, before front-end validation.
enum RawRequire {
    /// `(Comp)`
    Marker { component: Path },
    /// `(prop: Comp, as = T, ...)` — generated single-property export (component-first).
    /// `cfg` is boxed to keep the enum variants similar in size (clippy::large_enum_variant).
    Newtype {
        prop: Ident,
        component: Path,
        cfg: Box<Directives>,
    },
    /// `(prop: Comp { field(as = T, ...), ... })` — generated multi-property export.
    Struct {
        component: Path,
        fields: Vec<(Ident, Directives)>,
    },
    /// `(Comp { bevy_field: godot_field, ... })` — bind existing Godot props (Godot-first).
    Binding {
        component: Path,
        pairs: Vec<(Ident, Ident)>,
    },
}

fn parse_one_require(input: ParseStream) -> syn::Result<RawRequire> {
    let first: Path = input.parse()?;

    // `prop: Comp ...` — a single colon (not the `::` path separator) marks the
    // generated-export forms, where `first` is the property name.
    if input.peek(Token![:]) && !input.peek(Token![::]) {
        let prop = first
            .get_ident()
            .cloned()
            .ok_or_else(|| Error::new_spanned(&first, "expected a single identifier before `:`"))?;
        input.parse::<Token![:]>()?;
        let component: Path = input.parse()?;

        if input.peek(syn::token::Brace) {
            let content;
            braced!(content in input);
            let mut fields = Vec::new();
            while !content.is_empty() {
                let fname: Ident = content.parse()?;
                let cfg_content;
                parenthesized!(cfg_content in content);
                let cfg = parse_directives(&cfg_content)?;
                fields.push((fname, cfg));
                if content.peek(Token![,]) {
                    content.parse::<Token![,]>()?;
                }
            }
            if !input.is_empty() {
                return Err(input
                    .error("cannot mix struct fields and newtype config in one `require(...)`"));
            }
            Ok(RawRequire::Struct { component, fields })
        } else {
            let cfg = if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
                parse_directives(input)?
            } else {
                Directives::default()
            };
            if !input.is_empty() {
                return Err(input
                    .error("cannot mix struct fields and newtype config in one `require(...)`"));
            }
            Ok(RawRequire::Newtype {
                prop,
                component,
                cfg: Box::new(cfg),
            })
        }
    } else if input.peek(syn::token::Brace) {
        let content;
        braced!(content in input);
        let mut pairs = Vec::new();
        while !content.is_empty() {
            let bevy_field: Ident = content.parse()?;
            content.parse::<Token![:]>()?;
            let godot_field: Ident = content.parse()?;
            pairs.push((bevy_field, godot_field));
            if content.peek(Token![,]) {
                content.parse::<Token![,]>()?;
            }
        }
        Ok(RawRequire::Binding {
            component: first,
            pairs,
        })
    } else {
        if !input.is_empty() {
            return Err(input.error("unexpected tokens in `require(...)`"));
        }
        Ok(RawRequire::Marker { component: first })
    }
}

/// Struct-level `#[gdbevy(...)]` directives: `base`, `class_name`, and `require(...)`.
#[derive(Default)]
struct StructLevel {
    base: Option<Ident>,
    class_name: Option<Ident>,
    requires: Vec<RawRequire>,
}

impl Parse for StructLevel {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut sl = StructLevel::default();
        while !input.is_empty() {
            let key: Ident = input.parse()?;
            if key == "require" {
                let content;
                parenthesized!(content in input);
                let entry = parse_one_require(&content)?;
                if !content.is_empty() {
                    return Err(content.error("unexpected tokens after `require(...)` entry"));
                }
                sl.requires.push(entry);
            } else if key == "base" {
                input.parse::<Token![=]>()?;
                sl.base = Some(input.parse()?);
            } else if key == "class_name" {
                input.parse::<Token![=]>()?;
                sl.class_name = Some(input.parse()?);
            } else {
                return Err(Error::new(
                    key.span(),
                    format!(
                        "unknown key `{key}`; expected `base`, `class_name`, or `require(...)`"
                    ),
                ));
            }
            if input.peek(Token![,]) {
                input.parse::<Token![,]>()?;
            } else {
                break;
            }
        }
        Ok(sl)
    }
}

fn collect_struct_level(input: &DeriveInput) -> syn::Result<StructLevel> {
    let mut acc = StructLevel::default();
    for attr in &input.attrs {
        if !attr.path().is_ident("gdbevy") {
            continue;
        }
        let sl: StructLevel = attr.parse_args()?;
        if sl.base.is_some() {
            acc.base = sl.base;
        }
        if sl.class_name.is_some() {
            acc.class_name = sl.class_name;
        }
        acc.requires.extend(sl.requires);
    }
    Ok(acc)
}

fn struct_fields(input: &DeriveInput) -> syn::Result<Vec<&Field>> {
    match &input.data {
        Data::Struct(s) => match &s.fields {
            Fields::Named(n) => Ok(n.named.iter().collect()),
            Fields::Unit => Ok(Vec::new()),
            Fields::Unnamed(u) => Ok(u.unnamed.iter().collect()),
        },
        _ => Err(Error::new_spanned(input, "expected a struct")),
    }
}

fn find_bevy_attr(field: &Field) -> Option<&Attribute> {
    field.attrs.iter().find(|a| a.path().is_ident("gdbevy"))
}

fn parse_field_directives(attr: &Attribute) -> syn::Result<Directives> {
    match &attr.meta {
        Meta::Path(_) => Ok(Directives::default()),
        Meta::List(_) => attr.parse_args_with(parse_directives),
        Meta::NameValue(nv) => Err(Error::new_spanned(nv, "expected `#[gdbevy(...)]`")),
    }
}

fn empty_path() -> Path {
    Path {
        leading_colon: None,
        segments: Punctuated::new(),
    }
}

fn cf_companion(raw: RawRequire) -> syn::Result<ComponentPlan> {
    match raw {
        RawRequire::Marker { component } => Ok(ComponentPlan {
            path: component,
            generated_exports: false,
            init: ComponentInit::Marker,
        }),
        RawRequire::Newtype {
            prop,
            component,
            cfg,
        } => {
            let cfg = *cfg;
            if cfg.component.is_some() {
                return Err(Error::new_spanned(
                    &component,
                    "`component` is not valid inside `require(...)`",
                ));
            }
            let Some(as_type) = cfg.as_type else {
                return Err(Error::new(
                    prop.span(),
                    format!("generated export `{prop}` requires `as = <Type>`"),
                ));
            };
            Ok(ComponentPlan {
                path: component,
                generated_exports: true,
                init: ComponentInit::Newtype(Mapping {
                    godot_prop: prop,
                    bevy_field: None,
                    tuple_index: None,
                    as_type: Some(as_type),
                    default: cfg.default,
                    with: cfg.with,
                    docs: Vec::new(),
                    description: cfg.description,
                    hint: cfg.hint,
                    hint_string: cfg.hint_string,
                }),
            })
        }
        RawRequire::Struct { component, fields } => {
            let mut mappings = Vec::new();
            for (fname, cfg) in fields {
                if cfg.component.is_some() {
                    return Err(Error::new_spanned(
                        &fname,
                        "`component` is not valid inside `require(...)`",
                    ));
                }
                let Some(as_type) = cfg.as_type else {
                    return Err(Error::new(
                        fname.span(),
                        format!("generated export `{fname}` requires `as = <Type>`"),
                    ));
                };
                mappings.push(Mapping {
                    godot_prop: fname.clone(),
                    bevy_field: Some(fname),
                    tuple_index: None,
                    as_type: Some(as_type),
                    default: cfg.default,
                    with: cfg.with,
                    docs: Vec::new(),
                    description: cfg.description,
                    hint: cfg.hint,
                    hint_string: cfg.hint_string,
                });
            }
            Ok(ComponentPlan {
                path: component,
                generated_exports: true,
                init: ComponentInit::Fields(mappings),
            })
        }
        RawRequire::Binding { component, .. } => Err(Error::new_spanned(
            &component,
            "the `Comp { bevy: godot }` binding form is Godot-first only",
        )),
    }
}

fn gf_companion(raw: RawRequire) -> syn::Result<ComponentPlan> {
    match raw {
        RawRequire::Marker { component } => Ok(ComponentPlan {
            path: component,
            generated_exports: false,
            init: ComponentInit::Marker,
        }),
        RawRequire::Newtype { prop, .. } => Err(Error::new(
            prop.span(),
            "generated-export `require(prop: Comp, ...)` entries are not supported in Godot-first",
        )),
        RawRequire::Struct { component, .. } => Err(Error::new_spanned(
            &component,
            "generated-export `require(prop: Comp { ... })` entries are not supported in Godot-first",
        )),
        RawRequire::Binding { component, pairs } => {
            let mappings = pairs
                .into_iter()
                .map(|(bevy_field, godot_field)| Mapping {
                    godot_prop: godot_field,
                    bevy_field: Some(bevy_field),
                    tuple_index: None,
                    as_type: None,
                    default: None,
                    with: None,
                    docs: Vec::new(),
                    description: None,
                    hint: None,
                    hint_string: None,
                })
                .collect();
            Ok(ComponentPlan {
                path: component,
                generated_exports: false,
                init: ComponentInit::Fields(mappings),
            })
        }
    }
}

fn collect_primary_fields(input: &DeriveInput) -> syn::Result<Vec<Mapping>> {
    let mut out = Vec::new();
    for (i, field) in struct_fields(input)?.into_iter().enumerate() {
        let Some(attr) = find_bevy_attr(field) else {
            continue;
        };
        let (godot_prop, bevy_field, tuple_index) = match &field.ident {
            Some(ident) => (ident.clone(), Some(ident.clone()), None),
            None => (format_ident!("value{i}"), None, Some(i)),
        };
        let d = parse_field_directives(attr)?;
        if d.component.is_some() {
            return Err(Error::new_spanned(
                attr,
                "`component` is not valid on a component-first field; it is for Godot-first field bindings",
            ));
        }
        if !d.export {
            return Err(Error::new_spanned(
                attr,
                "component-first field attributes require `export`, e.g. `#[gdbevy(export)]`",
            ));
        }
        out.push(Mapping {
            godot_prop,
            bevy_field,
            tuple_index,
            as_type: d.as_type,
            default: d.default,
            with: d.with,
            docs: doc_attributes(field),
            description: d.description,
            hint: d.hint,
            hint_string: d.hint_string,
        });
    }
    Ok(out)
}

fn collect_field_bindings(input: &DeriveInput) -> syn::Result<Vec<ComponentPlan>> {
    if matches!(&input.data, Data::Struct(s) if matches!(s.fields, Fields::Unnamed(_))) {
        return Err(Error::new_spanned(
            input,
            "tuple structs are only supported for component-first `GodotNode`",
        ));
    }

    let mut out = Vec::new();
    for field in struct_fields(input)? {
        let Some(attr) = find_bevy_attr(field) else {
            continue;
        };
        let name = field
            .ident
            .clone()
            .expect("tuple structs are rejected above");
        let d = parse_field_directives(attr)?;
        if d.as_type.is_some() {
            return Err(Error::new_spanned(
                attr,
                "`as` is not allowed on a Godot-first field binding",
            ));
        }
        if d.default.is_some() {
            return Err(Error::new_spanned(
                attr,
                "`default` is not allowed on a Godot-first field binding",
            ));
        }
        if d.export {
            return Err(Error::new_spanned(
                attr,
                "`export` is not valid on a Godot-first field binding",
            ));
        }
        if d.description.is_some() || d.hint.is_some() || d.hint_string.is_some() {
            return Err(Error::new_spanned(
                attr,
                "`description`, `hint`, and `hint_string` are only valid on generated component-first exports",
            ));
        }
        let Some(component) = d.component else {
            return Err(Error::new_spanned(
                attr,
                "a Godot-first field binding requires `component = <Component>`",
            ));
        };
        out.push(ComponentPlan {
            path: component,
            generated_exports: false,
            init: ComponentInit::Newtype(Mapping {
                godot_prop: name,
                bevy_field: None,
                tuple_index: None,
                as_type: None,
                default: None,
                with: d.with,
                docs: Vec::new(),
                description: None,
                hint: None,
                hint_string: None,
            }),
        });
    }
    Ok(out)
}

fn check_duplicate_props(primary: &PrimaryPlan, companions: &[ComponentPlan]) -> syn::Result<()> {
    let mut props: Vec<&Ident> = primary.fields.iter().map(|m| &m.godot_prop).collect();
    for c in companions {
        if !c.generated_exports {
            continue;
        }
        match &c.init {
            ComponentInit::Newtype(m) => props.push(&m.godot_prop),
            ComponentInit::Fields(ms) => props.extend(ms.iter().map(|m| &m.godot_prop)),
            ComponentInit::Marker => {}
        }
    }
    let mut seen = HashSet::new();
    for ident in props {
        if !seen.insert(ident.to_string()) {
            return Err(Error::new(
                ident.span(),
                format!("duplicate Godot property `{ident}`"),
            ));
        }
    }
    Ok(())
}

pub fn parse_component_first(input: &DeriveInput) -> syn::Result<ClassPlan> {
    let sl = collect_struct_level(input)?;
    let struct_ident = input.ident.clone();
    let base = sl.base.unwrap_or_else(|| parse_quote!(Node));
    let godot_class = sl
        .class_name
        .unwrap_or_else(|| format_ident!("{}BevyComponent", struct_ident));
    if godot_class == struct_ident {
        return Err(Error::new(
            godot_class.span(),
            "`class_name` cannot be the same as the component name",
        ));
    }

    let companions = sl
        .requires
        .into_iter()
        .map(cf_companion)
        .collect::<syn::Result<Vec<_>>>()?;
    let primary = PrimaryPlan {
        path: struct_ident.clone().into(),
        fields: collect_primary_fields(input)?,
    };
    check_duplicate_props(&primary, &companions)?;

    Ok(ClassPlan {
        godot_class,
        base,
        emit_node_class: true,
        trigger: Some(struct_ident.into()),
        primary,
        companions,
    })
}

pub fn parse_godot_first(input: &DeriveInput) -> syn::Result<ClassPlan> {
    let sl = collect_struct_level(input)?;
    if let Some(base) = &sl.base {
        return Err(Error::new(
            base.span(),
            "`base`/`class_name` are only valid in component-first (`GodotNode`)",
        ));
    }
    if let Some(class_name) = &sl.class_name {
        return Err(Error::new(
            class_name.span(),
            "`base`/`class_name` are only valid in component-first (`GodotNode`)",
        ));
    }

    let mut companions = sl
        .requires
        .into_iter()
        .map(gf_companion)
        .collect::<syn::Result<Vec<_>>>()?;
    // field-existence validation is deferred to the compiler
    companions.extend(collect_field_bindings(input)?);

    Ok(ClassPlan {
        godot_class: input.ident.clone(),
        base: parse_quote!(Node),
        emit_node_class: false,
        trigger: None,
        primary: PrimaryPlan {
            path: empty_path(),
            fields: Vec::new(),
        },
        companions,
    })
}

/// Parses the `#[derive(AttachableComponent)]` macro input.
///
/// Extracts the `#[gdbevy(target = YourBevyComponent)]` attribute and generates
/// a function that converts the Godot class into the specified Bevy component,
/// registering it with the `AttachComponentRegistry` via `inventory::submit!`.
pub fn parse_attachable_component(input: &DeriveInput) -> syn::Result<TokenStream> {
    let class = &input.ident;

    let mut target_type: Option<Path> = None;

    for attr in &input.attrs {
        if attr.path().is_ident("gdbevy") {
            attr.parse_nested_meta(|meta| {
                if meta.path.is_ident("target") {
                    target_type = Some(meta.value()?.parse()?);
                    Ok(())
                } else {
                    Err(meta.error("unsupported property, expected `target`"))
                }
            })?
        }
    }

    let target = target_type.ok_or_else(|| {
        Error::new(
            input.ident.span(),
            "Missing #[gdbevy(target = YourBevyComponent)] attribute",
        )
    })?;

    let sync_fn_name = format_ident!("__{}_attach_component_fn", class.to_string().to_lowercase());

    let expanded = quote! {
        fn #sync_fn_name(
            commands: &mut godot_bevy::bevy_ecs::system::Commands,
            parent: godot_bevy::bevy_ecs::entity::Entity,
            godot: &mut godot_bevy::interop::GodotAccess,
            handle: godot_bevy::interop::GodotNodeHandle,
        ) -> bool {
            if let Some(mut gd) = godot.try_get::<#class>(handle) {
                let bevy_component = <#target as ::core::convert::From<_>>::from(::core::ops::Deref::deref(&gd.bind()));
                commands.entity(parent).insert(bevy_component);
                true
            } else {
                false
            }
        }

        godot_bevy::inventory::submit! {
            godot_bevy::prelude::AttachComponentRegistry {
                godot_class_name: stringify!(#class),
                godot_class_id_fn: || <#class as ::godot::prelude::GodotClass>::class_id(),
                attach_component_fn: #sync_fn_name,
            }
        };
    };

    Ok(expanded)
}

#[cfg(test)]
include!("bevy_attr_tests.rs");
