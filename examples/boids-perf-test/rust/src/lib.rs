#![allow(clippy::type_complexity)]

use bevy::prelude::*;
use godot_bevy::prelude::{
    godot_prelude::{gdextension, ExtensionLibrary},
    *,
};

use crate::bevy_boids::{BevyBoids, BoidsContainer};

mod bevy_boids;

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
        app.add_systems(Update, check_simulation_running);
    }
}

// check by querying any nodes with BoidsContainer, we'll cast them to BevyBoids and check the property
fn check_simulation_running(mut boids: Query<&mut GodotNodeHandle, With<BoidsContainer>>) {
    for mut handle in boids.iter_mut() {
        // cast to BevyBoids
        let bevy_boids = handle.get::<BevyBoids>();
        if bevy_boids.bind().is_running {
            println!("Simulation is running");
            // TODO: set a resource or state that then begins all our other systems
        } else {
            println!("Simulation is not running");
        }
    }
}
