#![allow(clippy::type_complexity)]

use crate::bevy_boids::BoidsPlugin;
use bevy::prelude::App;
use godot::prelude::gdextension;
use godot_bevy::prelude::godot_prelude::ExtensionLibrary;
use godot_bevy::prelude::{bevy_app, GodotPackedScenePlugin, GodotTransformSyncPlugin};

mod bevy_boids;
mod container;

/// Performance benchmark comparing pure Godot vs godot-bevy boids implementations
///
/// This benchmark demonstrates the performance benefits of using Bevy's ECS
/// for computationally intensive tasks like boids simulation.

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotPackedScenePlugin)
        .add_plugins(GodotTransformSyncPlugin::default())
        .add_plugins(BoidsPlugin);
}
