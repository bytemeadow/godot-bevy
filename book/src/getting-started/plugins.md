# Plugin System

godot-bevy follows Bevy's philosophy of opt-in plugins, giving you granular control over which features are included in your build. This results in smaller binaries, better performance, and clearer dependencies.

## Default Behavior

By default, `GodotPlugin` (automatically included by the `#[bevy_app]` macro) only provides minimal core functionality through `GodotCorePlugins`:

- Scene tree management (automatic entity mirroring)
- Asset loading system
- Basic Bevy setup

All other features must be explicitly added as plugins.

## Plugin Groups

- **`GodotCorePlugins`**: Minimal required functionality

  - Automatically included by `#[bevy_app]` macro via `GodotPlugin`
  - Includes:
    - `GodotBaseCorePlugin`: Bevy MinimalPlugins, logging, diagnostics, schedules
    - `GodotSceneTreePlugin`: Scene tree entity mirroring and management

- **`GodotDefaultPlugins`**: Contains all plugins typically necessary for building a game
  - Includes:
    - `GodotAssetsPlugin`: Godot resource loading through Bevy's asset system
    - `GodotTransformSyncPlugin`: Transform synchronization
    - `GodotCollisionsPlugin`: Collision detection
    - `GodotSignalsPlugin`: Signal to event bridge
    - `BevyInputBridgePlugin`: Bevy input API support
    - `GodotAudioPlugin`: Audio system
    - `GodotPackedScenePlugin`: Runtime scene spawning
    - `GodotBevyLogPlugin`: Unify/improve bevy and godot logging such that `info!`, `debug!`, etc log messages are visible in the Godot Editor

## Available Plugins

### Core Infrastructure (Included by Default)

- **`GodotBaseCorePlugin`**: Foundation setup

  - Bevy MinimalPlugins (without ScheduleRunnerPlugin)
  - Asset system with Godot resource reader
  - Logging and diagnostics
  - Physics update schedule
  - Main thread marker resource

- **`GodotSceneTreePlugin`**: Scene tree management

  - Automatic entity creation for scene nodes
  - Scene tree change monitoring
  - Transform component addition (configurable)
  - AutoSync bundle registration
  - Groups component for Godot groups

### Additional Plugins

- **`GodotAssetsPlugin`**: Asset loading

  - Load Godot resources through Bevy's AssetServer
  - Supports .tscn, .tres, textures, sounds, etc.
  - Development and export path handling

- **`GodotTransformSyncPlugin`**: Transform synchronization

  - Configure sync mode: `Disabled`, `OneWay` (default), or `TwoWay`
  - Synchronizes Bevy Transform components with Godot node transforms
  - Required for moving/positioning nodes from Bevy

- **`GodotCollisionsPlugin`**: Collision detection

  - Monitors Area2D/3D and RigidBody2D/3D collision signals
  - Provides `Collisions` component with entered/exited tracking
  - Converts Godot collision signals to queryable data

- **`GodotSignalsPlugin`**: Signal event bridge

  - Converts Godot signals to Bevy events
  - Use `EventReader<GodotSignal>` to handle signals
  - Essential for UI interactions (button clicks, etc.)

- **`GodotInputEventPlugin`**: Raw input events

  - Provides Godot input as Bevy events
  - Keyboard, mouse, touch, gamepad, and action events
  - Lower-level alternative to `BevyInputBridgePlugin`

- **`BevyInputBridgePlugin`**: Bevy input API

  - Use Bevy's standard `ButtonInput<KeyCode>`, mouse events, etc.
  - Automatically includes `GodotInputEventPlugin`
  - Higher-level, more ergonomic than raw events

- **`GodotAudioPlugin`**: Audio system

  - Channel-based audio API
  - Spatial audio support
  - Audio tweening and easing
  - Integrates with Godot's audio engine

- **`GodotPackedScenePlugin`**: Scene spawning

  - Spawn/instantiate scenes at runtime
  - Support for both asset handles and paths
  - Automatic transform application

- **`GodotBevyLogPlugin`**: Improved logging by default

  - Log message components are color-coded for readability by default. Color coding can be disabled entirely. NOTE: There is a performance penalty for color-coding, so if your application is very performance sensitive, consider disabling this feature
  - Log messages are prefixed with a short timestamp, e.g., `12:00:36.196`. Timestamps can be customized or entirely disabled
  - Log messages are prefixed with a short log level, e.g., `T` for `TRACE`, `D` for `DEBUG`, `I` for `INFO`, `W` for `WARN`, `E` for `ERROR`
  - Log messages are suffixed with a shortened path and line number location, e.g., `@ loading_state/systems.rs:186`
  - Log level filtering is `INFO` and higher severity by default, this can be customized directly in your code or set at runtime using `RUST_LOG`, e.g., `RUST_LOG=trace cargo run`

## Usage Examples

### Minimal Setup (Default)

The `#[bevy_app]` macro automatically provides core functionality:

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    // GodotCorePlugins is already added
    // You have scene tree, assets, and basic setup
    app.add_systems(Update, my_game_system);
}
```

### Adding Specific Features

Add only the plugins you need:

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotTransformSyncPlugin::default())  // Move nodes
        .add_plugins(GodotAudioPlugin)                    // Play sounds
        .add_plugins(BevyInputBridgePlugin);              // Handle input

    app.add_systems(Update, my_game_systems);
}
```

### Everything Enabled

For all features or easy migration from older versions:

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotDefaultPlugins);  // All optional features
    app.add_systems(Update, my_game_systems);
}
```

### Game-Specific Configurations

**Pure ECS Game**:

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotTransformSyncPlugin::default())  // Move entities
        .add_plugins(GodotAudioPlugin)                    // Play sounds
        .add_plugins(BevyInputBridgePlugin);              // Input handling
    // Core plugins handle entity creation
}
```

**Physics Platformer**:

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotTransformSyncPlugin {
            sync_mode: TransformSyncMode::Disabled,  // Use Godot physics
        })
        .add_plugins(GodotCollisionsPlugin)         // Detect collisions
        .add_plugins(GodotSignalsPlugin)            // Handle signals
        .add_plugins(GodotAudioPlugin);             // Play sounds
}
```

**UI-Heavy Game**:

```rust
#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotSignalsPlugin)            // Button clicks, etc.
        .add_plugins(BevyInputBridgePlugin)        // Keyboard shortcuts
        .add_plugins(GodotAudioPlugin);            // UI sounds
    // Don't need transform sync for UI
}
```

## Plugin Configuration

### Transform Sync Modes

```rust
// Default: One-way sync (Bevy → Godot)
app.add_plugins(GodotTransformSyncPlugin::default());

// Two-way sync (Bevy ↔ Godot)
app.add_plugins(GodotTransformSyncPlugin {
    sync_mode: TransformSyncMode::TwoWay,
});

// Disabled (use Godot physics directly)
app.add_plugins(GodotTransformSyncPlugin {
    sync_mode: TransformSyncMode::Disabled,
});
```

### Scene Tree Configuration

```rust
// Configure transform component creation
app.add_plugins(GodotSceneTreePlugin::default());
```

Note: This is already included in `GodotCorePlugins`, so you'd need to disable the default `GodotPlugin` and build your own plugin setup to customize this.

## Plugin Dependencies

Some plugins automatically include their dependencies:

- `BevyInputBridgePlugin` → includes `GodotInputEventPlugin`
- `GodotPlugin` → includes `GodotCorePlugins`

## Choosing the Right Plugins

### Ask Yourself:

1. **Do I want to load Godot resources through Bevy's asset system?** → Add `GodotAssetsPlugin`
2. **Do I want to move/position nodes from Bevy?** → Add `GodotTransformSyncPlugin`
3. **Do I want to play sounds and music?** → Add `GodotAudioPlugin`
4. **Do I want to respond to UI signals?** → Add `GodotSignalsPlugin`
5. **Do I want to detect collisions?** → Add `GodotCollisionsPlugin`
6. **Do I want to handle input?** → Add `BevyInputBridgePlugin` or `GodotInputEventPlugin`
7. **Do I want to spawn scenes at runtime?** → Add `GodotPackedScenePlugin`

### When in Doubt:

Start with `GodotDefaultPlugins` and optimize later by removing unused plugins.

## Benefits

### Smaller Binaries

Only compile the features you actually use.

### Better Performance

Skip unused systems and resources.

### Clear Dependencies

Your plugin list shows exactly what features you're using.

### Future-Proof

New optional features can be added without breaking existing code.

## Migration Note

If upgrading from an older version where all features were included by default, simply add:

```rust
app.add_plugins(GodotDefaultPlugins);
```

This restores the old behavior with all features enabled.
