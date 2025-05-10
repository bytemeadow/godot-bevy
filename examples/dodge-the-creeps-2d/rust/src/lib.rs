use bevy::prelude::*;
use godot_bevy::prelude::{
    godot_prelude::{gdextension, ExtensionLibrary},
    *,
};

pub mod macros;
mod nodes;

#[bevy_app]
fn build_app(app: &mut App) {
    app;
}
