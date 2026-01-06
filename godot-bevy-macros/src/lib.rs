mod bevy_bundle;
mod godot_node;
mod node_tree_view;

use crate::godot_node::derive_godot_node;
use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parser;
use syn::{DeriveInput, Error, parse_macro_input};

#[proc_macro_attribute]
pub fn bevy_app(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as syn::ItemFn);
    let name = &input_fn.sig.ident;

    // Parse attribute for configuration options
    let config = if !attr.is_empty() {
        match parse_bevy_app_config(attr) {
            Ok(cfg) => cfg,
            Err(err) => return err.into_compile_error().into(),
        }
    } else {
        BevyAppConfig::default()
    };

    let scene_tree_auto_despawn_children = config.scene_tree_auto_despawn_children;

    let expanded = quote! {
        struct BevyExtensionLibrary;

        #[gdextension]
        unsafe impl ExtensionLibrary for BevyExtensionLibrary {
            fn on_level_init(level: godot::prelude::InitLevel) {
                if level == godot::prelude::InitLevel::Core {
                    godot::private::class_macros::registry::class::auto_register_classes(level);

                    // Store the scene tree configuration
                    let _ = godot_bevy::app::BEVY_APP_CONFIG.set(godot_bevy::app::BevyAppConfig {
                        scene_tree_auto_despawn_children: #scene_tree_auto_despawn_children,
                    });

                    // Stores the client's entrypoint, which we'll call shortly when our `BevyApp`
                    // Godot Node has its `ready()` invoked
                    let _ = godot_bevy::app::BEVY_INIT_FUNC.get_or_init(|| Box::new(#name));

                    // Initialize profiling (Tracy or other backends)
                    // This function handles all feature gating internally
                    godot_bevy::profiling::init_profiler();
                }
            }


            fn on_level_deinit(_level: godot::prelude::InitLevel) {
                if _level == godot::prelude::InitLevel::Core {
                    // Shutdown profiling cleanly
                    // This function handles all feature gating internally
                    godot_bevy::profiling::shutdown_profiler();
                }
            }
        }

        #input_fn
    };

    expanded.into()
}

struct BevyAppConfig {
    scene_tree_auto_despawn_children: bool,
}

impl Default for BevyAppConfig {
    fn default() -> Self {
        Self {
            scene_tree_auto_despawn_children: true,
        }
    }
}

fn parse_bevy_app_config(attr: TokenStream) -> Result<BevyAppConfig, Error> {
    let mut config = BevyAppConfig::default();
    let parser = syn::meta::parser(|meta| {
        if meta.path.is_ident("scene_tree_auto_despawn_children") {
            config.scene_tree_auto_despawn_children = meta.value()?.parse::<syn::LitBool>()?.value;
            Ok(())
        } else if meta.path.is_ident("scene_tree_add_child_relationship") {
            Err(meta.error(
                "scene_tree_add_child_relationship was removed; use scene_tree_auto_despawn_children",
            ))
        } else {
            Err(meta.error("unsupported bevy_app attribute"))
        }
    });

    parser.parse(attr)?;
    Ok(config)
}

/// Derive this macro on a struct for easy access to a scene's nodes.
///
/// Example:
/// ```ignore
/// #[derive(NodeTreeView)]
/// pub struct MenuUi {
///     #[node("/root/Main/HUD/Message")]
///     pub message_label: GodotNodeHandle,
/// }
/// ```
/// Node paths can be specified with patterns:
/// - `/root/*/HUD/CurrentLevel` - matches any single node name where * appears
/// - `/root/Level*/HUD/CurrentLevel` - matches node names starting with "Level"
/// - `*/HUD/CurrentLevel` - matches relative to the base node
///
/// See `godot_bevy::node_tree_view::find_node_by_pattern` for details on how nodes are found.
///
/// Supported field types are:
/// - `GodotNodeHandle`: `from_node()` returns `NodeTreeViewError` if the node is not found.
/// - `Option<GodotNodeHandle>`: Filled with `None` if the node is not found.
///
/// For each field annotated with `#[node(<path>)]`, a companion string constant is generated
/// containing that path. The constant name is `<UPPERCASE_FIELD_NAME>_PATH`, and it is defined
/// in the struct that derives `NodeTreeView`.
///
/// Example:
/// ```ignore
/// #[derive(NodeTreeView)]
/// pub struct MobNodes {
///     #[node("AnimatedSprite2D")]
///     animated_sprite: GodotNodeHandle,
///
///     #[node("Node2D/*/VisibleOnScreenNotifier2D")]
///     visibility_notifier: GodotNodeHandle,
/// }
/// /// Generated companion string constants:
/// impl MobNodes {
///     pub const ANIMATED_SPRITE_PATH: &'static str = "AnimatedSprite2D";
///     pub const VISIBILITY_NOTIFIER_PATH: &'static str = "Node2D/*/VisibleOnScreenNotifier2D";
/// }
/// ```
#[proc_macro_derive(NodeTreeView, attributes(node))]
pub fn derive_node_tree_view(item: TokenStream) -> TokenStream {
    let view = parse_macro_input!(item as DeriveInput);

    let expanded = node_tree_view::node_tree_view(view).unwrap_or_else(Error::into_compile_error);

    TokenStream::from(expanded)
}

#[proc_macro_derive(BevyBundle, attributes(bevy_bundle))]
pub fn derive_bevy_bundle(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    let expanded = bevy_bundle::bevy_bundle(input).unwrap_or_else(Error::into_compile_error);

    TokenStream::from(expanded)
}

/// # Generates a Godot Node from a Bevy Component or Bevy Bundle
///
/// A struct level attribute can be used to specify the Godot class to extend, and the class name:
///
/// ```ignore
/// #[godot_node(base(<godot_node_type>), class_name(<custom_class_name>))]
/// ```
///
/// - `base` (Default: `Node`) Godot node to extend.
/// - `class_name` (Default: `<struct_name>BevyComponent`) Name of generated Godot class.
///
/// ## Annotating structs that derive `Bundle`
///
/// Bundle component fields can be annotated with `#[export_fields(...)]` to expose them to Godot.
/// The `export_fields` attribute takes a list of component field entries:
/// - Struct component fields: `field_name(export_type(Type), transform_with(path::to::fn), default(expr))`
/// - Tuple/newtype components: `value(export_type(Type), transform_with(path::to::fn), default(expr))`
///
/// Each entry can take optional parameters to configure how it will be exported. See
/// the [export configuration attributes](#export-configuration-attributes) for details.
///
/// Example syntax:
///
/// ```ignore
/// #[export_fields(
///     <field1_name>(
///         export_type(<godot_type>),
///         transform_with(<conversion_function>),
///         default(<value>)
///     ),
///     <field2_name>(...),
///     ...
/// )]
/// ```
///
/// ## Annotating structs that derive `Component`
///
/// Component fields can be exposed to Godot as node properties using the `#[godot_export]` attribute.
/// The attribute syntax is:
///
/// ```ignore
/// #[godot_export(export_type(<godot_type>), transform_with(<conversion_function>), default(<value>))]
/// ```
///
/// See the [export configuration attributes for](#export-configuration-attributes)
/// for export parameter details.
///
/// ## Export configuration attributes
///
/// For fields with types incompatible with Godot-Rust's `#[export]` macro:
/// - Use `export_type` to specify an alternate Godot-compatible type
/// - Use `transform_with` to provide a conversion function from the Godot type to the field type
/// - Use `default` to provide an initial value to the exported Godot field.
#[proc_macro_derive(GodotNode, attributes(godot_export, godot_node, export_fields))]
pub fn component_as_godot_node(input: TokenStream) -> TokenStream {
    let parsed: DeriveInput = parse_macro_input!(input as DeriveInput);
    derive_godot_node(parsed)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
