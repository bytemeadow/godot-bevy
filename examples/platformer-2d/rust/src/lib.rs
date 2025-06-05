use bevy::{prelude::*, state::app::StatesPlugin};
use bevy_asset_loader::loading_state::{LoadingState, LoadingStateAppExt};
use godot_bevy::prelude::{
    godot_prelude::{gdextension, ExtensionLibrary},
    *,
};

mod main_menu;

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(StatesPlugin)
    .init_state::<GameState>()
    .add_loading_state(
        LoadingState::new(GameState::Loading)
            .continue_to_state(GameState::MainMenu)
    )
    .add_plugins(main_menu::MainMenuPlugin);
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    Loading,
    MainMenu,
    InGame,
    GameOver,
}
