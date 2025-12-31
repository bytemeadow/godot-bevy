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
        system::{Query, Res, ResMut},
    },
    log::{debug, info},
    state::{
        condition::in_state,
        state::{NextState, OnEnter},
    },
};
use godot::classes::{Button, DisplayServer, display_server::WindowMode};
use godot::obj::Singleton;
use godot_bevy::prelude::*;

#[derive(Resource, Default)]
pub struct MenuAssets {
    pub start_button: Option<GodotNodeId>,
    pub fullscreen_button: Option<GodotNodeId>,
    pub quit_button: Option<GodotNodeId>,
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

#[main_thread_system]
fn init_menu_assets(mut menu_assets: ResMut<MenuAssets>, mut scene_tree: SceneTreeRef) {
    // Try to find menu nodes, but handle failure gracefully
    if let Some(root) = scene_tree.get().get_root() {
        // Try to create MenuUi - this might fail if nodes aren't ready yet
        match MenuUi::from_node(root) {
            Ok(menu_ui) => {
                info!("MainMenu: Successfully found menu nodes");
                menu_assets.start_button = Some(menu_ui.start_button.id());
                menu_assets.fullscreen_button = Some(menu_ui.fullscreen_button.id());
                menu_assets.quit_button = Some(menu_ui.quit_button.id());
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
    source: GodotNodeId,
}

#[main_thread_system]
fn connect_buttons(
    mut menu_assets: ResMut<MenuAssets>,
    node_index: Res<NodeEntityIndex>,
    mut nodes: Query<&mut GodotNodeHandle>,
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
        if let Some(start_id) = menu_assets.start_button
            && let Some(entity) = node_index.get(start_id.instance_id())
            && let Ok(mut handle) = nodes.get_mut(entity)
        {
            typed_start.connect_map(&mut handle, "pressed", None, |_args, _node_id, _ent| {
                Some(StartGameRequested)
            });
        }

        if let Some(fullscreen_id) = menu_assets.fullscreen_button
            && let Some(entity) = node_index.get(fullscreen_id.instance_id())
            && let Ok(mut handle) = nodes.get_mut(entity)
        {
            typed_fullscreen.connect_map(
                &mut handle,
                "pressed",
                None,
                |_args, _node_id, _ent| Some(ToggleFullscreenRequested),
            );
        }

        if let Some(quit_id) = menu_assets.quit_button
            && let Some(entity) = node_index.get(quit_id.instance_id())
            && let Ok(mut handle) = nodes.get_mut(entity)
        {
            typed_quit.connect_map(&mut handle, "pressed", None, |_args, node_id, _ent| {
                Some(QuitRequested { source: node_id })
            });
        }

        menu_assets.signals_connected = true;
        info!("MainMenu: Connected button signals");
    }
}

#[main_thread_system]
fn listen_for_button_press(
    _menu_assets: Res<MenuAssets>,
    node_index: Res<NodeEntityIndex>,
    mut nodes: Query<&mut GodotNodeHandle>,
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
        if DisplayServer::singleton().window_get_mode() == WindowMode::FULLSCREEN {
            DisplayServer::singleton().window_set_mode(WindowMode::WINDOWED);
        } else if DisplayServer::singleton().window_get_mode() == WindowMode::WINDOWED {
            DisplayServer::singleton().window_set_mode(WindowMode::FULLSCREEN);
        }
    }

    for ev in quit_ev.read() {
        println!("Quit button pressed (typed)");
        if let Some(entity) = node_index.get(ev.source.instance_id())
            && let Ok(mut handle) = nodes.get_mut(entity)
            && let Some(button) = handle.try_get::<Button>()
            && let Some(mut tree) = button.get_tree()
        {
            tree.quit();
        }
    }
}
