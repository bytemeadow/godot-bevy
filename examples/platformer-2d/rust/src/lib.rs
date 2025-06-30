use bevy::{prelude::*, state::app::StatesPlugin};
use bevy_asset_loader::prelude::*;
use gameplay::audio::GameAudio;
use godot_bevy::prelude::{
    godot_prelude::{gdextension, ExtensionLibrary},
    GodotDefaultPlugins, *,
};

mod components;
mod gameplay;
mod level_manager;
mod main_menu;
mod scene_management;

#[bevy_app]
fn build_app(app: &mut App) {
    // This example uses most godot-bevy features
    app.add_plugins(GodotDefaultPlugins)
        .add_plugins(StatesPlugin)
        .init_state::<GameState>()
        .add_loading_state(
            LoadingState::new(GameState::Loading)
                .continue_to_state(GameState::MainMenu)
                .load_collection::<GameAudio>(),
        )
        .add_plugins((
            scene_management::SceneManagementPlugin,
            main_menu::MainMenuPlugin,
            level_manager::LevelManagerPlugin,
            gameplay::GameplayPlugin,
        ));
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    Loading,
    MainMenu,
    InGame,
}
