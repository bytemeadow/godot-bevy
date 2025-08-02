#![allow(clippy::type_complexity)]
#![allow(unexpected_cfgs)] // silence potential `tracy_trace` feature config warning brought in by `bevy_app` macro

use bevy::prelude::*;
use godot_bevy::prelude::{
    GodotTransformSyncPlugin,
    godot_prelude::{ExtensionLibrary, gdextension},
    *,
};

use crate::particle_rain::ParticleRainPlugin;

mod container;
mod particle_rain;

/// Performance benchmark comparing pure Godot vs godot-bevy implementations
///
/// This benchmark demonstrates the performance characteristics of different
/// implementations for simple entity management and transform updates.

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotPackedScenePlugin)
        .add_plugins(GodotBevyLogPlugin::default())
        .add_plugins(GodotAssetsPlugin)
        .add_plugins(GodotTransformSyncPlugin::default().without_auto_sync())
        .add_plugins(ParticleRainPlugin);
}
