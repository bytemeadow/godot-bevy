#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::parse::Parser;
    use syn::parse_quote;

    #[test]
    fn cf_marker_and_newtype_companions() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(Component, GodotNode, Default)]
            #[gdbevy(base = CharacterBody2D, class_name = Player2D)]
            #[gdbevy(require(speed: Speed, as = f32, default = 250.0), require(Stunned))]
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
        assert_eq!(
            format!("{plan:?}"),
            "ClassPlan(Player2D, emit_node_class=true, 2 companions)"
        );
    }

    #[test]
    fn require_parser_consumes_marker_struct_and_binding_entries() {
        let marker = Parser::parse2(parse_one_require, quote!(Stunned)).unwrap();
        assert!(
            matches!(marker, RawRequire::Marker { component } if component.is_ident("Stunned"))
        );

        let structured = Parser::parse2(
            parse_one_require,
            quote!(stats: Stats { current(as = i32), maximum(as = i32, default = 100) }),
        )
        .unwrap();
        match structured {
            RawRequire::Struct { component, fields } => {
                assert!(component.is_ident("Stats"));
                assert_eq!(fields.len(), 2);
                assert_eq!(fields[0].0.to_string(), "current");
                assert_eq!(fields[1].0.to_string(), "maximum");
                assert!(fields[0].1.as_type.is_some());
                assert!(fields[1].1.default.is_some());
            }
            _ => panic!("expected structured require"),
        }

        let binding = Parser::parse2(
            parse_one_require,
            quote!(Stats {
                current: max_health,
                maximum: max_health
            }),
        )
        .unwrap();
        match binding {
            RawRequire::Binding { component, pairs } => {
                assert!(component.is_ident("Stats"));
                assert_eq!(pairs.len(), 2);
                assert_eq!(pairs[0].0.to_string(), "current");
                assert_eq!(pairs[0].1.to_string(), "max_health");
                assert_eq!(pairs[1].0.to_string(), "maximum");
                assert_eq!(pairs[1].1.to_string(), "max_health");
            }
            _ => panic!("expected binding require"),
        }
    }

    #[test]
    fn cf_primary_field_default() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(Component, GodotNode, Default)]
            #[gdbevy(base = Area2D, class_name = Door2D)]
            struct Door { #[gdbevy(export, default = LevelId::Level1)] level_id: LevelId }
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
    fn cf_primary_field_missing_export() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(Component, GodotNode, Default)]
            #[gdbevy(base = Area2D, class_name = Door2D)]
            struct Door { #[gdbevy(default = 1.0)] level_id: f32 }
        };
        assert!(
            parse_component_first(&di)
                .unwrap_err()
                .to_string()
                .contains("export")
        );
    }

    #[test]
    fn cf_primary_field_bare_export() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(Component, GodotNode, Default)]
            #[gdbevy(base = Area2D, class_name = Door2D)]
            struct Door { #[gdbevy(export)] level_id: LevelId }
        };
        let plan = parse_component_first(&di).unwrap();
        assert_eq!(plan.primary.fields.len(), 1);
        assert_eq!(plan.primary.fields[0].godot_prop.to_string(), "level_id");
        assert!(plan.primary.fields[0].default.is_none());
        assert!(plan.primary.fields[0].as_type.is_none());
    }

    #[test]
    fn cf_tuple_newtype_export() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(Component, GodotNode, Default)]
            struct Level(#[gdbevy(export, default = 1)] i32);
        };
        let plan = parse_component_first(&di).unwrap();
        let field = &plan.primary.fields[0];
        assert_eq!(field.godot_prop.to_string(), "value0");
        assert_eq!(field.tuple_index, Some(0));
        assert!(field.default.is_some());
    }

    #[test]
    fn cf_tuple_export_uses_field_position() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(Component, GodotNode, Default)]
            struct Velocity(f64, #[gdbevy(export)] f64, f64);
        };
        let plan = parse_component_first(&di).unwrap();
        assert_eq!(plan.primary.fields.len(), 1);
        assert_eq!(plan.primary.fields[0].godot_prop.to_string(), "value1");
        assert_eq!(plan.primary.fields[0].tuple_index, Some(1));
    }

    #[test]
    fn gf_field_binding() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(GodotClass, BevyComponents)]
            #[gdbevy(require(Player))]
            struct PlayerNode {
                base: Base<Node2D>,
                #[gdbevy(component = Speed, with = to_speed)]
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
    fn gf_tuple_struct_is_rejected() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(GodotClass, BevyComponents)]
            struct PlayerNode(#[gdbevy(component = Speed)] f32);
        };
        assert!(
            parse_godot_first(&di)
                .unwrap_err()
                .to_string()
                .contains("tuple structs are only supported")
        );
    }

    #[test]
    fn cf_as_missing_on_companion() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(Component, GodotNode)]
            #[gdbevy(require(speed: Speed, default = 250.0))]
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
            #[gdbevy(require(speed: Speed, as = f32), require(speed: Boost, as = f32))]
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
            #[gdbevy(require(stats: Stats { current(as = i32) }, default = 5))]
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
            #[gdbevy(class_name = Player)]
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
    fn duplicate_directive_key_is_error() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(Component, GodotNode)]
            #[gdbevy(require(speed: Speed, as = f32, default = 1.0, default = 2.0))]
            struct Player;
        };
        assert!(
            parse_component_first(&di)
                .unwrap_err()
                .to_string()
                .contains("duplicate `default`")
        );
    }

    #[test]
    fn gf_as_on_field_binding() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(GodotClass, BevyComponents)]
            struct PlayerNode {
                base: Base<Node2D>,
                #[gdbevy(component = Speed, as = f32)]
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
                #[gdbevy(component = Speed, default = 5.0)]
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
                #[gdbevy(with = to_speed)]
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
            #[gdbevy(require(speed: Speed, as = f32))]
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
            #[gdbevy(base = Node2D)]
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
            #[gdbevy(require(speed: Speed, as = f32, sync = two_way))]
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
            #[gdbevy(require(speed: Speed, as = f32, into = Foo))]
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
    fn hint_string_requires_hint() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(Component, GodotNode)]
            #[gdbevy(require(speed: Speed, as = f32, hint_string = "one"))]
            struct Player;
        };
        assert!(
            parse_component_first(&di)
                .unwrap_err()
                .to_string()
                .contains("`hint_string` requires `hint`")
        );
    }

    #[test]
    fn parses_export_description_and_hint() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(Component, GodotNode)]
            #[gdbevy(require(
                kind: Kind,
                as = GString,
                with = from_godot_string,
                description = "Weapon kind",
                hint = ENUM,
                hint_string = "Hands,Knife"
            ))]
            struct Player;
        };
        let plan = parse_component_first(&di).expect("valid export metadata");
        let ComponentInit::Newtype(mapping) = &plan.companions[0].init else {
            panic!("expected generated newtype mapping");
        };
        assert_eq!(mapping.description.as_ref().unwrap().value(), "Weapon kind");
        assert_eq!(mapping.hint.as_ref().unwrap().to_string(), "ENUM");
        assert!(mapping.hint_string.is_some());
    }

    #[test]
    fn metadata_survives_named_tuple_and_struct_companion_mappings() {
        let metadata = quote!(
            as = GString, with = convert, default = initial(),
            description = "Description", hint = ENUM, hint_string = labels()
        );
        for definition in [
            quote!(
                struct Settings {
                    #[gdbevy(export, #metadata)]
                    value: String,
                }
            ),
            quote!(
                struct Settings(u32, #[gdbevy(export, #metadata)] String);
            ),
            quote!(
                #[gdbevy(require(settings: SettingsData { value(#metadata) }))]
                struct Settings;
            ),
        ] {
            let input = syn::parse2(definition).unwrap();
            let plan = parse_component_first(&input).unwrap();
            let mapping = if let Some(mapping) = plan.primary.fields.first() {
                if mapping.bevy_field.is_none() {
                    assert_eq!(mapping.tuple_index, Some(1));
                    assert_eq!(mapping.godot_prop.to_string(), "value1");
                }
                mapping
            } else {
                let ComponentInit::Fields(fields) = &plan.companions[0].init else {
                    panic!("expected struct companion");
                };
                &fields[0]
            };
            assert_eq!(mapping.description.as_ref().unwrap().value(), "Description");
            assert_eq!(mapping.hint.as_ref().unwrap().to_string(), "ENUM");
            assert!(mapping.hint_string.is_some());
            assert!(mapping.as_type.is_some());
            assert!(mapping.with.is_some());
            assert!(mapping.default.is_some());
        }
    }

    #[test]
    fn invalid_metadata_is_rejected_at_each_generated_placement() {
        for (directives, expected) in [
            (
                quote!(description = "a", description = "b"),
                "duplicate `description`",
            ),
            (quote!(hint = ENUM, hint = RANGE), "duplicate `hint`"),
            (
                quote!(hint = ENUM, hint_string = "a", hint_string = "b"),
                "duplicate `hint_string`",
            ),
            (
                quote!(description = description()),
                "expected string literal",
            ),
            (quote!(hint_string = "a"), "`hint_string` requires `hint`"),
            (quote!(unknown = "a"), "unknown key `unknown`"),
        ] {
            for definition in [
                quote!(
                    struct Settings {
                        #[gdbevy(export, #directives)]
                        value: GString,
                    }
                ),
                quote!(
                    struct Settings(#[gdbevy(export, #directives)] GString);
                ),
                quote!(
                    #[gdbevy(require(value: Value, as = GString, #directives))]
                    struct Settings;
                ),
                quote!(
                    #[gdbevy(require(settings: SettingsData { value(as = GString, #directives) }))]
                    struct Settings;
                ),
            ] {
                let input = syn::parse2(definition).unwrap();
                assert!(
                    parse_component_first(&input)
                        .unwrap_err()
                        .to_string()
                        .contains(expected),
                    "{expected}"
                );
            }
        }
    }

    #[test]
    fn godot_first_metadata_uses_native_attributes() {
        for metadata in [
            quote!(description = "Description"),
            quote!(hint = ENUM),
            quote!(hint = ENUM, hint_string = "a,b"),
        ] {
            let input = syn::parse2(quote! {
                struct SettingsNode {
                    #[gdbevy(component = Settings, #metadata)]
                    value: GString,
                }
            })
            .unwrap();
            assert!(
                parse_godot_first(&input)
                    .unwrap_err()
                    .to_string()
                    .contains("only valid on generated component-first exports")
            );
        }
    }

    #[test]
    fn attachable_component_success() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(AttachableComponent, GodotClass)]
            #[class(init, base=Node)]
            #[gdbevy(target = Movement)]
            struct MovementComponent {
                #[export]
                max_speed: f32,
            }
        };
        let tokens = parse_attachable_component(&di).unwrap();
        let code = tokens.to_string();

        // Verify the target type, struct name, and the generated function name are present
        assert!(code.contains("Movement"));
        assert!(code.contains("MovementComponent"));
        assert!(code.contains("__movementcomponent"));
    }

    #[test]
    fn attachable_component_missing_target() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(AttachableComponent, GodotClass)]
            #[class(init, base=Node)]
            struct MovementComponent {
                #[export]
                max_speed: f32,
            }
        };
        let err = parse_attachable_component(&di).unwrap_err();
        assert!(
            err.to_string()
                .contains("Missing #[gdbevy(target = YourBevyComponent)] attribute")
        );
    }

    #[test]
    fn attachable_component_unsupported_property() {
        let di: syn::DeriveInput = parse_quote! {
            #[derive(AttachableComponent, GodotClass)]
            #[class(init, base=Node)]
            #[gdbevy(foo = "bar")]
            struct MovementComponent {
                #[export]
                max_speed: f32,
            }
        };
        let err = parse_attachable_component(&di).unwrap_err();
        assert!(
            err.to_string()
                .contains("unsupported property, expected `target`")
        );
    }
}
