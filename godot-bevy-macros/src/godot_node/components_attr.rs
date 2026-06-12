use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::{Error, Expr, Ident, Path, Token, Type, braced, parenthesized};

/// Per-export configuration shared by newtype entries and struct-entry fields.
#[derive(Clone)]
pub struct ExportConfig {
    pub export_type: Option<Type>,
    pub default_expr: Option<Expr>,
    pub transform_with: Option<Path>,
}

/// One entry of `#[godot_components(...)]`:
/// - `(Comp)` — marker companion, no Godot export
/// - `prop(Comp, export_type(T), default(expr), transform_with(path))` — newtype companion
/// - `prop(Comp { field(export_type(T), ...), ... })` — struct companion
pub enum CompanionEntry {
    Marker {
        component: Path,
    },
    Newtype {
        prop: Ident,
        component: Path,
        // Boxed: ExportConfig is large (three Option<syn> fields), and boxing it
        // keeps CompanionEntry's variants similarly sized (clippy::large_enum_variant).
        config: Box<ExportConfig>,
    },
    Struct {
        component: Path,
        fields: Vec<(Ident, ExportConfig)>,
    },
}

pub struct GodotComponentsAttr {
    pub entries: Vec<CompanionEntry>,
}

// Debug impl required so tests can call `.unwrap()` on `Result<GodotComponentsAttr, _>`.
impl std::fmt::Debug for GodotComponentsAttr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "GodotComponentsAttr({} entries)", self.entries.len())
    }
}

fn parse_config(input: ParseStream) -> syn::Result<ExportConfig> {
    let mut config = ExportConfig {
        export_type: None,
        default_expr: None,
        transform_with: None,
    };

    while !input.is_empty() {
        let key: Ident = input.parse()?;
        let content;
        parenthesized!(content in input);

        if key == "export_type" {
            if config.export_type.is_some() {
                return Err(Error::new(key.span(), "Duplicate export_type(..)"));
            }
            config.export_type = Some(content.parse()?);
        } else if key == "default" {
            if config.default_expr.is_some() {
                return Err(Error::new(key.span(), "Duplicate default(..)"));
            }
            config.default_expr = Some(content.parse()?);
        } else if key == "transform_with" {
            if config.transform_with.is_some() {
                return Err(Error::new(key.span(), "Duplicate transform_with(..)"));
            }
            config.transform_with = Some(content.parse()?);
        } else {
            return Err(Error::new(
                key.span(),
                "Unknown key. Expected export_type(..), transform_with(..), or default(..)",
            ));
        }

        if input.peek(Token![,]) {
            let _comma: Token![,] = input.parse()?;
        }
    }

    Ok(config)
}

impl Parse for CompanionEntry {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        // Marker form: `(Comp)`
        if input.peek(syn::token::Paren) {
            let content;
            parenthesized!(content in input);
            let component: Path = content.parse()?;
            if !content.is_empty() {
                return Err(Error::new(
                    content.span(),
                    "Marker entries take only a component path: (Comp)",
                ));
            }
            return Ok(CompanionEntry::Marker { component });
        }

        let prop: Ident = input.parse()?;
        let content;
        parenthesized!(content in input);
        let component: Path = content.parse()?;

        // Struct form: `prop(Comp { field(..), .. })`
        if content.peek(syn::token::Brace) {
            let fields_content;
            braced!(fields_content in content);

            let mut fields = Vec::new();
            while !fields_content.is_empty() {
                let field_name: Ident = fields_content.parse()?;
                let config_content;
                parenthesized!(config_content in fields_content);
                let config = parse_config(&config_content)?;
                if config.export_type.is_none() {
                    return Err(Error::new(
                        field_name.span(),
                        "Missing export_type(..) – required for #[godot_components] entries",
                    ));
                }
                fields.push((field_name, config));

                if fields_content.peek(Token![,]) {
                    let _comma: Token![,] = fields_content.parse()?;
                }
            }
            if fields.is_empty() {
                return Err(Error::new(
                    prop.span(),
                    "Struct companion must list at least one field",
                ));
            }
            if !content.is_empty() {
                return Err(Error::new(
                    content.span(),
                    "Unexpected tokens after struct companion fields",
                ));
            }
            // Struct companions expose one Godot property per field, so the outer `prop` name is not stored.
            return Ok(CompanionEntry::Struct { component, fields });
        }

        // Newtype form: `prop(Comp, export_type(T), ...)`
        let config = if content.peek(Token![,]) {
            let _comma: Token![,] = content.parse()?;
            parse_config(&content)?
        } else {
            ExportConfig {
                export_type: None,
                default_expr: None,
                transform_with: None,
            }
        };
        if config.export_type.is_none() {
            return Err(Error::new(
                prop.span(),
                "Missing export_type(..) – required for #[godot_components] entries",
            ));
        }

        Ok(CompanionEntry::Newtype {
            prop,
            component,
            config: Box::new(config),
        })
    }
}

impl Parse for GodotComponentsAttr {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let entries = Punctuated::<CompanionEntry, Token![,]>::parse_terminated(input)?;
        Ok(GodotComponentsAttr {
            entries: entries.into_iter().collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::parse2;

    #[test]
    fn parses_all_three_entry_forms() {
        let attr: GodotComponentsAttr = parse2(quote! {
            (Grounded),
            speed(Speed, export_type(f32), default(250.0), transform_with(to_speed)),
            stats(Stats { current(export_type(i32), default(100)), max(export_type(i32)) })
        })
        .unwrap();

        assert_eq!(attr.entries.len(), 3);
        assert!(matches!(attr.entries[0], CompanionEntry::Marker { .. }));
        match &attr.entries[1] {
            CompanionEntry::Newtype { prop, config, .. } => {
                assert_eq!(prop.to_string(), "speed");
                assert!(config.export_type.is_some());
                assert!(config.default_expr.is_some());
                assert!(config.transform_with.is_some());
            }
            _ => panic!("Expected newtype entry"),
        }
        match &attr.entries[2] {
            CompanionEntry::Struct { fields, .. } => {
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].0.to_string(), "current");
                assert!(fields[1].1.default_expr.is_none());
            }
            _ => panic!("Expected struct entry"),
        }
    }

    #[test]
    fn newtype_without_export_type_is_error() {
        let result = parse2::<GodotComponentsAttr>(quote! {
            speed(Speed, default(250.0))
        });
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Missing export_type")
        );
    }

    #[test]
    fn struct_field_without_export_type_is_error() {
        let result = parse2::<GodotComponentsAttr>(quote! {
            stats(Stats { current(default(100)) })
        });
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Missing export_type")
        );
    }

    #[test]
    fn unknown_config_key_is_error() {
        let result = parse2::<GodotComponentsAttr>(quote! {
            speed(Speed, export_type(f32), color(blue))
        });
        assert!(result.unwrap_err().to_string().contains("Unknown key"));
    }

    #[test]
    fn marker_with_extra_tokens_is_error() {
        let result = parse2::<GodotComponentsAttr>(quote! {
            (Grounded, export_type(f32))
        });
        assert!(result.is_err());
    }

    #[test]
    fn empty_struct_companion_is_error() {
        let result = parse2::<GodotComponentsAttr>(quote! {
            stats(Stats {})
        });
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("at least one field")
        );
    }

    #[test]
    fn struct_companion_with_trailing_tokens_is_error() {
        let result = parse2::<GodotComponentsAttr>(quote! {
            stats(Stats { current(export_type(i32)) } extra)
        });
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("Unexpected tokens after struct companion fields")
        );
    }
}
