#![allow(clippy::type_complexity)]

use bevy::{prelude::*, state::app::StatesPlugin};
use bevy_asset_loader::prelude::*;
use gameplay::{audio::GameAudio, mob::MobAssets, player::PlayerAssets};
use godot_bevy::prelude::{
    godot_prelude::{gdextension, ExtensionLibrary},
    *,
};

mod gameplay;
mod main_menu;
mod nodes;

#[bevy_app]
fn build_app(app: &mut App) {
    // Note: Asset loading with path verification bypass is now handled automatically
    // by GodotCorePlugin, so Bevy's asset_server can load Godot resources from .pck files

    app.add_plugins(StatesPlugin)
        .init_state::<GameState>()
        .add_loading_state(
            LoadingState::new(GameState::Loading)
                .continue_to_state(GameState::MainMenu)
                .load_collection::<PlayerAssets>()
                .load_collection::<MobAssets>()
                .load_collection::<GameAudio>(),
        )
        .init_resource::<Score>()
        .init_resource::<gameplay::audio::GameAudioState>()
        .add_plugins(main_menu::MainMenuPlugin)
        .add_plugins(gameplay::GameplayPlugin);
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    Loading,
    MainMenu,
    Countdown,
    InGame,
    GameOver,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Default, Resource)]
pub struct Score(i64);
