use bevy::{prelude::*, state::app::StatesPlugin};
use bevy_asset_loader::prelude::*;
use gameplay::audio::GameAudio;
use godot_bevy::prelude::{GodotDefaultPlugins, *};

mod attachables;
mod components;
mod gameplay;
mod level_manager;
mod main_menu;
mod scene_management;

// ANCHOR: itest
#[cfg(feature = "itest")]
mod itests;

#[cfg(feature = "itest")]
godot_bevy_test::declare_test_runner!();
// ANCHOR_END: itest

#[bevy_app]
fn build_app(app: &mut App) {
    // This example uses most godot-bevy features
    app.add_plugins(GodotDefaultPlugins)
        .add_plugins(GodotActionsPlugin)
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
        ))
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
