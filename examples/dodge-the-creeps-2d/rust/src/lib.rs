use bevy::{prelude::*, state::app::StatesPlugin};
use godot_bevy::prelude::{
    godot_prelude::{gdextension, ExtensionLibrary},
    *,
};

mod gameplay;
mod nodes;

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(StatesPlugin)
        .init_state::<GameState>()
        .add_plugins(gameplay::GameplayPlugin);
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    InGame,
}
