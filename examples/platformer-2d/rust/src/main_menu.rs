use crate::{
    GameState,
    level_manager::{LevelId, LoadLevelMessage},
};
use bevy::{
    app::prelude::*,
    ecs::{
        event::Event,
        observer::On,
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Commands, Res, ResMut},
    },
    log::{debug, info},
    state::{
        condition::in_state,
        state::{NextState, OnEnter},
    },
};
use godot::classes::{Button, DisplayServer, display_server::WindowMode};
use godot_bevy::interop::signal_names::BaseButtonSignals;
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
            // Enable signal routing for our menu events
            .add_plugins(GodotSignalsPlugin::<StartGameRequested>::default())
            .add_plugins(GodotSignalsPlugin::<ToggleFullscreenRequested>::default())
            .add_plugins(GodotSignalsPlugin::<QuitRequested>::default())
            .add_systems(OnEnter(GameState::MainMenu), reset_menu_assets)
            .add_systems(
                Update,
                (
                    init_menu_assets.run_if(menu_not_initialized),
                    connect_buttons.run_if(menu_initialized_but_signals_not_connected),
                )
                    .run_if(in_state(GameState::MainMenu)),
            )
            // Use observers for button press handling
            .add_observer(on_start_game)
            .add_observer(on_toggle_fullscreen)
            .add_observer(on_quit);
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

fn init_menu_assets(mut menu_assets: ResMut<MenuAssets>, mut scene_tree: SceneTreeRef) {
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

// Typed events for menu actions
#[derive(Event, Debug, Clone)]
struct StartGameRequested;

#[derive(Event, Debug, Clone)]
struct ToggleFullscreenRequested;

#[derive(Event, Debug, Clone)]
struct QuitRequested {
    source: GodotNodeHandle,
}

fn connect_buttons(
    mut menu_assets: ResMut<MenuAssets>,
    // Typed bridges for precise events
    signals_start: GodotSignals<StartGameRequested>,
    signals_fullscreen: GodotSignals<ToggleFullscreenRequested>,
    signals_quit: GodotSignals<QuitRequested>,
) {
    // Check if all buttons are available first
    if menu_assets.start_button.is_some()
        && menu_assets.fullscreen_button.is_some()
        && menu_assets.quit_button.is_some()
        && !menu_assets.signals_connected
    {
        if let Some(start_handle) = menu_assets.start_button {
            signals_start.connect(
                start_handle,
                BaseButtonSignals::PRESSED,
                None,
                |_args, _node_handle, _ent| Some(StartGameRequested),
            );
        }

        if let Some(fullscreen_handle) = menu_assets.fullscreen_button {
            signals_fullscreen.connect(
                fullscreen_handle,
                BaseButtonSignals::PRESSED,
                None,
                |_args, _node_handle, _ent| Some(ToggleFullscreenRequested),
            );
        }

        if let Some(quit_handle) = menu_assets.quit_button {
            signals_quit.connect(
                quit_handle,
                BaseButtonSignals::PRESSED,
                None,
                |_args, node_handle, _ent| {
                    Some(QuitRequested {
                        source: node_handle,
                    })
                },
            );
        }

        menu_assets.signals_connected = true;
        info!("MainMenu: Connected button signals");
    }
}

fn on_start_game(
    _trigger: On<StartGameRequested>,
    state: Res<bevy::state::state::State<GameState>>,
    mut app_state: ResMut<NextState<GameState>>,
    mut commands: Commands,
) {
    // Only respond when in MainMenu state
    if *state.get() != GameState::MainMenu {
        return;
    }
    println!("Start button pressed (typed)");
    app_state.set(GameState::InGame);
    commands.trigger(LoadLevelMessage {
        level_id: LevelId::Level1,
    });
}

fn on_toggle_fullscreen(
    _trigger: On<ToggleFullscreenRequested>,
    state: Res<bevy::state::state::State<GameState>>,
    mut godot: GodotAccess,
) {
    // Only respond when in MainMenu state
    if *state.get() != GameState::MainMenu {
        return;
    }
    println!("Fullscreen button pressed (typed)");
    let mut display_server = godot.singleton::<DisplayServer>();
    let window_mode = display_server.window_get_mode();
    if window_mode == WindowMode::FULLSCREEN {
        display_server.window_set_mode(WindowMode::WINDOWED);
    } else if window_mode == WindowMode::WINDOWED {
        display_server.window_set_mode(WindowMode::FULLSCREEN);
    }
}

fn on_quit(
    trigger: On<QuitRequested>,
    state: Res<bevy::state::state::State<GameState>>,
    mut godot: GodotAccess,
) {
    // Only respond when in MainMenu state
    if *state.get() != GameState::MainMenu {
        return;
    }
    println!("Quit button pressed (typed)");
    if let Some(button) = godot.try_get::<Button>(trigger.event().source)
        && let Some(mut tree) = button.get_tree()
    {
        tree.quit();
    }
}
