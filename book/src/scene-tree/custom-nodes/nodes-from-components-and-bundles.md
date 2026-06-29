# Nodes from Components

godot-bevy bridges Godot nodes and Bevy entities through two derive macros that share one `#[bevy(...)]` attribute grammar:

- **`GodotNode`** (component-first) — you write a Bevy `Component`; the macro generates the Godot class.
- **`BevyComponents`** (Godot-first) — you write the `GodotClass` yourself; the macro wires its `#[export]` fields into Bevy components.

Pick whichever fits your workflow. Both produce the same result at runtime: a Godot scene node whose editor-set values become Bevy components on the entity.

## Component-first: `GodotNode`

Derive `Component` and `GodotNode` on a plain Rust struct. The macro generates a Godot class with `#[export]` properties for each annotated field, plus an autosync registration so that components are inserted when the node enters the scene tree.

### Minimal marker node

```rust
#[derive(Component, GodotNode, Default, Debug, Clone)]
#[bevy(base = Area2D, class_name = Gem2D)]
pub struct Gem;
```

This generates a `Gem2D` Godot class (extending `Area2D`). When `Gem2D` enters the scene tree, a `Gem` component is inserted on its entity. No exported properties.

### Primary fields with defaults

Fields on the component struct can be exported to the editor:

```rust
#[derive(Component, GodotNode, Default, Debug, Clone)]
#[bevy(base = Area2D, class_name = Door2D)]
pub struct Door {
    #[bevy(default = LevelId::Level1)]
    pub level_id: LevelId,
}
```

`#[bevy(default = expr)]` sets the editor default (via `#[init(val = …)]`). The field's Rust type is used as the Godot export type unless you add `as = T`.

Available keys on a field-level `#[bevy(...)]`:

| Key | Meaning |
|-----|---------|
| `as = T` | Godot export type (defaults to the field's Rust type). |
| `default = expr` | Editor default value (via `#[init(val = …)]`). A pure-Bevy `spawn(T)` uses the struct's own `Default` — make them agree if you rely on `spawn(T)`. |
| `with = fn` | Converts the Godot value before assigning to the field. |

### Companion components

Use `require(...)` at the struct level to generate exported properties that feed separate companion components. This is useful when a single node should spawn multiple components — without needing a separate bundle type.

```rust
#[derive(Component, GodotNode, Default, Debug, Clone, Reflect)]
#[reflect(Component)]
#[bevy(base = CharacterBody2D, class_name = Player2D)]
#[bevy(
    require(speed: Speed, as = f32, default = 250.0),
    require(jump_velocity: JumpVelocity, as = f32, default = -400.0),
    require(gravity: Gravity, as = f32, default = 980.0),
)]
pub struct Player;
```

The `Player2D` Godot class gains three `#[export]` properties (`speed`, `jump_velocity`, `gravity`). When the node enters the tree, `Player`, `Speed(…)`, `JumpVelocity(…)`, and `Gravity(…)` are all inserted on the entity.

`require` forms:

| Form | Meaning |
|------|---------|
| `require(Marker)` | Insert `Marker::default()` — no export property. |
| `require(prop: Comp, as = T, default = expr)` | Generate one export property; build `Comp(value)`. `as = T` is required. |
| `require(prop: Comp { field(as = T, default = expr), … })` | Generate multiple properties; build a struct `Comp { field: value, … }`. The name before `:` is required by the grammar but ignored — the generated export properties use the inner field names. |

`with = fn` is available on all non-marker forms and converts the Godot value before it is passed to the component constructor.

> **Pure-Bevy spawn:** because `GodotNode` also registers required components, `commands.spawn(Player)` in a test or a headless context inserts `Speed`, `JumpVelocity`, and `Gravity` with the declared defaults — no Godot scene needed.

## Godot-first: `BevyComponents`

When you already own the `GodotClass` struct — or prefer writing gdext code yourself — derive `BevyComponents` instead of `GodotNode`. The macro emits only the Bevy side; no new Godot class is generated.

```rust
#[derive(GodotClass, BevyComponents)]
#[class(base = Node2D, init)]
#[bevy(require(Player))]
struct PlayerNode {
    base: Base<Node2D>,

    /// Maps the `speed` export to `Speed(to_speed(speed))`.
    #[bevy(component = Speed, with = to_speed)]
    #[export]
    #[init(val = 250.0)]
    speed: f32,
}
```

Field-level `#[bevy(...)]` keys on a Godot-first binding:

| Key | Meaning |
|-----|---------|
| `component = Comp` | **(required)** The Bevy component to insert — `Comp(value)`. |
| `with = fn` | Converts the Godot value before constructing the component. |

`as` and `default` are **not** allowed on Godot-first field bindings — gdext's `#[init(val = …)]` owns defaults, and the field's type is already visible.

Struct-level `require(...)` on the Godot-first path supports markers and N→1 bindings:

| Form | Meaning |
|------|---------|
| `require(Marker)` | Insert `Marker::default()`. |
| `require(Comp { bevy_field: godot_field, … })` | Build `Comp` from existing export fields. |

## Which derive to use

| | `GodotNode` | `BevyComponents` |
|---|---|---|
| Who writes the Godot class | Macro | You |
| `base` / `class_name` | `#[bevy(base = …, class_name = …)]` | `#[class(base = …)]` in gdext |
| Required-components (pure Bevy spawn) | Yes | No |
| Custom `init` / `#[godot_api]` | No | Yes — full gdext control |

Use `GodotNode` for new nodes defined entirely in Rust. Use `BevyComponents` when you need custom gdext lifecycle methods, or when the node class is shared with GDScript.
