use bevy::{prelude::*, state::app::StatesPlugin};
use bevy_asset_loader::prelude::*;
use gameplay::audio::GameAudio;
use godot_bevy::prelude::{
    GodotDefaultPlugins,
    godot_prelude::{ExtensionLibrary, gdextension},
    *,
};
use godot_bevy_inspector::InspectorPlugin;

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
        // Add the inspector plugin - press F12 to toggle
        .add_plugins(InspectorPlugin::default())
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
        ))
        // Register types for inspector reflection
        .register_type::<components::Speed>()
        .register_type::<components::JumpVelocity>()
        .register_type::<components::Gravity>()
        .register_type::<components::Player>();
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    Loading,
    MainMenu,
    InGame,
}
