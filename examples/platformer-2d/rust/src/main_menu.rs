use crate::{
    GameState,
    level_manager::{LevelId, LoadLevelEvent},
};
use bevy::{
    app::prelude::*,
    ecs::{
        event::{EventReader, EventWriter},
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Res, ResMut},
    },
    log::{debug, info},
    state::{
        condition::in_state,
        state::{NextState, OnEnter},
    },
};
use godot::classes::{Button, DisplayServer, Node, display_server::WindowMode};
use godot_bevy::prelude::*;

#[derive(Resource, Default)]
pub struct MenuAssets {
    pub start_button: Option<GodotNodeHandle>,
    pub fullscreen_button: Option<GodotNodeHandle>,
    pub quit_button: Option<GodotNodeHandle>,
    pub initialized: bool,
    pub signals_connected: bool,
}

pub struct MainMenuPlugin;
impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuAssets>()
            .add_systems(OnEnter(GameState::MainMenu), reset_menu_assets)
            .add_systems(
                Update,
                (
                    init_menu_assets.run_if(menu_not_initialized),
                    connect_buttons.run_if(menu_initialized_but_signals_not_connected),
                    listen_for_button_press.run_if(menu_is_initialized),
                )
                    .run_if(in_state(GameState::MainMenu)),
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

fn reset_menu_assets(mut menu_assets: ResMut<MenuAssets>) {
    menu_assets.start_button = None;
    menu_assets.fullscreen_button = None;
    menu_assets.quit_button = None;
    menu_assets.initialized = false;
    menu_assets.signals_connected = false;
}

#[main_thread_system]
fn init_menu_assets(mut menu_assets: ResMut<MenuAssets>, mut scene_tree: SceneTreeRef) {
    // Try to find menu nodes, but handle failure gracefully
    if let Some(root) = scene_tree.get().get_root() {
        // Try to create MenuUi - this might fail if nodes aren't ready yet
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| MenuUi::from_node(root))) {
            Ok(menu_ui) => {
                info!("MainMenu: Successfully found menu nodes");
                menu_assets.start_button = Some(menu_ui.start_button.clone());
                menu_assets.fullscreen_button = Some(menu_ui.fullscreen_button.clone());
                menu_assets.quit_button = Some(menu_ui.quit_button.clone());
                menu_assets.initialized = true;
            }
            Err(_) => {
                debug!("MainMenu: Menu nodes not ready yet, will retry next frame");
            }
        }
    } else {
        debug!("MainMenu: Scene root not available yet");
    }
}

fn menu_not_initialized(menu_assets: Res<MenuAssets>) -> bool {
    !menu_assets.initialized
}

fn menu_initialized_but_signals_not_connected(menu_assets: Res<MenuAssets>) -> bool {
    menu_assets.initialized && !menu_assets.signals_connected
}

fn menu_is_initialized(menu_assets: Res<MenuAssets>) -> bool {
    menu_assets.initialized
}

fn connect_buttons(mut menu_assets: ResMut<MenuAssets>, signals: GodotSignals) {
    // Check if all buttons are available first
    if menu_assets.start_button.is_some()
        && menu_assets.fullscreen_button.is_some()
        && menu_assets.quit_button.is_some()
        && !menu_assets.signals_connected
    {
        // Get mutable references one at a time to avoid multiple borrows
        if let Some(start_btn) = menu_assets.start_button.as_mut() {
            signals.connect(start_btn, "pressed");
        }
        if let Some(fullscreen_btn) = menu_assets.fullscreen_button.as_mut() {
            signals.connect(fullscreen_btn, "pressed");
        }
        if let Some(quit_btn) = menu_assets.quit_button.as_mut() {
            signals.connect(quit_btn, "pressed");
        }

        menu_assets.signals_connected = true;
        info!("MainMenu: Connected button signals");
    }
}

#[main_thread_system]
fn listen_for_button_press(
    menu_assets: Res<MenuAssets>,
    mut events: EventReader<GodotSignal>,
    mut app_state: ResMut<NextState<GameState>>,
    mut level_load_events: EventWriter<LoadLevelEvent>,
) {
    for evt in events.read() {
        // Skip events for freed nodes - check if target node still exists
        if evt.target.clone().try_get::<Node>().is_none() {
            continue;
        }

        if evt.name == "pressed" {
            if let Some(start_button) = &menu_assets.start_button
                && &evt.target == start_button
            {
                println!("Start button pressed");
                app_state.set(GameState::InGame);
                level_load_events.write(LoadLevelEvent {
                    level_id: LevelId::Level1,
                });
                continue;
            }

            if let Some(fullscreen_button) = &menu_assets.fullscreen_button
                && &evt.target == fullscreen_button
            {
                println!("Fullscreen button pressed");
                if DisplayServer::singleton().window_get_mode() == WindowMode::FULLSCREEN {
                    DisplayServer::singleton().window_set_mode(WindowMode::WINDOWED);
                } else if DisplayServer::singleton().window_get_mode() == WindowMode::WINDOWED {
                    DisplayServer::singleton().window_set_mode(WindowMode::FULLSCREEN);
                }
                continue;
            }

            if let Some(quit_button) = &menu_assets.quit_button
                && &evt.target == quit_button
            {
                println!("Quit button pressed");
                if let Some(button) = evt.target.clone().try_get::<Button>()
                    && let Some(mut tree) = button.get_tree()
                {
                    tree.quit();
                }
            }
        }
    }
}
