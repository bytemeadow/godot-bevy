#![allow(clippy::type_complexity)]

use crate::bevy_boids::BoidsPlugin;
use bevy::prelude::App;
use godot::prelude::gdextension;
use godot_bevy::prelude::godot_prelude::ExtensionLibrary;
use godot_bevy::prelude::{
    GodotAssetsPlugin, GodotPackedScenePlugin, GodotTransformSyncPlugin,
    GodotTransformSyncPluginExt, bevy_app,
};

mod bevy_boids;
mod container;

// TODO move this, it should be visible from the bevy_app macro, without every client injecting it
#[cfg(feature = "profiling")]
// Single global handle; will be initialised exactly once.
static TRACY_CLIENT: std::sync::OnceLock<tracing_tracy::client::Client> =
    std::sync::OnceLock::new();

/// Performance benchmark comparing pure Godot vs godot-bevy boids implementations
///
/// This benchmark demonstrates the performance benefits of using Bevy's ECS
/// for computationally intensive tasks like boids simulation.

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotAssetsPlugin)
        .add_plugins(GodotPackedScenePlugin)
        .add_plugins(GodotTransformSyncPlugin::default().without_auto_sync())
        .add_plugins(BoidsPlugin);
}
