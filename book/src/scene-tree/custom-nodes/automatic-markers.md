# Automatic Markers

godot-bevy **automatically** creates marker components for all built-in Godot node types:

```rust
// These markers are created automatically:
// Sprite2DMarker, CharacterBody2DMarker, Area2DMarker, etc.

fn update_sprites(sprites: Query<&GodotNodeHandle, With<Sprite2DMarker>>) {
    // Works automatically for any Sprite2D in your scene
}
```

### Custom Godot Nodes

Custom nodes defined in Rust or GDScript **do NOT** receive automatic markers for their custom type,
though they DO inherit markers from their base class (e.g., `Node2DMarker` if they extend Node2D).
This is by design—custom nodes should use the `BevyBundle` macro for explicit component control.

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
