use bevy::prelude::*;
use godot_bevy::prelude::*;

/// This example demonstrates the new opt-in plugin architecture.
/// You can now choose exactly which Godot-Bevy functionality you need.

fn main() {
    // Option 1: Minimal core functionality only
    // This includes scene tree management, asset loading, and basic bridge components
    App::new()
        .add_plugins(GodotCorePlugins)
        // Your game systems here...
        .run();

    // Option 2: Add specific functionality you need
    App::new()
        .add_plugins(GodotCorePlugins)
        .add_plugins(GodotTransformsPlugin)  // Transform sync between Bevy and Godot
        .add_plugins(GodotAudioPlugin)       // Audio system
        .add_plugins(GodotSignalsPlugin)     // Godot signal → Bevy event bridge
        // Your game systems here...
        .run();

    // Option 3: All functionality (equivalent to old DefaultGodotPlugin)
    App::new()
        .add_plugins(GodotDefaultPlugins)
        // Your game systems here...
        .run();

    // Option 4: Fine-grained control for specific use cases
    App::new()
        .add_plugins(GodotCorePlugins)
        .add_plugins((
            GodotTransformsPlugin,
            GodotCollisionsPlugin,
            // Skip audio and input for a lightweight physics simulation
        ))
        // Your game systems here...
        .run();
}

/// Example system that works with the minimal core
fn minimal_example_system(
    // Scene tree is always available with GodotCorePlugins
    scene_tree: Res<SceneTreeRef>,
    // Asset loading is always available
    asset_server: Res<AssetServer>,
) {
    // Load a scene (this works with just GodotCorePlugins)
    let _scene: Handle<GodotResource> = asset_server.load("scenes/example.tscn");
    
    // Access scene tree
    let _root = scene_tree.get_root();
}

/// Example system that requires specific plugins
fn transform_example_system(
    // This requires GodotTransformsPlugin
    mut transforms: Query<&mut Transform2D>,
) {
    for mut transform in transforms.iter_mut() {
        // Modify transform - will sync to Godot automatically
        transform.as_godot_mut().origin.x += 1.0;
    }
}

/// Example system that uses audio
fn audio_example_system(
    // This requires GodotAudioPlugin
    mut audio: Audio,
) {
    // Play a sound
    audio.play("sounds/example.ogg");
}