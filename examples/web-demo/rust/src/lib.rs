//! Web Demo Example
//!
//! This example demonstrates godot-bevy running in a web browser via WebAssembly.
//! It shows a simple rotating sprite controlled by Bevy's ECS while rendered by Godot.
//!
//! ## Building for Web
//!
//! See the README.md in the parent directory for build instructions.

use bevy::prelude::*;
use godot::classes::Sprite2D;
use godot::prelude::*;
use godot_bevy::prelude::*;

/// Entry point for the Bevy application.
/// This is called when the BevyApp node is ready in Godot.
#[bevy_app]
fn build_app(app: &mut App) {
    godot_print!("Web Demo: Initializing Bevy app...");

    app.add_plugins(GodotTransformSyncPlugin::default())
        .add_systems(Update, (setup_rotators, rotate_sprites).chain());

    godot_print!("Web Demo: Bevy app initialized!");
}

/// Marker component for sprites that have been initialized
#[derive(Component)]
struct Rotator {
    speed: f32,
}

/// Marker for sprites that have already been set up
#[derive(Component)]
struct Initialized;

/// System that finds Sprite2D nodes and adds rotation components to them
#[main_thread_system]
#[allow(clippy::type_complexity)]
fn setup_rotators(
    mut commands: Commands,
    mut query: Query<(Entity, &mut GodotNodeHandle), (With<Sprite2DMarker>, Without<Initialized>)>,
) {
    for (entity, mut handle) in query.iter_mut() {
        // Get the node for logging
        let node = handle.get::<Sprite2D>();
        let name = node.get_name();
        godot_print!("Web Demo: Setting up rotator for '{}'", name);

        // Add rotation component with varying speeds based on hierarchy depth
        // This creates a nice visual effect with nested sprites
        let speed = 1.0 + (entity.index() as f32 * 0.5);

        commands
            .entity(entity)
            .insert((Rotator { speed }, Initialized));
    }
}

/// System that rotates all sprites with the Rotator component
fn rotate_sprites(time: Res<Time>, mut query: Query<(&mut Transform, &Rotator)>) {
    let delta = time.delta_secs();

    for (mut transform, rotator) in query.iter_mut() {
        // Rotate around the Z axis (2D rotation)
        transform.rotate_z(rotator.speed * delta);
    }
}
