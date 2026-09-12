#![allow(clippy::type_complexity)]

use bevy::prelude::*;
use godot_bevy::prelude::{GodotTransformSyncPlugin, *};

use crate::particle_rain::ParticleRainPlugin;

mod container;
mod particle_rain;

#[cfg(feature = "capture")]
mod capture;

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(GodotPackedScenePlugin)
        .add_plugins(GodotBevyLogPlugin::default())
        .add_plugins(GodotAssetsPlugin)
        .add_plugins(GodotTransformSyncPlugin::default().without_auto_sync())
        .add_plugins(ParticleRainPlugin);

    #[cfg(feature = "capture")]
    if godot_bevy_test::capture::is_enabled() {
        capture::install(app);
    }
}
