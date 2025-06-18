# Scene Tree Initialization and Timing

The godot-bevy library automatically parses the Godot scene tree and creates corresponding Bevy entities before your game logic runs. This means you can safely query for scene entities in your `Startup` systems:

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_systems(Startup, find_player);
}

fn find_player(query: Query<&PlayerBundle>) {
    // Your player entity will be here! ✨
    for player in &query {
        println!("Found the player!");
    }
}
```

## How It Works

The scene tree initialization happens in the `PreStartup` schedule, ensuring entities are ready before any `Startup` systems run. This process has two parallel systems:

1. **`initialize_scene_tree`** - Traverses the entire Godot scene tree and creates Bevy entities with components like `GodotNodeHandle`, `Name`, transforms, and more
2. **`connect_scene_tree`** - Sets up event listeners for runtime scene changes (nodes being added, removed, or renamed)

Both systems run in parallel during `PreStartup`, and both complete before your `Startup` systems run. This means you can safely query for Godot scene entities in `Startup`!

## Runtime Scene Updates

After the initial parse, the library continues to listen for scene tree changes during runtime. This is handled by two systems that run in the `First` schedule:

- **`write_scene_tree_events`** - Receives events from Godot (via an mpsc channel) and writes them to Bevy's event system
- **`read_scene_tree_events`** - Processes those events to create/update/remove entities

This separation allows other systems to also react to `SceneTreeEvent`s if needed.

## What Components Are Available?

When the scene tree is parsed, each Godot node becomes a Bevy entity with these components:

- **`GodotNodeHandle`** - Reference to the Godot node
- **`Name`** - The node's name from Godot
- **`Transform2D`** or **`Transform3D`** - For Node2D and Node3D types respectively
- **`Groups`** - The node's group memberships
- **`Collisions`** - If the node has collision signals
- **Node type markers** - Components like `ButtonMarker`, `Sprite2DMarker`, etc.
- **Custom bundles** - Components from `#[derive(BevyBundle)]` are automatically added

## BevyBundle Component Timing

If you've defined custom Godot node types with `#[derive(BevyBundle)]`, their components are added **immediately** during scene tree processing. This happens:

- **During `PreStartup`** for nodes that exist when the scene is first loaded
- **During `First`** for nodes added dynamically at runtime

This means BevyBundle components are available in `Startup` systems for initial scene nodes, and immediately available for dynamically added nodes.

```rust
#[derive(GodotClass, BevyBundle)]
#[class(base=Node2D)]
#[bevy_bundle((Health), (Velocity))]
pub struct Player {
    base: Base<Node2D>,
}

// This will work in Startup - the Health and Velocity components
// are automatically added during PreStartup for existing nodes
fn setup_player(mut query: Query<(Entity, &Health, &Velocity)>) {
    for (entity, health, velocity) in &mut query {
        // Player components are guaranteed to be here!
    }
}
```

## Best Practices

1. **Use `Startup` for initialization** - Scene entities are guaranteed to be ready
2. **Use `Update` for gameplay logic** - This is where most of your game code should live
3. **Custom `PreStartup` systems** - If you add systems to `PreStartup`, be aware they run before scene parsing unless you explicitly order them with `.after()`

## Understanding the Event Flow

Here's what happens when a node is added to the scene tree during runtime:

1. Godot emits a `node_added` signal
2. The `SceneTreeWatcher` (on the Godot side) receives the signal
3. It sends a `SceneTreeEvent` through an mpsc channel
4. `write_scene_tree_events` (in `First` schedule) reads from the channel and writes to Bevy's event system
5. `read_scene_tree_events` (also in `First` schedule) processes the event and creates/updates entities

This architecture allows for flexible event handling while maintaining a clean separation between Godot and Bevy.

## Dynamic Scene Spawning Timing

When you spawn scenes dynamically using `GodotScene::from_handle()`, there's an important timing consideration to be aware of:

```rust
fn spawn_enemy(mut commands: Commands, enemy_scene: Res<EnemyScene>) {
    let entity = commands.spawn_empty()
        .insert(GodotScene::from_handle(enemy_scene.0.clone()))
        .insert(NeedsInitialization)
        .id();
    
    // The entity exists but the Godot node hasn't been created yet!
    // This system will run before the node is available
}
```

### The Timeline

Here's what happens when you spawn with `GodotScene::from_handle()`:

1. **Frame N, Update**: Entity created with `GodotScene` component
2. **Frame N, PostUpdate**: `spawn_scene` system creates Godot node, adds `GodotNodeHandle`
3. **Frame N+1, First**: Scene tree events register entity in NodeRegistry
4. **Frame N+1**: Entity is fully ready for NodeRegistry access

### Working with Newly Spawned Scenes

If you need to work with newly spawned scenes immediately, use `With<GodotNodeHandle>` in your queries to ensure entities are ready:

```rust
// ❌ This might process entities before their nodes are ready
fn initialize_enemies(
    enemies: Query<Entity, With<NeedsInitialization>>,
    registry: NodeRegistryAccess,
) {
    for entity in enemies.iter() {
        // This might fail if the node isn't ready yet!
        if let Some(enemy) = registry.try_access::<EnemyNode>(entity) {
            // Initialize enemy...
        }
    }
}

// ✅ This only processes entities with ready nodes
fn initialize_enemies(
    enemies: Query<Entity, (With<NeedsInitialization>, With<GodotNodeHandle>)>,
    registry: NodeRegistryAccess,
) {
    for entity in enemies.iter() {
        // This is guaranteed to work since we require GodotNodeHandle
        let enemy = registry.access::<EnemyNode>(entity);
        // Initialize enemy...
    }
}
```

### Schedule Considerations

If you absolutely need to process newly spawned scenes in the same frame, run your systems in the `First` schedule after scene tree processing:

```rust
app.add_systems(
    First,
    initialize_enemies.after(bevy::ecs::event::event_update_system),
);
```

This ensures your system runs after the scene tree events have been processed and entities are registered in the NodeRegistry.