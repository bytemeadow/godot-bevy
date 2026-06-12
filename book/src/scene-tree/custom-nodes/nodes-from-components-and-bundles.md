# Nodes from Components and Bundles

Often, we want to make Godot nodes from Rust ECS types. The `GodotNode` derive macro supports two types:

- Component: `#[derive(Component, GodotNode)]`
- Bundle: `#[derive(Bundle, GodotNode)]` (deprecated — see below)

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

### Companion Components with `#[godot_components]`

When an entity needs multiple components — some populated from Godot editor exports — add the
`#[godot_components(...)]` attribute to your primary component. Each entry declares a *companion
component* that is inserted alongside the primary one.

```rust
use godot::classes::ProjectSettings;
use godot::obj::Singleton;

#[derive(Component, GodotNode, Debug, Clone, Default, Reflect)]
#[reflect(Component)]
#[godot_node(base(CharacterBody2D), class_name(Player2D))]
#[godot_components(
    speed(Speed, export_type(f32), default(250.0)),
    jump_velocity(JumpVelocity, export_type(f32), default(-400.0)),
    gravity(Gravity, export_type(f32), default(ProjectSettings::singleton()
        .get_setting("physics/2d/default_gravity")
        .try_to::<f32>()
        .unwrap_or(980.0))),
)]
pub struct Player;
```

**Two insertion paths, one source of truth for defaults:**

- **Scene-spawned** (`Player2D` node in the editor): each companion is populated from the
  node's exported properties, so designers can tweak values per-node without touching code.
- **Bevy-spawned** (`commands.spawn(Player)`): companions are inserted via Bevy's required
  components mechanism, using the `default(...)` expressions declared in the macro. No
  separate bundle needed.

**Entry forms:**

| Form | When to use |
|------|-------------|
| `(Marker)` | Marker companion; no Godot export; always `Default::default()` |
| `prop(Comp, export_type(T), default(expr))` | Newtype/tuple companion; one Godot property named `prop` |
| `prop(Comp { field(export_type(T), ...), ... })` | Struct companion; one export per listed field; unlisted fields from `Comp::default()` |

The `transform_with(path::to::fn)` option is available on both newtype and struct entries to
convert the Godot value before it is stored in the component.

**Constraints:**

- `export_type(T)` is required for every exported entry.
- Every companion must implement `Default` (used for Bevy-spawned insertion).
- Do **not** also write Bevy's `#[require(Comp)]` for the same companion — the macro
  registers it automatically; a duplicate registration is a compile error.

## Bundle + GodotNode → Node

> **Deprecated:** bundle mode requires a user-authored bundle struct — the pattern
> Bevy replaced with [required components](https://docs.rs/bevy/latest/bevy/ecs/component/derive.Component.html#required-components).
> Prefer `#[godot_components]` on a component (above). Bundle mode keeps working in
> this release; compile-time deprecation warnings and removal will follow in later
> releases.

Sometimes a single component isn't the right abstraction for your editor node. When you want one node to represent an entity with multiple components, derive on a Bevy `Bundle`:

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

### Migrating from Bundle Mode

The platformer-2d example was migrated from bundle mode to `#[godot_components]`. The generated
class name (`Player2D`) and property names (`speed`, `jump_velocity`, `gravity`) are identical, so
existing `.tscn` scenes continue to work unchanged.

**Before (bundle mode):**

```rust
#[derive(Bundle, GodotNode)]
#[godot_node(base(CharacterBody2D), class_name(Player2D))]
pub struct PlayerBundle {
    pub player: Player,

    #[export_fields(value(export_type(f32), default(250.0)))]
    pub speed: Speed,

    #[export_fields(value(export_type(f32), default(-400.0)))]
    pub jump_velocity: JumpVelocity,

    #[export_fields(value(export_type(f32), default(ProjectSettings::singleton()
        .get_setting("physics/2d/default_gravity")
        .try_to::<f32>()
        .unwrap_or(980.0))))]
    pub gravity: Gravity,
}
```

**After (`#[godot_components]` on a component):**

```rust
#[derive(Component, GodotNode, Debug, Clone, Default, Reflect)]
#[reflect(Component)]
#[godot_node(base(CharacterBody2D), class_name(Player2D))]
#[godot_components(
    speed(Speed, export_type(f32), default(250.0)),
    jump_velocity(JumpVelocity, export_type(f32), default(-400.0)),
    gravity(Gravity, export_type(f32), default(ProjectSettings::singleton()
        .get_setting("physics/2d/default_gravity")
        .try_to::<f32>()
        .unwrap_or(980.0))),
)]
pub struct Player;
```

The bundle struct and its field is replaced by a single primary component. Bevy's required
components mechanism handles insertion of companions in both the scene-spawned and Bevy-spawned
paths — no separate bundle type is needed.
