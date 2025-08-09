# Nodes from Components and Bundles

Often, we want to make a Godot node from Rust ECS types. There are two common flows:

- Components → Nodes with `#[derive(GodotNode)]`
- Bundles → Nodes with `#[derive(GodotNodeBundle)]`

Both generate a Godot class you can place in the editor and auto‑insert the corresponding ECS data when the scene is scanned.

## Components → Nodes (GodotNode)

Use when a single component is the natural editor‑facing unit.

```rust
#[derive(Component, GodotNode)]
#[godot_node(base(Node2D), class_name(PlayerNode))]
pub struct Player {
    #[godot_export]
    pub active: bool,

    #[godot_export(
        export_type(Vector2),
        transform_with(transform_to_vec2),
        default(Vector2::new(5.0, 15.0)),
    )]
    pub position: Vec2,

    // Won't be exposed to Godot
    pub internal_data: Vec<f32>,
}
```

This generates a `PlayerNode` Godot class with exported properties for fields marked with `#[godot_export]` and inserts the `Player` component when the node is discovered.

See the `GodotNode` Rust docs for full syntax and options: `https://docs.rs/godot-bevy/latest/godot_bevy/prelude/derive.GodotNode.html`.

## Bundles → Nodes (GodotNodeBundle)

Sometimes a single component isn’t the right abstraction for your editor node. When you want one node to represent an entity with multiple components, derive on a Bevy `Bundle`:

```rust
#[derive(Bundle, GodotNodeBundle)]
#[godot_node(base(CharacterBody2D), class_name(Player2D))]
pub struct PlayerBundle {
    // Inserted as Default::default(), no Godot properties
    pub player: Player,

    // Tuple/newtype → property name is the bundle field name
    #[godot_props((:, export_type(f32), default(250.0)))]
    pub speed: Speed,

    #[godot_props((:, export_type(f32), default(-400.0)))]
    pub jump_velocity: JumpVelocity,

    // Custom default pulled from ProjectSettings
    #[godot_props((:, export_type(f32), default(godot::classes::ProjectSettings::singleton()
        .get_setting("physics/2d/default_gravity")
        .try_to::<f32>()
        .unwrap_or(980.0))))]
    pub gravity: Gravity,
}
```

What `#[godot_props]` does:

- Selects which component data is exported to the Godot editor
- Sets the Godot property type with `export_type(Type)`
- Optionally provides a default with `default(expr)`
- Optionally converts Godot → Bevy with `transform_with(path::to::fn)` when building the bundle

Property naming rules:

- Struct field entries export using the Bevy field name
- Tuple/newtype entries export using the bundle field name
- Renaming is not supported; duplicate property names across the bundle are a compile error

Construction rules:

- Components without `#[godot_props]` are constructed with `Default::default()`
- For struct components, only the exported fields are set; the rest come from `..Default::default()`
- Nested bundles are allowed and will be flattened by Bevy on insertion; only top‑level fields can export properties

This derive generates a Godot class (`Player2D` above) and an autosync registration so the bundle is inserted automatically for matching nodes.

See the `GodotNodeBundle` Rust docs for full syntax and options: `https://docs.rs/godot-bevy/latest/godot_bevy/prelude/derive.GodotNodeBundle.html`.


