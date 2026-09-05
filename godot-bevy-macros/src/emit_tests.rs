#[cfg(test)]
mod tests {
    use super::*;
    use quote::quote;
    use syn::{Expr, Member, Stmt, Type, parse_quote};

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

    #[test]
    fn primary_and_field_tokens_reparse_to_exact_initializers() {
        let mapping = Mapping {
            godot_prop: parse_quote!(speed),
            bevy_field: Some(parse_quote!(velocity)),
            tuple_index: None,
            as_type: None,
            default: None,
            with: None,
        };
        let primary = PrimaryPlan {
            path: parse_quote!(Player),
            fields: vec![mapping],
        };

        let value: Expr = syn::parse2(primary_value(&primary).unwrap()).unwrap();
        let Expr::Struct(value) = value else {
            panic!("expected struct initializer");
        };
        assert!(value.path.is_ident("Player"));
        assert_eq!(value.fields.len(), 1);
        assert!(matches!(
            &value.fields[0].member,
            Member::Named(name) if name == "velocity"
        ));
        let read = value.fields[0].expr.to_token_stream().to_string();
        assert_eq!(read, "node . bind () . speed . clone ()");
        assert!(value.rest.is_some());

        let marker = PrimaryPlan {
            path: parse_quote!(Stunned),
            fields: Vec::new(),
        };
        let marker: Expr = syn::parse2(primary_value(&marker).unwrap()).unwrap();
        assert_eq!(
            marker.to_token_stream().to_string(),
            "Stunned :: default ()"
        );
    }

    #[test]
    fn companion_default_tokens_preserve_conversion_and_literal() {
        let mapping = Mapping {
            godot_prop: parse_quote!(speed),
            bevy_field: None,
            tuple_index: None,
            as_type: Some(parse_quote!(f32)),
            default: Some(parse_quote!(2.5)),
            with: Some(parse_quote!(to_speed)),
        };
        let value: Expr = syn::parse2(companion_default_value(&mapping)).unwrap();
        let Expr::Call(call) = value else {
            panic!("expected conversion call");
        };
        assert_eq!(call.func.to_token_stream().to_string(), "to_speed");
        assert_eq!(call.args.len(), 1);
        assert_eq!(call.args[0].to_token_stream().to_string(), "2.5");
    }

    #[test]
    fn primary_field_type_selects_only_the_named_field() {
        let input: DeriveInput = parse_quote! {
            struct Source { speed: f32, count: usize }
        };
        let count_mapping = Mapping {
            godot_prop: parse_quote!(count),
            bevy_field: Some(parse_quote!(count)),
            tuple_index: None,
            as_type: None,
            default: None,
            with: None,
        };
        let count = primary_field_type(&input, &count_mapping).unwrap();
        assert!(matches!(count, Type::Path(path) if path.path.is_ident("usize")));
        let missing_mapping = Mapping {
            godot_prop: parse_quote!(missing),
            bevy_field: Some(parse_quote!(missing)),
            tuple_index: None,
            as_type: None,
            default: None,
            with: None,
        };
        assert!(primary_field_type(&input, &missing_mapping).is_none());
    }

    #[test]
    fn partial_tuple_primary_starts_from_default_and_assigns_by_index() {
        let primary = PrimaryPlan {
            path: parse_quote!(Velocity),
            fields: vec![Mapping {
                godot_prop: parse_quote!(value1),
                bevy_field: None,
                tuple_index: Some(1),
                as_type: None,
                default: None,
                with: Some(parse_quote!(to_velocity)),
            }],
        };
        let value: Expr = syn::parse2(primary_value(&primary).unwrap()).unwrap();
        assert_eq!(
            value.to_token_stream().to_string(),
            "{ let mut c = Velocity :: default () ; c . 1 = to_velocity (node . bind () . value1 . clone ()) ; c }"
        );
    }

    #[test]
    fn registration_warning_reparses_to_the_expected_macro() {
        let warning = registration_warn(&parse_quote!(Speed), &parse_quote!(Player));
        let block: syn::Block = syn::parse2(quote!({ #warning })).unwrap();
        assert_eq!(block.stmts.len(), 1);
        let Stmt::Macro(statement) = &block.stmts[0] else {
            panic!("expected warning macro statement");
        };
        assert_eq!(
            statement.mac.path.to_token_stream().to_string(),
            "godot_bevy :: tracing :: warn"
        );
        let arguments = statement.mac.tokens.to_string();
        assert!(arguments.contains("Speed"));
        assert!(arguments.contains("Player"));
    }

    #[test]
    fn top_level_comma_detection_tracks_angles_pipes_and_comparisons() {
        assert!(has_top_level_comma(quote!(left, right)));
        assert!(!has_top_level_comma(quote!(foo::<Left, Right>())));
        assert!(!has_top_level_comma(quote!(|left, right| left + right)));
        assert!(has_top_level_comma(quote!(foo::<Left, Right>(), tail)));
        assert!(has_top_level_comma(quote!(left > right, tail)));
    }

    #[test]
    fn require_entry_consumes_trailing_default_syntax() {
        let entry: RequireEntry = syn::parse2(quote!(Speed = default_speed())).unwrap();
        assert!(entry.0.is_ident("Speed"));
    }
}
