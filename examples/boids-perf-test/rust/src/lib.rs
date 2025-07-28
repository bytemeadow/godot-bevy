#![allow(clippy::type_complexity)]
#![allow(unexpected_cfgs)] // silence potential `tracy_trace` feature config warning brought in by `bevy_app` macro

use crate::bevy_boids::BoidsPlugin;
use bevy::prelude::App;
use godot::prelude::gdextension;
use godot_bevy::prelude::godot_prelude::ExtensionLibrary;
use godot_bevy::prelude::{
    GodotAssetsPlugin, GodotBevyLogPlugin, GodotPackedScenePlugin, GodotTransformSyncPlugin,
    GodotTransformSyncPluginExt, bevy_app,
};

mod bevy_boids;
mod container;

/// Performance benchmark comparing pure Godot vs godot-bevy boids implementations
///
/// This benchmark demonstrates the performance benefits of using Bevy's ECS
/// for computationally intensive tasks like boids simulation.

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotAssetsPlugin)
        .add_plugins(GodotPackedScenePlugin)
        .add_plugins(GodotBevyLogPlugin::default())
        .add_plugins(GodotTransformSyncPlugin::default().without_auto_sync())
        .add_plugins(BoidsPlugin);
}
