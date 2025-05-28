# Automatic Component Generation

The `#[derive(BevyComponent)]` macro automatically generates Bevy components from Godot exported properties, eliminating all boilerplate and providing a completely automatic syncing experience.

## Quick Start

Transform this manual boilerplate:

```rust
// OLD WAY - Manual everything
#[derive(GodotClass)]
pub struct Player {
    #[export] speed: f32,
}

#[derive(Component)]
pub struct PlayerComponent {
    speed: f32,  // Duplicate!
}

// Manual syncing in systems
fn sync_player(mut commands: Commands, mut player: Query<&mut GodotNodeHandle>) {
    player_component.speed = godot_node.bind().get_speed(); // Manual sync!
}
```

Into this automatic solution:

```rust
// NEW WAY - Completely automatic!
#[derive(GodotClass, BevyComponent)]
#[bevy_component("Player")]  // Custom component name
pub struct PlayerNode {
    #[export] speed: f32,
}

// Everything else is auto-generated:
// - Player component struct
// - Auto-sync plugin
// - All sync methods

// Setup once in your app
app.add_plugins(PlayerPlugin);  // Auto-generated!

// Spawn anywhere
commands.spawn(GodotSceneWithComponent::<Player>::from_resource(asset));
// Speed is automatically synced when Godot node becomes ready!
```

## Generated Code

The macro automatically generates **everything** you need:

### 1. Component Struct
```rust
#[derive(Component, Debug, Clone)]
pub struct Player {  // Custom name from #[bevy_component("Player")]
    pub speed: f32,  // From #[export] property
}

impl Default for Player {
    fn default() -> Self {
        Self { speed: 0.0 }  // Smart defaults for common types
    }
}
```

### 2. Sync Methods
```rust
impl Player {
    /// Create component synced from Godot node
    pub fn from_godot(godot_node: &mut GodotNodeHandle) -> Self { ... }
    
    /// Update component from Godot node
    pub fn sync_from_godot(&mut self, godot_node: &mut GodotNodeHandle) { ... }
    
    /// Auto-sync method (used internally)
    pub fn auto_sync(&mut self, godot_node: &mut GodotNodeHandle) { ... }
}
```

### 3. Auto-Sync Plugin
```rust
pub struct PlayerAutoSyncPlugin;  // Clear, no conflicts!

impl Plugin for PlayerAutoSyncPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, |mut query: Query<(&mut Player, &mut GodotNodeHandle), Added<GodotNodeHandle>>| {
            for (mut component, mut handle) in query.iter_mut() {
                component.auto_sync(&mut handle);  // Automatic!
            }
        });
    }
}
```

## Component Naming

Control the generated component name with `#[bevy_component("Name")]`:

### Default Naming
```rust
#[derive(GodotClass, BevyComponent)]
pub struct Player {
    #[export] speed: f32,
}
// Generates: 
// - Component: PlayerComponent
// - Plugin: PlayerComponentAutoSyncPlugin
```

### Custom Naming
```rust
#[derive(GodotClass, BevyComponent)]
#[bevy_component("PlayerData")]
pub struct Player {
    #[export] speed: f32,
}
// Generates:
// - Component: PlayerData  
// - Plugin: PlayerDataAutoSyncPlugin
```

### Clean Naming (Recommended)
```rust
#[derive(GodotClass, BevyComponent)]
#[bevy_component("Player")]     // Clean component name
pub struct PlayerNode {         // Descriptive Godot node name  
    #[export] speed: f32,
}
// Generates:
// - Component: Player
// - Plugin: PlayerAutoSyncPlugin (clear!)
```

## Property Selection

Control which exported properties are synced:

### Sync All Exports (Default)
```rust
#[derive(GodotClass, BevyComponent)]
pub struct PlayerNode {
    #[export] speed: f32,     // ✅ Synced
    #[export] health: i32,    // ✅ Synced
    internal_state: String,   // ❌ Not exported, not synced
}
```

### Selective Syncing
```rust
#[derive(GodotClass, BevyComponent)]
pub struct PlayerNode {
    #[export] #[sync] speed: f32,        // ✅ Explicitly synced
    #[export] editor_debug: String,      // ❌ Exported but not synced
    #[export] #[sync] health: i32,       // ✅ Explicitly synced
}
// Generated component only has: speed, health
```

## Supported Types

Automatic smart defaults for common Godot types:

| Godot Type | Default Value | Notes |
|------------|---------------|-------|
| `f32`, `f64` | `0.0` | Numeric defaults |
| `i32`, `i64`, `u32`, `u64` | `0` | Integer defaults |
| `bool` | `false` | Boolean default |
| `String` | `String::new()` | Empty string |
| `Vector2` | `Vector2::ZERO` | Godot vector zero |
| `Vector3` | `Vector3::ZERO` | Godot vector zero |
| Custom types | `Default::default()` | Uses type's default |

## Complete Usage Example

```rust
use godot_bevy::prelude::*;

// 1. Define Godot node with macro
#[derive(GodotClass, BevyComponent)]
#[class(base=Area2D)]
#[bevy_component("Player")]
pub struct PlayerNode {
    base: Base<Area2D>,
    #[export] speed: f32,
    #[export] max_health: i32,
    #[export] jump_height: f32,
}

// 2. Add auto-generated plugin once
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(PlayerAutoSyncPlugin)
        .add_plugins(PlayerPlugin)  // Your gameplay logic
        .add_systems(Update, move_player);
}

// 3. Spawn with smart bundle
fn spawn_player(mut commands: Commands, assets: Res<GameAssets>) {
    commands.spawn(GodotSceneWithComponent::<Player>::from_resource(
        assets.player_scene.clone()
    ));
    // Component automatically synced when Godot node becomes ready!
}

// 4. Use clean component in systems  
fn move_player(
    mut players: Query<(&Player, &mut Transform2D)>,
    input: Res<Input<KeyCode>>,
) {
    for (player_data, mut transform) in players.iter_mut() {
        if input.pressed(KeyCode::Right) {
            transform.translation.x += player_data.speed * time.delta_seconds();
        }
        // player_data.speed automatically synced from Godot!
    }
}
```
