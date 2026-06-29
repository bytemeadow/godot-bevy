use quote::format_ident;
use std::collections::HashSet;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{
    Attribute, Data, DeriveInput, Error, Expr, Field, Fields, Ident, Meta, Path, Token, Type,
    braced, parenthesized, parse_quote,
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
    pub as_type: Option<syn::Type>,
    pub default: Option<syn::Expr>,
    pub with: Option<syn::Path>,
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
                    d.default = Some(input.parse()?);
                }
                "with" => {
                    input.parse::<Token![=]>()?;
                    d.with = Some(input.parse()?);
                }
                "component" => {
                    input.parse::<Token![=]>()?;
                    d.component = Some(input.parse()?);
                }
                _ => {
                    return Err(Error::new(
                        key.span(),
                        format!(
                            "unknown key `{name}`; expected `as`, `default`, `with`, or `component`"
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
    Ok(d)
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

/// Struct-level `#[bevy(...)]` directives: `base`, `class_name`, and `require(...)`.
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
        if !attr.path().is_ident("bevy") {
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
            Fields::Unnamed(_) => Err(Error::new_spanned(
                input,
                "tuple structs are not supported; use a named-field or unit struct",
            )),
        },
        _ => Err(Error::new_spanned(input, "expected a struct")),
    }
}

fn find_bevy_attr(field: &Field) -> Option<&Attribute> {
    field.attrs.iter().find(|a| a.path().is_ident("bevy"))
}

fn parse_field_directives(attr: &Attribute) -> syn::Result<Directives> {
    match &attr.meta {
        Meta::Path(_) => Ok(Directives::default()),
        Meta::List(_) => attr.parse_args_with(parse_directives),
        Meta::NameValue(nv) => Err(Error::new_spanned(nv, "expected `#[bevy(...)]`")),
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
                    as_type: Some(as_type),
                    default: cfg.default,
                    with: cfg.with,
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
                    as_type: Some(as_type),
                    default: cfg.default,
                    with: cfg.with,
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
                    as_type: None,
                    default: None,
                    with: None,
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
    for field in struct_fields(input)? {
        let Some(attr) = find_bevy_attr(field) else {
            continue;
        };
        let name = field.ident.clone().unwrap();
        let d = parse_field_directives(attr)?;
        if d.component.is_some() {
            return Err(Error::new_spanned(
                attr,
                "`component` is not valid on a component-first field; it is for Godot-first field bindings",
            ));
        }
        out.push(Mapping {
            godot_prop: name.clone(),
            bevy_field: Some(name),
            as_type: d.as_type,
            default: d.default,
            with: d.with,
        });
    }
    Ok(out)
}

fn collect_field_bindings(input: &DeriveInput) -> syn::Result<Vec<ComponentPlan>> {
    let mut out = Vec::new();
    for field in struct_fields(input)? {
        let Some(attr) = find_bevy_attr(field) else {
            continue;
        };
        let name = field.ident.clone().unwrap();
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
                as_type: None,
                default: None,
                with: d.with,
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

#[cfg(test)]
mod tests {
    use super::*;
    use syn::parse_quote;

    #[test]
    fn cf_marker_and_newtype_companions() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(Component, GodotNode, Default)]
            #[bevy(base = CharacterBody2D, class_name = Player2D)]
            #[bevy(require(speed: Speed, as = f32, default = 250.0), require(Stunned))]
            struct Player;
        };
        let plan = parse_component_first(&di).unwrap();
        assert!(plan.emit_node_class);
        assert_eq!(plan.base.to_string(), "CharacterBody2D");
        assert_eq!(plan.godot_class.to_string(), "Player2D");
        assert_eq!(plan.companions.len(), 2);
        assert!(plan.companions[0].generated_exports);
        assert_eq!(
            plan.companions[0].path.get_ident().unwrap().to_string(),
            "Speed"
        );
        assert_eq!(
            plan.companions[1].path.get_ident().unwrap().to_string(),
            "Stunned"
        );
        assert!(matches!(plan.companions[1].init, ComponentInit::Marker));
        match &plan.companions[0].init {
            ComponentInit::Newtype(m) => {
                assert_eq!(m.godot_prop.to_string(), "speed");
                assert!(m.bevy_field.is_none());
                assert!(m.as_type.is_some());
            }
            _ => panic!("expected newtype companion"),
        }
        assert_eq!(plan.primary.path.get_ident().unwrap().to_string(), "Player");
    }

    #[test]
    fn cf_primary_field_default() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(Component, GodotNode, Default)]
            #[bevy(base = Area2D, class_name = Door2D)]
            struct Door { #[bevy(default = LevelId::Level1)] level_id: LevelId }
        };
        let plan = parse_component_first(&di).unwrap();
        assert_eq!(plan.primary.fields.len(), 1);
        assert!(plan.primary.fields[0].default.is_some());
        assert!(plan.primary.fields[0].as_type.is_none());
        assert!(plan.primary.fields[0].with.is_none());
        assert_eq!(plan.primary.fields[0].godot_prop.to_string(), "level_id");
        assert_eq!(
            plan.primary.fields[0]
                .bevy_field
                .as_ref()
                .unwrap()
                .to_string(),
            "level_id"
        );
    }

    #[test]
    fn gf_field_binding() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(GodotClass, BevyComponents)]
            #[bevy(require(Player))]
            struct PlayerNode {
                base: Base<Node2D>,
                #[bevy(component = Speed, with = to_speed)]
                #[export] speed: f32,
            }
        };
        let plan = parse_godot_first(&di).unwrap();
        assert!(!plan.emit_node_class);
        assert!(plan.trigger.is_none());
        assert!(plan.primary.fields.is_empty());
        assert!(plan.primary.path.segments.is_empty());
        assert_eq!(plan.companions.len(), 2);
        assert_eq!(
            plan.companions[0].path.get_ident().unwrap().to_string(),
            "Player"
        );
        assert!(matches!(plan.companions[0].init, ComponentInit::Marker));
        let speed = &plan.companions[1];
        assert_eq!(speed.path.get_ident().unwrap().to_string(), "Speed");
        assert!(!speed.generated_exports);
        match &speed.init {
            ComponentInit::Newtype(m) => {
                assert_eq!(m.godot_prop.to_string(), "speed");
                assert!(m.bevy_field.is_none());
                assert_eq!(
                    m.with.as_ref().unwrap().get_ident().unwrap().to_string(),
                    "to_speed"
                );
            }
            _ => panic!("expected newtype field binding"),
        }
    }

    #[test]
    fn cf_as_missing_on_companion() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(Component, GodotNode)]
            #[bevy(require(speed: Speed, default = 250.0))]
            struct Player;
        };
        assert!(
            parse_component_first(&di)
                .unwrap_err()
                .to_string()
                .contains("requires `as")
        );
    }

    #[test]
    fn cf_duplicate_export_prop() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(Component, GodotNode)]
            #[bevy(require(speed: Speed, as = f32), require(speed: Boost, as = f32))]
            struct Player;
        };
        assert!(
            parse_component_first(&di)
                .unwrap_err()
                .to_string()
                .contains("duplicate")
        );
    }

    #[test]
    fn cf_newtype_struct_mix_in_one_require() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(Component, GodotNode)]
            #[bevy(require(stats: Stats { current(as = i32) }, default = 5))]
            struct Player;
        };
        assert!(
            parse_component_first(&di)
                .unwrap_err()
                .to_string()
                .contains("cannot mix")
        );
    }

    #[test]
    fn class_name_equals_component() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(Component, GodotNode)]
            #[bevy(class_name = Player)]
            struct Player;
        };
        assert!(
            parse_component_first(&di)
                .unwrap_err()
                .to_string()
                .contains("class_name")
        );
    }

    #[test]
    fn gf_as_on_field_binding() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(GodotClass, BevyComponents)]
            struct PlayerNode {
                base: Base<Node2D>,
                #[bevy(component = Speed, as = f32)]
                #[export] speed: f32,
            }
        };
        assert!(
            parse_godot_first(&di)
                .unwrap_err()
                .to_string()
                .contains("`as`")
        );
    }

    #[test]
    fn gf_default_on_field_binding() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(GodotClass, BevyComponents)]
            struct PlayerNode {
                base: Base<Node2D>,
                #[bevy(component = Speed, default = 5.0)]
                #[export] speed: f32,
            }
        };
        assert!(
            parse_godot_first(&di)
                .unwrap_err()
                .to_string()
                .contains("`default`")
        );
    }

    #[test]
    fn gf_missing_component_key() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(GodotClass, BevyComponents)]
            struct PlayerNode {
                base: Base<Node2D>,
                #[bevy(with = to_speed)]
                #[export] speed: f32,
            }
        };
        assert!(
            parse_godot_first(&di)
                .unwrap_err()
                .to_string()
                .contains("component")
        );
    }

    #[test]
    fn gf_struct_level_generated_export() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(GodotClass, BevyComponents)]
            #[bevy(require(speed: Speed, as = f32))]
            struct PlayerNode { base: Base<Node2D> }
        };
        assert!(
            parse_godot_first(&di)
                .unwrap_err()
                .to_string()
                .contains("Godot-first")
        );
    }

    #[test]
    fn base_or_class_name_on_gf() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(GodotClass, BevyComponents)]
            #[bevy(base = Node2D)]
            struct PlayerNode { base: Base<Node2D> }
        };
        assert!(
            parse_godot_first(&di)
                .unwrap_err()
                .to_string()
                .contains("component-first")
        );
    }

    #[test]
    fn sync_key_is_reserved() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(Component, GodotNode)]
            #[bevy(require(speed: Speed, as = f32, sync = two_way))]
            struct Player;
        };
        assert!(
            parse_component_first(&di)
                .unwrap_err()
                .to_string()
                .contains("not yet available")
        );
    }

    #[test]
    fn into_key_is_reserved() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(Component, GodotNode)]
            #[bevy(require(speed: Speed, as = f32, into = Foo))]
            struct Player;
        };
        assert!(
            parse_component_first(&di)
                .unwrap_err()
                .to_string()
                .contains("not yet available")
        );
    }
}
