use bevy::{
    app::prelude::*,
    ecs::{
        event::{EventReader, EventWriter},
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Res, ResMut},
    },
    state::{
        condition::in_state,
        state::{NextState, OnEnter, OnExit},
    },
};
use godot::classes::{display_server::WindowMode, Button, DisplayServer};
use godot_bevy::prelude::*;

use crate::{
    level_manager::{LevelId, LoadLevelEvent},
    GameState,
};

#[derive(Resource, Default)]
pub struct MenuAssets {
    pub start_button: Option<GodotNodeHandle>,
    pub fullscreen_button: Option<GodotNodeHandle>,
    pub quit_button: Option<GodotNodeHandle>,
}

pub struct MainMenuPlugin;
impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuAssets>()
            .add_systems(
                OnEnter(GameState::MainMenu),
                (init_menu_assets, connect_buttons.after(init_menu_assets)),
            )
            .add_systems(
                Update,
                listen_for_button_press.run_if(in_state(GameState::MainMenu)),
            );
    }
}

#[derive(NodeTreeView)]
pub struct MenuUi {
    #[node("/root/MainMenu/Options/StartButton")]
    pub start_button: GodotNodeHandle,

    #[node("/root/MainMenu/Options/FullscreenButton")]
    pub fullscreen_button: GodotNodeHandle,

    #[node("/root/MainMenu/Options/QuitButton")]
    pub quit_button: GodotNodeHandle,
}

fn init_menu_assets(mut menu_assets: ResMut<MenuAssets>, mut scene_tree: SceneTreeRef) {
    let menu_ui = MenuUi::from_node(scene_tree.get().get_root().unwrap());

    menu_assets.start_button = Some(menu_ui.start_button.clone());
    menu_assets.fullscreen_button = Some(menu_ui.fullscreen_button.clone());
    menu_assets.quit_button = Some(menu_ui.quit_button.clone());
}

fn connect_buttons(mut menu_assets: ResMut<MenuAssets>, mut scene_tree: SceneTreeRef) {
    connect_godot_signal(
        menu_assets.start_button.as_mut().unwrap(),
        "pressed",
        &mut scene_tree,
    );
    connect_godot_signal(
        menu_assets.fullscreen_button.as_mut().unwrap(),
        "pressed",
        &mut scene_tree,
    );
    connect_godot_signal(
        menu_assets.quit_button.as_mut().unwrap(),
        "pressed",
        &mut scene_tree,
    );
}

fn listen_for_button_press(
    menu_assets: Res<MenuAssets>,
    mut events: EventReader<GodotSignal>,
    mut app_state: ResMut<NextState<GameState>>,
    mut level_load_events: EventWriter<LoadLevelEvent>,
) {
    for evt in events.read() {
        if evt.name == "pressed" && &evt.target == menu_assets.start_button.as_ref().unwrap() {
            println!("Start button pressed");

            // Change to InGame state
            app_state.set(GameState::InGame);

            // Send level load event to start with tutorial
            level_load_events.write(LoadLevelEvent {
                level_id: LevelId::Level1,
            });
        }
        if evt.name == "pressed" && &evt.target == menu_assets.fullscreen_button.as_ref().unwrap() {
            println!("Fullscreen button pressed");
            if DisplayServer::singleton().window_get_mode() == WindowMode::FULLSCREEN {
                DisplayServer::singleton().window_set_mode(WindowMode::WINDOWED);
            } else if DisplayServer::singleton().window_get_mode() == WindowMode::WINDOWED {
                DisplayServer::singleton().window_set_mode(WindowMode::FULLSCREEN);
            }
        }
        if evt.name == "pressed" && &evt.target == menu_assets.quit_button.as_ref().unwrap() {
            println!("Quit button pressed");
            if let Some(mut tree) = evt.target.clone().get::<Button>().get_tree() {
                tree.quit();
            }
        }
    }
}
