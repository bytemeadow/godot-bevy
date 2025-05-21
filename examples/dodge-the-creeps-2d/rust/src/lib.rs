use bevy::{prelude::*, state::app::StatesPlugin};
use godot_bevy::prelude::{
    godot_prelude::{gdextension, ExtensionLibrary},
    *,
};

mod gameplay;
mod main_menu;
mod nodes;

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(StatesPlugin)
        .init_state::<GameState>()
        .init_resource::<Score>()
        .add_plugins(main_menu::MainMenuPlugin)
        .add_plugins(gameplay::GameplayPlugin);
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    MainMenu,
    Countdown,
    InGame,
    GameOver,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default, Resource)]
pub struct Score(i64);
