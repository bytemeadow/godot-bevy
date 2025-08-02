#![allow(clippy::type_complexity)]

use bevy::prelude::*;
use godot_bevy::prelude::{
    godot_prelude::{gdextension, ExtensionLibrary},
    GodotTransformSyncPlugin, *,
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
        .add_plugins(GodotAssetsPlugin)
        .add_plugins(GodotTransformSyncPlugin::default().without_auto_sync())
        .add_plugins(ParticleRainPlugin);
}
