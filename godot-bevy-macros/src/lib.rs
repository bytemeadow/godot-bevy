mod bevy_attr;
mod emit;
mod godot_node;
mod node_tree_view;

use crate::godot_node::{
    derive_attachable_component, derive_bevy_components, derive_godot_node_component,
};
use proc_macro::TokenStream;
use quote::quote;
use syn::parse::Parser;
use syn::{DeriveInput, Error, parse_macro_input};

#[proc_macro_attribute]
pub fn bevy_app(attr: TokenStream, item: TokenStream) -> TokenStream {
    let input_fn = parse_macro_input!(item as syn::ItemFn);
    let name = &input_fn.sig.ident;

    let config = if !attr.is_empty() {
        match parse_bevy_app_config(attr) {
            Ok(cfg) => cfg,
            Err(err) => return err.into_compile_error().into(),
        }
    } else {
        BevyAppConfig::default()
    };

    let scene_tree_auto_despawn_children = config.scene_tree_auto_despawn_children;

    // Fully-qualified paths so a user's crate needs no `use godot::init::{...}`.
    let expanded = quote! {
        struct BevyExtensionLibrary;

        #[godot::init::gdextension]
        unsafe impl godot::init::ExtensionLibrary for BevyExtensionLibrary {
            fn on_stage_init(stage: godot::prelude::InitStage) {
                if stage == godot::prelude::InitStage::Core {
                    godot_bevy::app::init_with_config(
                        godot_bevy::app::BevyAppConfig {
                            scene_tree_auto_despawn_children: #scene_tree_auto_despawn_children,
                        },
                        #name,
                    );
                }
            }

            fn on_stage_deinit(stage: godot::prelude::InitStage) {
                if stage == godot::prelude::InitStage::Core {
                    godot_bevy::app::deinit();
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
/// <!-- qualification-doctest: scaffold=book-tests/src/doctest_scaffolds.rs#node_tree_view -->
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
/// <!-- qualification-doctest: scaffold=book-tests/src/doctest_scaffolds.rs#node_tree_paths -->
/// ```ignore
/// #[derive(NodeTreeView)]
/// pub struct MobNodes {
///     #[node("AnimatedSprite2D")]
///     animated_sprite: GodotNodeHandle,
///
///     #[node("Node2D/*/VisibleOnScreenNotifier2D")]
///     visibility_notifier: GodotNodeHandle,
/// }
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

/// # Attaches Bevy components to a user-owned Godot class (Godot-first)
///
/// Derive `BevyComponents` alongside `GodotClass` when you write the Godot class yourself.
/// The derive generates the autosync glue that reads `#[export]` property values off the
/// Godot node and inserts the mapped Bevy components onto the entity — without generating
/// a new `GodotClass` struct.
///
/// ## Field bindings
///
/// Annotate each `#[export]` field with `#[gdbevy(component = Comp)]` to map it onto a
/// newtype component:
///
/// <!-- qualification-doctest: scaffold=book-tests/src/doctest_scaffolds.rs#bevy_components_fields -->
/// ```rust,ignore
/// #[derive(GodotClass, BevyComponents)]
/// #[class(init, base=Node2D)]
/// struct PlayerNode {
///     base: Base<Node2D>,
///
///     /// Maps `speed` → `Speed(speed_value)`.  `with` converts the Godot value first.
///     #[export]
///     #[gdbevy(component = Speed, with = to_speed)]
///     speed: f32,
///
///     /// Maps `health` → `Health(health)` directly (no `with`).
///     #[export]
///     #[gdbevy(component = Health)]
///     health: f32,
/// }
/// ```
///
/// A field binding requires `component = Comp`. Optional `with = fn` converts the
/// Godot value before passing it to the component constructor. Use gdext's native
/// `#[export]`, `#[var(hint = ..., hint_string = ...)]`, and `#[init(val = ...)]`
/// attributes for property metadata and defaults. `as`, `default`, `description`,
/// `hint`, and `hint_string` are not valid Godot-first `gdbevy` keys.
///
/// ## Struct-level companions
///
/// Use `#[gdbevy(require(...))]` at the struct level to add components that are not tied to
/// a specific exported property:
///
/// <!-- qualification-doctest: scaffold=book-tests/src/doctest_scaffolds.rs#bevy_components_require -->
/// ```rust,ignore
/// #[derive(GodotClass, BevyComponents)]
/// #[class(init, base=Node2D)]
/// #[gdbevy(require(Player))]          // marker — inserted via `Player::default()`
/// #[gdbevy(require(Stats { current: max_health, max: max_health }))]  // N→1 binding
/// struct PlayerNode {
///     base: Base<Node2D>,
///     #[export] max_health: f32,
/// }
/// ```
///
/// Two forms of struct-level `require(...)`:
///
/// | Form | Meaning |
/// |------|---------|
/// | `require(Marker)` | Insert `Marker::default()` — a pure marker component. |
/// | `require(Comp { bevy_field: godot_field, … })` | Build `Comp` from existing Godot exports, mapping each Bevy field name to a Godot property name. |
///
/// `base`, `class_name`, and generated-export forms (`require(prop: Comp, …)`) are
/// **not** valid here — those are component-first (`GodotNode`) only.
#[proc_macro_derive(BevyComponents, attributes(gdbevy))]
pub fn derive_bevy_components_entry(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);
    derive_bevy_components(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// # Generates a Godot class from a Bevy `Component` (component-first)
///
/// Derive `GodotNode` alongside `Component` to generate a Godot class whose `#[export]`
/// properties are authored in the Godot editor and mirrored onto the entity's components
/// when the node enters the scene tree.
///
/// ## Struct-level attributes
///
/// Place `#[gdbevy(...)]` directly on the struct to set class metadata and declare companion
/// components:
///
/// <!-- qualification-doctest: scaffold=book-tests/src/doctest_scaffolds.rs#godot_node -->
/// ```rust,ignore
/// #[derive(Component, GodotNode, Default)]
/// #[gdbevy(base = CharacterBody2D, class_name = Player2D)]
/// #[gdbevy(require(speed: Speed, as = f32, default = 250.0))]
/// #[gdbevy(require(Stunned))]
/// struct Player;
/// ```
///
/// ### `base` and `class_name`
///
/// | Key | Default | Meaning |
/// |-----|---------|---------|
/// | `base = GodotBase` | `Node` | Godot class to extend (`CharacterBody2D`, `Area2D`, …). |
/// | `class_name = Name` | `<Struct>BevyComponent` | Name of the generated `GodotClass` struct. Must differ from the component name. |
///
/// ### `require(...)`
///
/// Declares a companion Bevy component. Three forms:
///
/// **Marker** — inserts the component via `Default`:
/// <!-- qualification-doctest: scaffold=book-tests/src/doctest_scaffolds.rs#godot_node_marker -->
/// ```rust,ignore
/// #[gdbevy(require(Stunned))]
/// ```
///
/// **Newtype** — generates one `#[export]` property and creates the component from it:
/// <!-- qualification-doctest: scaffold=book-tests/src/doctest_scaffolds.rs#godot_node_newtype -->
/// ```rust,ignore
/// #[gdbevy(require(speed: Speed, as = f32, default = 250.0, with = to_speed))]
/// //               ^^^^^ prop name  ^^^^ Godot export type
/// ```
/// `as = T` is **required**. `default = expr` sets the editor default and the Bevy
/// required-component default. `with = fn` converts the Godot value before constructing
/// the component.
///
/// **Struct** — generates multiple `#[export]` properties for a multi-field component:
/// <!-- qualification-doctest: scaffold=book-tests/src/doctest_scaffolds.rs#godot_node_struct -->
/// ```rust,ignore
/// #[gdbevy(require(stats: Stats { current(as = i32, default = 100), max(as = i32, default = 100) }))]
/// ```
/// Each inner `field(as = T, …)` follows the same `as`/`default`/`with` grammar. The name
/// before `:` (e.g. `stats`) is required by the grammar but ignored — the generated export
/// properties use the inner field names (`current`, `max`).
///
/// Multiple `require(...)` entries may appear in one `#[gdbevy(...)]` or in separate
/// `#[gdbevy(...)]` attributes — both are equivalent.
///
/// ## Field-level attributes
///
/// Annotate primary struct fields with `#[gdbevy(export, ...)]` to expose them as `#[export]`
/// properties on the generated Godot class. `export` is required — it marks the field as a
/// generated Godot export:
///
/// <!-- qualification-doctest: scaffold=book-tests/src/doctest_scaffolds.rs#godot_node_fields -->
/// ```rust,ignore
/// #[derive(Component, GodotNode, Default)]
/// #[gdbevy(base = Area2D, class_name = Door2D)]
/// struct Door {
///     #[gdbevy(export, default = LevelId::Level1)]
///     level_id: LevelId,
///
///     #[gdbevy(export, as = f32, with = meters_to_units)]
///     range: f32,
///
///     /// Label shown to the player.
///     #[gdbevy(export, as = GString, with = from_godot_string,
///         default = GString::from("Open"), description = "Door label",
///         hint = ENUM, hint_string = "Open,Locked")]
///     label: String,
/// }
///
/// fn from_godot_string(value: GString) -> String {
///     value.to_string()
/// }
/// ```
///
/// Tuple structs are supported for component-first nodes. Exported fields use `value0`,
/// `value1`, and so on as their Godot property names.
///
/// <!-- qualification-doctest: scaffold=book-tests/src/doctest_scaffolds.rs#godot_node_tuple -->
/// ```ignore
/// #[derive(Component, GodotNode, Default)]
/// struct Velocity(
///     #[gdbevy(export)] f32,
///     #[gdbevy(export)] f32,
/// );
/// ```
///
/// | Key | Meaning |
/// |-----|---------|
/// | `export` (**required**) | Marks the field as a generated Godot export. |
/// | `as = T` | Godot export type (defaults to the field's Rust type when omitted). |
/// | `default = expr` | Editor default value passed to `#[init(val = …)]`. A pure-Bevy `spawn(T)` uses the struct's own `Default` — make them agree if you rely on `spawn(T)`. |
/// | `with = fn` | Converts the Godot export value before assigning to the field. |
/// | `description = "..."` | Adds property docs to a primary field or generated companion export. |
/// | `hint = NAME` | Sets the Godot `PropertyHint` variant. |
/// | `hint_string = expr` | Supplies the hint string; requires `hint = NAME`. |
///
/// Metadata applies to named and tuple primary fields, `require(prop: Comp, ...)`,
/// and each field in `require(group: Comp { field(as = T, ...), ... })`.
/// Use one `#[gdbevy(export, ...)]` attribute per primary field.
/// `description` must be a string literal. `hint` is a bare `PropertyHint` variant.
/// Rust `///` docs on primary fields precede an explicit `description`.
///
/// Enable `godot-bevy/register-docs` to register descriptions in Godot's editor.
/// This feature is off by default and requires Godot API 4.3 or later.
/// Without it, hints still work and descriptions remain Rust docs. The same feature
/// enables native gdext property docs on Godot-first classes.
///
/// Export types must implement gdext's `Export` trait. For Rust `String` fields,
/// export a `GString` and convert it with `with`, as in the `Door` example.
///
/// ## Reserved keys
///
/// `into` and `sync` are reserved for the deferred component-sync feature and will produce
/// a compile error if used.
#[proc_macro_derive(GodotNode, attributes(gdbevy))]
pub fn component_as_godot_node(input: TokenStream) -> TokenStream {
    let parsed: DeriveInput = parse_macro_input!(input as DeriveInput);
    derive_godot_node_component(parsed)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

/// Attach a component to a parent entity from an editor-authored Godot node.
///
/// Derive `AttachableComponent` alongside `GodotClass`, set
/// `#[gdbevy(target = YourComponent)]`, and implement `From<&YourGodotClass>`
/// for that Bevy component. `GodotCorePlugins` registers the carrier automatically.
///
/// The carrier must be a leaf under a mirrored node. After ordinary scene-tree
/// messages are processed, the conversion runs once against the carrier's live
/// parent. The component is inserted on that parent's entity and the carrier is
/// queued for deletion. The carrier never gets its own entity.
///
/// This is one-shot configuration. Carrier nodes and their paths disappear.
/// Removing and re-adding the surviving parent does not restore the component.
/// Instantiate the saved scene again to get fresh carriers. Use `BevyComponents`
/// when the authored node should survive and receive components on its own entity.
///
/// Children, including internal children, cause rejection before `From` runs.
/// The parent cannot be another carrier, the root viewport, excluded, queued for
/// deletion, or missing from the mirror. Rejected carriers stay alive and
/// unmirrored. A warning gives the class, path, parent, and reason. Another
/// `NodeAdded` is required for a new attempt; empty drains do not retry.
/// Detached, freed, or queued carriers do not convert. Excluded subtrees normally
/// produce no add messages.
///
/// `From` may copy values, retain owned resources, and capture handles to nodes
/// that survive independently. Do not capture the carrier, its descendants,
/// other carriers, or paths through consumed nodes. Do not mutate the scene tree
/// in `From`. Use `GodotAccess::try_get` when a captured node may have been freed.
///
/// # Example
///
/// Assign `character_body` to a surviving parent or sibling in the Inspector.
///
/// <!-- qualification-doctest: scaffold=book-tests/src/doctest_scaffolds.rs#attachable_component -->
/// ```rust,ignore
/// use bevy::prelude::{Component, Vec2};
/// use godot::classes::CharacterBody3D;
/// use godot::obj::OnEditor;
/// use godot::prelude::*;
/// use godot_bevy::prelude::*;
///
/// #[derive(AttachableComponent, GodotClass)]
/// #[class(init, base=Node)]
/// #[gdbevy(target = Movement)]
/// struct MovementComponent {
///     #[export]
///     max_speed: f32,
///     #[export]
///     jump_height: f32,
///     #[export]
///     character_body: OnEditor<Gd<CharacterBody3D>>,
/// }
///
/// #[derive(Component)]
/// struct Movement {
///     max_speed: f32,
///     jump_height: f32,
///     character_body_3d: GodotNodeHandle,
///     desired_direction: Vec2,
/// }
///
/// impl From<&MovementComponent> for Movement {
///     fn from(value: &MovementComponent) -> Movement {
///         Movement {
///             max_speed: value.max_speed,
///             jump_height: value.jump_height,
///             character_body_3d: GodotNodeHandle::new(value.character_body.clone()),
///             desired_direction: Vec2::ZERO,
///         }
///     }
/// }
/// ```
///
/// # Struct-level attributes
///
/// `#[gdbevy(target = Comp)]` is required. `Comp` is the Bevy component to insert
/// on the parent and must implement `From<&YourGodotClass>`.
#[proc_macro_derive(AttachableComponent, attributes(gdbevy))]
pub fn derive_attachable_component_entry(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    derive_attachable_component(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
