# Nodes from Components and Bundles

Often, we want to make Godot nodes from Rust ECS types. The `GodotNode` derive macro supports two types:

- Component: `#[derive(Component, GodotNode)]`
- Bundle: `#[derive(Bundle, GodotNode)]`

Both generate a Godot class you can place in the editor and auto‑insert the corresponding ECS data when the scene is scanned.

See the `GodotNode` Rust docs for full syntax and options:
`https://docs.rs/godot-bevy/latest/godot_bevy/prelude/derive.GodotNode.html`.

## Configuring the Node

You can configure the Godot node's base type and class name with the `godot_node` struct-level attribute: 

```rust
#[derive(GodotNode, ...)]
#[godot_node(base(Area2D), class_name(Gem2D))]
pub struct Gem;
```

## Component + GodotNode → Node

Use the following method to create a Godot node from a single component.
Use when you want to expose a single component to the editor.

Gem marker component:

```rust
#[derive(Component, GodotNode, Default, Debug, Clone)]
#[godot_node(base(Area2D), class_name(Gem2D))]
pub struct Gem;
```

Door with an exported property:

```rust
#[derive(Component, GodotNode, Default, Debug, Clone)]
#[godot_node(base(Area2D), class_name(Door2D))]
pub struct Door {
    #[godot_export(default(LevelId::Level1))]
    pub level_id: LevelId,
}
```

Each derive generates a corresponding Godot class (e.g., `Gem2D`, `Door2D`) and inserts the component when the node is discovered. Fields marked with `#[godot_export]` become Godot editor properties.

## Bundle + GodotNode → Node

Sometimes a single component isn’t the right abstraction for your editor node. When you want one node to represent an entity with multiple components, derive on a Bevy `Bundle`:

```rust
#[derive(Bundle, GodotNode)]
#[godot_node(base(CharacterBody2D), class_name(Player2D))]
pub struct PlayerBundle {
    // Inserted as Default::default(), no Godot properties
    pub player: Player,

    // Tuple/newtype → property name is the bundle field name
    #[export_fields(value(export_type(f32), default(250.0)))]
    pub speed: Speed,

    #[export_fields(value(export_type(f32), default(-400.0)))]
    pub jump_velocity: JumpVelocity,

    // Custom default pulled from ProjectSettings
    #[export_fields(value(export_type(f32), default(godot::classes::ProjectSettings::singleton()
        .get_setting("physics/2d/default_gravity")
        .try_to::<f32>()
        .unwrap_or(980.0))))]
    pub gravity: Gravity,
}
```

What `#[export_fields]` does:

- Selects which component data is exported to the Godot editor
- Sets the Godot property type with `export_type(Type)`
- Optionally provides a default with `default(expr)`
- Optionally converts Godot → Bevy with `transform_with(path::to::fn)` when building the bundle

Property naming rules:

- Struct field entries export using the Bevy field name
- Tuple/newtype entry (value(...)) exports using the bundle field name
- Renaming is not supported; duplicate property names across the bundle are a compile error

Construction rules:

- Components without `#[export_fields]` are constructed with `Default::default()`
- For struct components, only the exported fields are set; the rest come from `..Default::default()`
- Nested bundles are allowed and will be flattened by Bevy on insertion; only top‑level fields can export properties

This derive generates a Godot class (`Player2D` above) and an autosync registration so the bundle is inserted automatically for matching nodes.
