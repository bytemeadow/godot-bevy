#![allow(clippy::type_complexity)]

use bevy::prelude::*;
use godot_bevy::prelude::{
    godot_prelude::{gdextension, ExtensionLibrary},
    *,
};

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(TimingTestPlugin);
}

struct TimingTestPlugin;

impl Plugin for TimingTestPlugin {
    fn build(&self, app: &mut App) {
        app;
    }
}
