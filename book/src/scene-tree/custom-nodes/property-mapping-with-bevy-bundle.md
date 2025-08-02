# Property Mapping from Godot to Bevy

The `BevyBundle` macro allows you to attach Bevy Components to Custom Godot nodes.
It supports several ways to map Godot node properties to Bevy components:

#### Default Component Creation

The simplest form creates a component with its default value:

```rust
#[derive(GodotClass, BevyBundle)]
#[class(base=Node2D)]
#[bevy_bundle((Player))]
pub struct PlayerNode {
    base: Base<Node2D>,
}
```

#### Single Field Mapping

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

#### Struct Component Mapping

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



## Transform Function

Sometimes a Godot property's type isn't convertable to a Bevy/Rust compatible type,
or maybe you want to process the value from Godot before it's assigned to a component.
To solve this, you can use `transform_with` to apply a transformation function to
convert Godot values before they're assigned to components:

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



## Recommended approach

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



# Complete Example

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
