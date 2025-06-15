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

You can also map properties from Godot Node exported vars to component values:

```rust
// ✅ Good: Meaningful components with property mapping
#[derive(GodotClass, BevyBundle)]
#[class(base=Node2D)]
#[bevy_bundle((Enemy), (Health: max_health), (AttackDamage: damage))]
pub struct Goblin {
    base: Base<Node2D>,
    #[export] max_health: f32,  // This value initializes Health component
    #[export] damage: f32,       // This value initializes AttackDamage component
}
```

## Summary

- Built-in Godot types get automatic markers (e.g., `Sprite2DMarker`)
- Custom nodes do NOT get automatic markers for their type, but DO inherit base class markers
- Use `BevyBundle` to define components for custom nodes
- Prefer semantic components over generic markers
- Combine base class markers with custom components for powerful queries

This design gives you full control over your ECS architecture while maintaining performance and clarity.