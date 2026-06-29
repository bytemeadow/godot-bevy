# Property Mapping — `#[bevy(...)]` Grammar

> This page was previously "Property Mapping with BevyBundle". The `BevyBundle` macro is gone; the unified `#[bevy(...)]` attribute replaces it on both bridging derives.

Both `GodotNode` (component-first) and `BevyComponents` (Godot-first) accept `#[bevy(...)]` annotations. The keys available depend on which derive you are using.

## Component-first (`GodotNode`)

### Companion component — newtype

Generate one exported property and wrap its value in a newtype component:

```rust
#[derive(Component, GodotNode, Default)]
#[bevy(base = CharacterBody2D, class_name = Player2D)]
#[bevy(require(speed: Speed, as = f32, default = 250.0))]
pub struct Player;
```

- `speed` becomes a `#[export] speed: f32` on the generated `Player2D` class (default 250.0).
- When the node enters the tree, `Speed(speed_value)` is inserted.
- `as = f32` is required here — the macro cannot see `Speed`'s inner type.

### Companion component — struct

Generate multiple exported properties for a multi-field component:

```rust
#[derive(Component, GodotNode, Default)]
#[bevy(base = CharacterBody2D, class_name = Enemy2D)]
#[bevy(require(stats: Stats {
    health(as = f32, default = 100.0),
    mana(as = f32, default = 50.0),
}))]
pub struct Enemy;
```

Each inner `field(as = T, …)` follows the same `as`/`default`/`with` grammar as the newtype form. The name before `:` (e.g. `stats`) is required by the grammar but ignored — the generated export properties use the inner field names (`health`, `mana`).

### Marker companion

Insert a component via `Default` with no exported property:

```rust
#[bevy(require(Stunned))]
```

### Primary field with conversion

Fields on the component struct itself can declare a type conversion:

```rust
#[derive(Component, GodotNode, Default)]
#[bevy(base = Node2D, class_name = Slider2D)]
pub struct Slider {
    /// Editor shows 0–100; component gets 0.0–1.0
    #[bevy(as = f32, with = percentage_to_fraction)]
    pub value: f32,
}

fn percentage_to_fraction(v: f32) -> f32 { v / 100.0 }
```

`as = T` is optional when the field type is already Godot-compatible; add it only when the export type differs from the Rust field type.

### All component-first keys

| Placement | Key | Required? | Meaning |
|-----------|-----|-----------|---------|
| struct | `base = GodotBase` | no (default: `Node`) | Godot class to extend |
| struct | `class_name = Name` | no (default: `<Struct>BevyComponent`) | Generated class name |
| struct | `require(…)` | no | Companion component (see forms above) |
| field | `as = T` | no | Godot export type |
| field | `default = expr` | no | Editor default (via `#[init(val = …)]`); a pure-Bevy `spawn(T)` uses the struct's own `Default` — make them agree if you rely on `spawn(T)`. |
| field | `with = fn` | no | Godot-value → field-value conversion |
| `require(prop: Comp, …)` | `as = T` | **yes** | Export type for the generated property |
| `require(prop: Comp, …)` | `default = expr` | no | Export default |
| `require(prop: Comp, …)` | `with = fn` | no | Conversion before constructing the component |

## Godot-first (`BevyComponents`)

### Field binding

Map a single `#[export]` property to a newtype component:

```rust
#[derive(GodotClass, BevyComponents)]
#[class(base = Node2D, init)]
pub struct EnemyNode {
    base: Base<Node2D>,

    #[bevy(component = Health)]
    #[export]
    max_health: f32,

    #[bevy(component = Speed, with = to_speed)]
    #[export]
    #[init(val = 100.0)]
    speed: f32,
}
```

`component = Comp` is required. `with = fn` is optional. `as` and `default` are **not** allowed — gdext's `#[init(val = …)]` owns defaults on this path.

### Marker at the struct level

```rust
#[derive(GodotClass, BevyComponents)]
#[class(base = CharacterBody2D, init)]
#[bevy(require(Player))]
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
#[bevy(require(Stats { health: max_health, mana: max_mana }))]
pub struct PlayerNode {
    base: Base<CharacterBody2D>,
    #[export] max_health: f32,
    #[export] max_mana: f32,
}
```

### All Godot-first keys

| Placement | Key | Required? | Meaning |
|-----------|-----|-----------|---------|
| struct | `require(Marker)` | no | Insert `Marker::default()` |
| struct | `require(Comp { bevy_field: godot_field, … })` | no | Build struct component from existing exports |
| field | `component = Comp` | **yes** | Bevy component type (`Comp(value)`) |
| field | `with = fn` | no | Godot-value → component-value conversion |

## Reserved keys

`into` and `sync` are reserved for the upcoming component-sync feature and will produce a compile error if used. Use only the keys documented above.
