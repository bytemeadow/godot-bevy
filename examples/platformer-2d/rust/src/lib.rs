use bevy::{prelude::*, state::app::StatesPlugin};
use bevy_asset_loader::prelude::*;
use gameplay::audio::GameAudio;
use godot_bevy::prelude::{GodotDefaultPlugins, *};

mod attachables;
#[cfg(feature = "capture-audio")]
mod capture_audio;
mod components;
mod gameplay;
mod level_manager;
mod main_menu;
mod scene_management;

#[cfg(feature = "capture")]
mod capture;
#[cfg(feature = "capture-input")]
mod capture_input;

// ANCHOR: itest
#[cfg(feature = "itest")]
mod itests;

#[cfg(feature = "itest")]
godot_bevy_test::declare_test_runner!();
// ANCHOR_END: itest

#[bevy_app]
fn build_app(app: &mut App) {
    #[cfg(feature = "capture-input")]
    if capture_input::is_selected() {
        capture_input::build(app);
        return;
    }

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
        ));

    #[cfg(feature = "capture")]
    capture::install(app);
    #[cfg(feature = "capture-audio")]
    capture_audio::install(app);
}

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    Loading,
    MainMenu,
    InGame,
}

#[cfg(test)]
mod debugger_registration_tests {
    use super::*;
    use godot_bevy::plugins::debugger::{
        edit::read_component,
        value::{Kind, Scalar, ValueLimits},
    };

    #[test]
    fn debugger_registers_platformer_components_automatically() {
        let mut app = App::new();
        let entity = app
            .world_mut()
            .spawn((
                components::Speed::default(),
                components::JumpVelocity::default(),
                components::Gravity::default(),
                components::Player,
            ))
            .id();
        for (type_path, expected) in [
            (components::Speed::type_path(), 100.0),
            (components::JumpVelocity::type_path(), -400.0),
            (components::Gravity::type_path(), 980.0),
        ] {
            let value =
                read_component(app.world(), entity, type_path, &ValueLimits::default()).unwrap();
            assert!(
                matches!(value.kind, Kind::TupleStruct(fields) if fields[0].kind == Kind::Scalar(Scalar::Float(expected)))
            );
        }
        let player = read_component(
            app.world(),
            entity,
            components::Player::type_path(),
            &ValueLimits::default(),
        )
        .unwrap();
        assert!(!matches!(player.kind, Kind::Unsupported(_)));
    }
}
