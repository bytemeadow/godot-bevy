use crate::{
    GameState,
    level_manager::{LevelId, LoadLevelMessage},
};
use bevy::prelude::Message;
use bevy::{
    app::prelude::*,
    ecs::{
        message::{MessageReader, MessageWriter},
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
use godot::classes::{Button, DisplayServer, display_server::WindowMode};
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
            // Enable typed signal routing for our menu events
            .add_plugins(GodotTypedSignalsPlugin::<StartGameRequested>::default())
            .add_plugins(GodotTypedSignalsPlugin::<ToggleFullscreenRequested>::default())
            .add_plugins(GodotTypedSignalsPlugin::<QuitRequested>::default())
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

fn init_menu_assets(
    mut menu_assets: ResMut<MenuAssets>,
    mut scene_tree: SceneTreeRef,
) {
    // Try to find menu nodes, but handle failure gracefully
    if let Some(root) = scene_tree.get().get_root() {
        // Try to create MenuUi - this might fail if nodes aren't ready yet
        match MenuUi::from_node(root) {
            Ok(menu_ui) => {
                info!("MainMenu: Successfully found menu nodes");
                menu_assets.start_button = Some(menu_ui.start_button);
                menu_assets.fullscreen_button = Some(menu_ui.fullscreen_button);
                menu_assets.quit_button = Some(menu_ui.quit_button);
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

// Typed events for menu actions
#[derive(Message, Debug, Clone)]
struct StartGameRequested;

#[derive(Message, Debug, Clone)]
struct ToggleFullscreenRequested;

#[derive(Message, Debug, Clone)]
struct QuitRequested {
    source: GodotNodeHandle,
}

fn connect_buttons(
    mut menu_assets: ResMut<MenuAssets>,
    // Typed bridges for precise events
    typed_start: TypedGodotSignals<StartGameRequested>,
    typed_fullscreen: TypedGodotSignals<ToggleFullscreenRequested>,
    typed_quit: TypedGodotSignals<QuitRequested>,
) {
    // Check if all buttons are available first
    if menu_assets.start_button.is_some()
        && menu_assets.fullscreen_button.is_some()
        && menu_assets.quit_button.is_some()
        && !menu_assets.signals_connected
    {
        if let Some(start_handle) = menu_assets.start_button {
            typed_start.connect_map(
                start_handle,
                "pressed",
                None,
                |_args, _node_handle, _ent| Some(StartGameRequested),
            );
        }

        if let Some(fullscreen_handle) = menu_assets.fullscreen_button {
            typed_fullscreen.connect_map(
                fullscreen_handle,
                "pressed",
                None,
                |_args, _node_handle, _ent| Some(ToggleFullscreenRequested),
            );
        }

        if let Some(quit_handle) = menu_assets.quit_button {
            typed_quit.connect_map(
                quit_handle,
                "pressed",
                None,
                |_args, node_handle, _ent| Some(QuitRequested { source: node_handle }),
            );
        }

        menu_assets.signals_connected = true;
        info!("MainMenu: Connected button signals");
    }
}

fn listen_for_button_press(
    mut godot: GodotAccess,
    mut start_ev: MessageReader<StartGameRequested>,
    mut toggle_ev: MessageReader<ToggleFullscreenRequested>,
    mut quit_ev: MessageReader<QuitRequested>,
    mut app_state: ResMut<NextState<GameState>>,
    mut level_load_events: MessageWriter<LoadLevelMessage>,
) {
    for _ in start_ev.read() {
        println!("Start button pressed (typed)");
        app_state.set(GameState::InGame);
        level_load_events.write(LoadLevelMessage {
            level_id: LevelId::Level1,
        });
    }

    for _ in toggle_ev.read() {
        println!("Fullscreen button pressed (typed)");
        let mut display_server = godot.singleton::<DisplayServer>();
        let window_mode = display_server.window_get_mode();
        if window_mode == WindowMode::FULLSCREEN {
            display_server.window_set_mode(WindowMode::WINDOWED);
        } else if window_mode == WindowMode::WINDOWED {
            display_server.window_set_mode(WindowMode::FULLSCREEN);
        }
    }

    for ev in quit_ev.read() {
        println!("Quit button pressed (typed)");
        if let Some(button) = godot.try_get::<Button>(ev.source)
            && let Some(mut tree) = button.get_tree()
        {
            tree.quit();
        }
    }
}
