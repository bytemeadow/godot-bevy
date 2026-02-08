#![allow(clippy::type_complexity)]

use bevy::prelude::*;
use godot_bevy::prelude::{
    GodotTransformSyncPlugin,
    godot_prelude::{ExtensionLibrary, gdextension},
    *,
};

use crate::particle_rain::ParticleRainPlugin;

mod container;
mod particle_rain;

/// Transform sync performance benchmark comparing pure Godot vs godot-bevy
/// with tens of thousands of entities requiring position updates each frame.

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotPackedScenePlugin)
        .add_plugins(GodotBevyLogPlugin::default())
        .add_plugins(GodotAssetsPlugin)
        .add_plugins(GodotTransformSyncPlugin::default().without_auto_sync())
        .add_plugins(ParticleRainPlugin);
}
