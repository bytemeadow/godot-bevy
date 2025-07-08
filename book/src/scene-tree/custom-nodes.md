# Custom Node Markers

This chapter explains how to work with custom Godot nodes in godot-bevy and the important distinction between automatic markers for built-in Godot types versus custom nodes.

## Automatic Markers vs Custom Nodes

### Built-in Godot Types

godot-bevy **automatically** creates marker components for all built-in Godot node types:

```rust
// These markers are created automatically:
// Sprite2DMarker, CharacterBody2DMarker, Area2DMarker, etc.

fn update_sprites(sprites: Query<&GodotNodeHandle, With<Sprite2DMarker>>) {
    // Works automatically for any Sprite2D in your scene
}
```

### Custom Godot Nodes

Custom nodes defined in Rust or GDScript **do NOT** receive automatic markers for their custom type, though they DO inherit markers from their base class (e.g., `Node2DMarker` if they extend Node2D). This is by design - custom nodes should use the `BevyBundle` macro for explicit component control.

```rust
// ❌ PlayerMarker is NOT automatically created
fn update_players(players: Query<&GodotNodeHandle, With<PlayerMarker>>) {
    // PlayerMarker doesn't exist unless you create it
}

// ✅ But you CAN use the base class marker
fn update_player_base(players: Query<&GodotNodeHandle, With<CharacterBody2DMarker>>) {
    // This works but includes ALL CharacterBody2D nodes, not just Players
}

// ✅ Use BevyBundle for custom components
#[derive(GodotClass, BevyBundle)]
#[class(base=CharacterBody2D)]
#[bevy_bundle((Player), (Health), (Speed))]
pub struct PlayerNode {
    base: Base<CharacterBody2D>,
}
```

## Creating Markers for Custom Nodes

The recommended approach is to use meaningful components instead of generic markers:

```rust
#[derive(Component)]
struct Player;

#[derive(Component)]
struct Health(f32);

#[derive(Component)]
struct Speed(f32);

#[derive(GodotClass, BevyBundle)]
#[class(base=CharacterBody2D)]
#[bevy_bundle((Player), (Health: max_health), (Speed: speed))]
pub struct PlayerNode {
    base: Base<CharacterBody2D>,
    #[export] max_health: f32,
    #[export] speed: f32,
}

// Now query using your custom components
fn update_players(
    players: Query<(&Health, &Speed), With<Player>>
) {
    for (health, speed) in players.iter() {
        // Process player entities
    }
}
```

You can also leverage the automatic markers from the base class:

```rust
#[derive(Component)]
struct Player;

#[derive(GodotClass, BevyBundle)]
#[class(base=CharacterBody2D)]
#[bevy_bundle((Player))]
pub struct PlayerNode {
    base: Base<CharacterBody2D>,
}

// Query using both the base class marker and your component
fn update_player_bodies(
    players: Query<&GodotNodeHandle, (With<CharacterBody2DMarker>, With<Player>)>
) {
    for handle in players.iter() {
        let mut body = handle.get::<CharacterBody2D>();
        body.move_and_slide();
    }
}
```

## Property Mapping from Godot to Bevy

The `BevyBundle` macro supports several ways to map Godot node properties to Bevy components:

### Default Component Creation

The simplest form creates a component with its default value:

```rust
#[derive(GodotClass, BevyBundle)]
#[class(base=Node2D)]
#[bevy_bundle((Player))]
pub struct PlayerNode {
    base: Base<Node2D>,
}
```

### Single Field Mapping

Map a single Godot property to initialize a component:

```rust
#[derive(Component)]
struct Health(f32);

#[derive(GodotClass, BevyBundle)]
#[class(base=Node2D)]
#[bevy_bundle((Enemy), (Health: max_health), (AttackDamage: damage))]
pub struct Goblin {
    base: Base<Node2D>,
    #[export] max_health: f32,  // This value initializes Health component
    #[export] damage: f32,       // This value initializes AttackDamage component
}
```

### Struct Component Mapping

Map multiple Godot properties to fields in a struct component:

```rust
#[derive(Component)]
struct Stats {
    health: f32,
    mana: f32,
    stamina: f32,
}

#[derive(GodotClass, BevyBundle)]
#[class(base=CharacterBody2D)]
#[bevy_bundle((Player), (Stats { health: max_health, mana: max_mana, stamina: max_stamina }))]
pub struct PlayerCharacter {
    base: Base<CharacterBody2D>,
    #[export] max_health: f32,
    #[export] max_mana: f32,
    #[export] max_stamina: f32,
}
```

### Transform Functions

You can apply transformation functions to convert Godot values before they're assigned to components:

```rust
fn percentage_to_fraction(value: f32) -> f32 {
    value / 100.0
}

#[derive(GodotClass, BevyBundle)]
#[class(base=Node2D)]
#[bevy_bundle((Enemy), (Health: health_percentage))]
pub struct Enemy {
    base: Base<Node2D>,
    #[export]
    #[bevy_bundle(transform_with = "percentage_to_fraction")]
    health_percentage: f32,  // Editor shows 0-100, component gets 0.0-1.0
}
```

### Complete Example

```rust
#[derive(Component)]
struct Velocity(Vec2);

#[derive(Component)]
struct Combat {
    damage: f32,
    attack_speed: f32,
    range: f32,
}

fn degrees_to_radians(degrees: f32) -> f32 {
    degrees.to_radians()
}

#[derive(GodotClass, BevyBundle)]
#[class(base=CharacterBody2D)]
#[bevy_bundle(
    (Player),
    (Health: max_health),
    (Velocity: movement_speed),
    (Combat { damage: attack_damage, attack_speed: attack_rate, range: attack_range })
)]
pub struct PlayerNode {
    base: Base<CharacterBody2D>,

    #[export] max_health: f32,
    #[export] movement_speed: Vec2,
    #[export] attack_damage: f32,
    #[export] attack_rate: f32,
    #[export] attack_range: f32,

    #[export]
    #[bevy_bundle(transform_with = "degrees_to_radians")]
    rotation_degrees: f32,  // Can be transformed even if not used in components
}
```

## Summary

- Built-in Godot types get automatic markers (e.g., `Sprite2DMarker`)
- Custom nodes do NOT get automatic markers for their type, but DO inherit base class markers
- Use `BevyBundle` to define components for custom nodes
- Prefer semantic components over generic markers
- Combine base class markers with custom components for powerful queries

This design gives you full control over your ECS architecture while maintaining performance and clarity.
