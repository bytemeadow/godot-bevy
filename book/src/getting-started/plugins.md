# Plugin System

godot-bevy follows Bevy's philosophy of opt-in plugins, giving you granular control over which features are included in your build. This results in smaller binaries, better performance, and clearer dependencies.

## Default Behavior

By default, `GodotPlugin` (automatically included by the `#[bevy_app]` macro) only provides minimal core functionality:

- Scene tree management
- Asset loading system
- Basic bridge components

All other features must be explicitly added as plugins.

## Available Plugins

### Core Plugins

- **`GodotCorePlugins`**: Basic godot-bevy setup (Bevy plugins + asset loading + basic scene tree access)
  - Automatically included by `#[bevy_app]` macro
  - Provides foundation for other plugins to work
  - Includes only `GodotSceneTreeRefPlugin` for basic scene tree access

### Scene Tree Plugins

- **`GodotSceneTreeRefPlugin`**: Basic scene tree access
  - Included automatically by `GodotCorePlugins`
  - Provides `SceneTreeRef` system parameter for scene tree access
  - Does not create entities or monitor changes

- **`GodotSceneTreeEventsPlugin`**: Scene tree change monitoring
  - Add if you want to receive events when nodes are added/removed/renamed
  - Provides `EventReader<SceneTreeEvent>` for monitoring scene changes
  - Does not automatically create entities

- **`GodotSceneTreeMirroringPlugin`**: Automatic entity creation
  - Add if you want entities automatically created for scene tree nodes
  - Automatically includes `GodotSceneTreeEventsPlugin`
  - Creates entities with `GodotNodeHandle` and marker components
  - This was the default behavior in v0.7.x

### Optional Feature Plugins

- **`GodotTransformsPlugin`**: Transform synchronization between Bevy and Godot
  - Add if you want to position/move Godot nodes from Bevy systems
  
- **`GodotAudioPlugin`**: Audio system with channels and spatial audio
  - Add if you want to play sounds and music from Bevy systems
  
- **`GodotSignalsPlugin`**: Godot signal → Bevy event bridge
  - Add if you want to respond to Godot signals (button clicks, area entered, etc.) in Bevy systems
  
- **`GodotCollisionsPlugin`**: Collision detection integration
  - Add if you want to detect collisions and physics events in Bevy systems
  
- **`GodotInputEventPlugin`**: Raw Godot input events
  - Add if you want to handle keyboard, mouse, and gamepad input from Godot in Bevy systems
  
- **`BevyInputBridgePlugin`**: Bevy input resources
  - Add if you want to use Bevy's standard input API instead of Godot's input system
  - Automatically includes `GodotInputEventPlugin`
  
- **`GodotPackedScenePlugin`**: Runtime scene spawning
  - Add if you want to spawn/instantiate scenes dynamically from Bevy systems

### Convenience Bundles

- **`GodotDefaultPlugins`**: All plugins enabled
  - Equivalent to the old v0.7.x behavior
  - Use for easy migration or if you need all features

## Usage Examples

### Minimal Setup (Default)

The `#[bevy_app]` macro automatically provides minimal functionality:

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    // Only basic scene tree access and assets are available
    // No automatic entity creation or scene monitoring
    // Perfect for games that manually manage entities
    app.add_systems(Update, my_core_systems);
}
```

### Adding Specific Features

Add only the plugins you need:

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotSceneTreeMirroringPlugin)  // Auto-create entities
        .add_plugins(GodotTransformsPlugin)         // Transform sync
        .add_plugins(GodotAudioPlugin)              // Audio system
        .add_plugins(GodotSignalsPlugin);           // UI signals
    
    app.add_systems(Update, my_game_systems);
}
```

### Everything Enabled (Migration)

For easy migration from v0.7.x or if you need all features:

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotDefaultPlugins);  // All features
    app.add_systems(Update, my_game_systems);
}
```

### Game-Specific Configurations

**Pure ECS Game**:
```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotSceneTreeMirroringPlugin)  // Auto-create entities
        .add_plugins(GodotTransformsPlugin)         // Move entities
        .add_plugins(GodotAudioPlugin);             // Play sounds
    // Skip collision/signals for pure ECS approach
}
```

**Physics Platformer**:
```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotSceneTreeMirroringPlugin)  // Auto-create entities
        .add_plugins(GodotCollisionsPlugin)         // Detect collisions
        .add_plugins(GodotSignalsPlugin)            // Handle signals
        .add_plugins(GodotAudioPlugin);             // Play sounds
    // Skip transforms - use Godot physics directly via GodotNodeHandle
}
```

**UI-Heavy Game**:
```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotSceneTreeMirroringPlugin)  // Auto-create entities
        .add_plugins(GodotSignalsPlugin)            // Button clicks, etc.
        .add_plugins(BevyInputBridgePlugin)         // Input handling
        .add_plugins(GodotAudioPlugin);             // UI sounds
    // Focus on UI interactions and input
}
```

## Plugin Dependencies

Some plugins automatically include their dependencies:

- `GodotSceneTreeMirroringPlugin` → automatically includes `GodotSceneTreeEventsPlugin`
- `BevyInputBridgePlugin` → automatically includes `GodotInputEventPlugin`

This means you don't need to manually add the dependencies if you're already using the higher-level plugin.

## Benefits

### Smaller Binaries
Only compile the features you actually use, resulting in smaller executable sizes.

### Better Performance  
Skip unused systems and resources, reducing runtime overhead.

### Clear Dependencies
Your plugin list explicitly shows what godot-bevy features your game relies on.

### Future-Proof
New optional features can be added without affecting existing minimal builds.

## Choosing the Right Plugins

### Ask Yourself:

1. **Do I want entities automatically created for scene tree nodes?** → Add `GodotSceneTreeMirroringPlugin`
2. **Do I want to monitor scene tree changes without automatic entities?** → Add `GodotSceneTreeEventsPlugin`
3. **Do I want to move/position nodes from Bevy?** → Add `GodotTransformsPlugin`
4. **Do I want to play sounds and music?** → Add `GodotAudioPlugin`  
5. **Do I want to respond to button clicks or other Godot signals?** → Add `GodotSignalsPlugin`
6. **Do I want to detect collisions and physics events?** → Add `GodotCollisionsPlugin`
7. **Do I want to handle keyboard/mouse/gamepad input?** → Add `BevyInputBridgePlugin` or `GodotInputEventPlugin`
8. **Do I want to spawn scenes dynamically at runtime?** → Add `GodotPackedScenePlugin`

### When in Doubt:
Start with `GodotDefaultPlugins` and optimize later by removing unused plugins.

## Migration from v0.7.x

If you're upgrading from v0.7.x, see the [Migration Guide](../migration/v0.7-to-v0.8.md) for detailed migration instructions.

The quickest fix is to add `app.add_plugins(GodotDefaultPlugins)` to your `build_app` function.

## Advanced: Manual Node Management

**For advanced users**: You can create `GodotNodeHandle` components manually without the mirroring plugin if you want full control:

```rust
use godot_bevy::bridge::GodotNodeHandle;

#[bevy_app]
fn build_app(app: &mut App) {
    // Only basic scene tree access - no automatic entity creation
    app.add_systems(Update, manually_create_entities);
}

fn manually_create_entities(
    mut commands: Commands,
    mut scene_tree: SceneTreeRef,
) {
    let some_node = scene_tree.get().get_node_as::<Node>("SomePath");
    
    // Manually create a component with a node handle
    commands.spawn((
        GodotNodeHandle::from_instance_id(some_node.instance_id()),
        MyCustomComponent,
    ));
}
```

However, most users will prefer the automatic scene tree integration provided by `GodotSceneTreeMirroringPlugin`.