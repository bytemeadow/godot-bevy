#![allow(clippy::type_complexity)]

use bevy::prelude::*;
use godot_bevy::prelude::{
    godot_prelude::{gdextension, ExtensionLibrary},
    *,
};

/// Performance benchmark comparing pure Godot vs godot-bevy boids implementations
///
/// This benchmark demonstrates the performance benefits of using Bevy's ECS
/// for computationally intensive tasks like boids simulation.

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(BoidsBenchmarkPlugin);
}

pub struct BoidsBenchmarkPlugin;

impl Plugin for BoidsBenchmarkPlugin {
    fn build(&self, app: &mut App) {
        app;
    }
}
