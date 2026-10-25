# Property Mapping — `#[gdbevy(...)]` Grammar

> This page was previously "Property Mapping with BevyBundle". The `BevyBundle` macro is gone; the unified `#[gdbevy(...)]` attribute replaces it on both bridging derives.

Both `GodotNode` (component-first) and `BevyComponents` (Godot-first) accept `#[gdbevy(...)]` annotations. The keys available depend on which derive you are using.

## Component-first (`GodotNode`)

### Companion component — newtype

Generate one exported property and wrap its value in a newtype component:

```rust
#[derive(Component, GodotNode, Default)]
#[gdbevy(base = CharacterBody2D, class_name = Player2D)]
#[gdbevy(require(speed: Speed, as = f32, default = 250.0))]
pub struct Player;
```

- `speed` becomes a `#[export] speed: f32` on the generated `Player2D` class (default 250.0).
- When the node enters the tree, `Speed(speed_value)` is inserted.
- `as = f32` is required here — the macro cannot see `Speed`'s inner type.

### Companion component — struct

Generate multiple exported properties for a multi-field component:

```rust
#[derive(Component, GodotNode, Default)]
#[gdbevy(base = CharacterBody2D, class_name = Enemy2D)]
#[gdbevy(require(stats: Stats {
    health(as = f32, default = 100.0),
    mana(as = f32, default = 50.0),
}))]
pub struct Enemy;
```

Each inner `field(as = T, …)` follows the same `as`/`default`/`with` grammar as the newtype form. The name before `:` (e.g. `stats`) is required by the grammar but ignored — the generated export properties use the inner field names (`health`, `mana`).

### Inspector metadata for generated exports

Metadata can be placed on a named or tuple primary field with
`#[gdbevy(export, ...)]`, on `require(prop: Comp, ...)`, or on each field inside
`require(group: Comp { field(as = T, ...), ... })`. Use one attribute per primary field.

Export types must implement gdext's `Export` trait. Rust `String` does not, so
export a `GString` and convert it when constructing the component:

```rust
{{#include ../../../../book-tests/src/inspector_metadata.rs}}
```

`description` takes a string literal. Primary field `///` docs are copied before
an explicit description. `hint` names a bare Godot `PropertyHint` variant, and
`hint_string` takes an expression. A hint string requires a hint.

Enable the `register-docs` feature on your `godot-bevy` dependency to register
property descriptions in Godot's editor:

```toml
godot-bevy = { version = "0.12", features = ["register-docs"] }
```

This feature is off by default and requires Godot API 4.3 or later. With it off,
hints still work and descriptions remain Rust docs. API 4.2 consumers must leave
it disabled.

### Marker companion

Insert a component via `Default` with no exported property:

```rust
#[gdbevy(require(Stunned))]
```

### Primary field with conversion

Fields on the component struct itself can declare a type conversion:

```rust
#[derive(Component, GodotNode, Default)]
#[gdbevy(base = Node2D, class_name = Slider2D)]
pub struct Slider {
    /// Editor shows 0–100; component gets 0.0–1.0
    #[gdbevy(export, as = f32, with = percentage_to_fraction)]
    pub value: f32,
}

fn percentage_to_fraction(v: f32) -> f32 { v / 100.0 }
```

`as = T` is optional when the field type already implements gdext's `Export` trait; add it only when the export type differs from the Rust field type.

### Tuple fields

Tuple structs are supported for component-first nodes. Exported fields are named `value0`, `value1`, and so on in the generated Godot class.

```rust
#[derive(Component, GodotNode, Default)]
pub struct Velocity(
    #[gdbevy(export)] f32,
    #[gdbevy(export)] f32,
);
```

### All component-first keys

| Placement | Key | Required? | Meaning |
|-----------|-----|-----------|---------|
| struct | `base = GodotBase` | no (default: `Node`) | Godot class to extend |
| struct | `class_name = Name` | no (default: `<Struct>BevyComponent`) | Generated class name |
| struct | `require(…)` | no | Companion component (see forms above) |
| field | `export` | **yes** | Marks the field as a generated Godot export |
| field | `as = T` | no | Godot export type |
| field | `default = expr` | no | Editor default (via `#[init(val = …)]`); a pure-Bevy `spawn(T)` uses the struct's own `Default` — make them agree if you rely on `spawn(T)`. |
| field | `with = fn` | no | Godot-value → field-value conversion |
| `require(prop: Comp, …)` | `as = T` | **yes** | Export type for the generated property |
| `require(prop: Comp, …)` | `default = expr` | no | Export default |
| `require(prop: Comp, …)` | `with = fn` | no | Conversion before constructing the component |
| generated primary or companion field | `sync = two_way` or `sync = to_godot` | no | Opt into live synchronization |
| generated primary or companion field | `into = fn` | with live `with` | Bevy-value → Godot-value conversion; requires `sync` |
| generated primary or companion field | `description = "..."` | no | Property docs, registered with `register-docs` |
| generated primary or companion field | `hint = NAME` | no | Bare Godot `PropertyHint` variant |
| generated primary or companion field | `hint_string = expr` | no | Hint string, requires `hint` |

## Godot-first (`BevyComponents`)

### Field binding

Map a single `#[export]` property to a newtype component:

```rust
#[derive(GodotClass, BevyComponents)]
#[class(base = Node2D, init)]
pub struct EnemyNode {
    base: Base<Node2D>,

    #[gdbevy(component = Health)]
    #[export]
    max_health: f32,

    #[gdbevy(component = Speed, with = to_speed)]
    #[export]
    #[init(val = 100.0)]
    speed: f32,
}
```

`component = Comp` is required for a newtype binding. `with = fn` is optional. `as` and `default` are **not** allowed — gdext's `#[init(val = …)]` owns defaults on this path.

### Marker at the struct level

```rust
#[derive(GodotClass, BevyComponents)]
#[class(base = CharacterBody2D, init)]
#[gdbevy(require(Player))]
pub struct PlayerNode {
    base: Base<CharacterBody2D>,
    // ...
}
```

### N→1 binding

Build a multi-field component from several existing `#[export]` properties:

```rust
#[derive(GodotClass, BevyComponents)]
#[class(base = CharacterBody2D, init)]
#[gdbevy(require(Stats { health: max_health, mana: max_mana }))]
pub struct PlayerNode {
    base: Base<CharacterBody2D>,
    #[export] max_health: f32,
    #[export] max_mana: f32,
}
```

Add `#[gdbevy(with = converter)]` to a source field to convert its value before
assigning it to each N→1 component field that reads it. No `component` key is
needed on that field; for example, exported `GString` paths can populate `String`
fields:

```rust
{{#include ../../../../book-tests/src/n_to_one_conversions.rs}}
```

### All Godot-first keys

| Placement | Key | Required? | Meaning |
|-----------|-----|-----------|---------|
| struct | `require(Marker)` | no | Insert `Marker::default()` |
| struct | `require(Comp { bevy_field: godot_field, … })` | no | Build struct component from existing exports |
| field | `component = Comp` | for newtype bindings | Bevy component type (`Comp(value)`) |
| field | `with = fn` | no | Godot-value → component-value conversion |
| field | `sync = two_way` or `sync = to_godot` | no | Opt into live synchronization |
| field | `into = fn` | with live `with` | Bevy-value → Godot-value conversion; requires `sync` |

Godot-first classes own their gdext declaration, so use gdext's native
`#[var(hint = ..., hint_string = ...)]` and Rust documentation attributes for
Inspector metadata on those fields.

Native property descriptions also require `godot-bevy/register-docs` and Godot
API 4.3 or later. Keep metadata on the native attributes, outside `gdbevy`.

## Live mapping grammar

Mappings are creation-only unless they opt into `sync`. The same directives work
on these placements:

| Placement | Example |
| --- | --- |
| `GodotNode` named or tuple field | `#[gdbevy(export, sync = two_way)]` |
| Generated newtype companion | `require(speed: Speed, as = f32, sync = two_way)` |
| Generated struct companion field | `require(stats: Stats { health(as = i32, sync = to_godot) })` |
| `BevyComponents` newtype source | `#[gdbevy(component = Speed, sync = two_way)]` |
| `BevyComponents` N→1 source | `#[gdbevy(sync = two_way)]` on the field named by `require(...)` |

| Directive | Meaning |
| --- | --- |
| `sync = two_way` | Read Godot in `PreUpdate`, write Bevy in `PostUpdate`; a pending Bevy edit wins on that field |
| `sync = to_godot` | Keep the creation-time import, then publish only Bevy edits |
| `with = path` | Convert an owned Godot value into the Bevy field value |
| `into = path` | Convert an owned Bevy field value into the Godot value; valid only with `sync` |

Both converter values are Rust paths, not strings or closures. A live mapping
with `with` requires `into`. Without converters the value types must agree.
`into` may also be supplied without `with` when no inbound conversion is needed.

```rust
{{#include ../../../../book-tests/src/live_property_sync.rs:companion}}
```

Here `LevelView.value0` maps to `Level.0`. The `ActorView` properties `speed`,
`health`, and `mana` map to their companion fields independently.

For an N→1 binding, put `sync`, `with`, and `into` on the existing Godot source
field. Other source fields can remain creation-only:

```rust
{{#include ../../../../book-tests/src/live_property_sync.rs:tuning}}
```

Each live source must feed exactly one destination. Two live mappings cannot
write the same Bevy field; a live target cannot overlap a creation-only target
either. Creation-only sources can still populate several destinations. Marker
companions, unmapped fields, and `AttachableComponent` carriers cannot use
`sync` or `into`.

Godot-first live sources require a plain stored `#[export]` or `#[var]` with no
custom getter or setter. Arrays, dictionaries, `Variant`, and objects are
unsupported on either side, even with converters. Immutable components accept
only `to_godot`. See
[live property sync](live-property-sync.md) for value bounds, conversion examples,
conflict rules, and lifecycle behavior.
