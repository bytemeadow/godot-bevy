use bevy::{
    app::{App, Plugin, Update},
    ecs::{event::EventReader, resource::Resource, schedule::IntoScheduleConfigs, system::ResMut},
    state::{
        condition::in_state,
        state::{NextState, OnEnter, OnExit},
    },
};
use godot::classes::Button;
use godot_bevy::{
    bridge::GodotNodeHandle,
    prelude::{connect_godot_signal, GodotSignal, NodeTreeView, SceneTreeRef},
};

use crate::GameState;

pub struct MainMenuPlugin;
impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<StartButtonConnected>()
            .init_resource::<MenuAssets>()
            .add_systems(OnEnter(GameState::MainMenu), connect_start_button)
            .add_systems(
                Update,
                listen_for_start_button.run_if(in_state(GameState::MainMenu)),
            )
            .add_systems(OnExit(GameState::MainMenu), hide_play_button)
            .add_systems(OnEnter(GameState::MainMenu), show_play_button);
    }
}

#[derive(Resource, Default)]
struct StartButtonConnected(bool);

#[derive(Resource, Default)]
pub struct MenuAssets {
    pub message_label: Option<GodotNodeHandle>,
    pub start_button: Option<GodotNodeHandle>,
    pub score_label: Option<GodotNodeHandle>,
}

#[derive(NodeTreeView)]
pub struct MenuUi {
    #[node("/root/Main/HUD/Message")]
    pub message_label: GodotNodeHandle,

    #[node("/root/Main/HUD/StartButton")]
    pub start_button: GodotNodeHandle,

    #[node("/root/Main/HUD/ScoreLabel")]
    pub score_label: GodotNodeHandle,
}

fn connect_start_button(
    mut scene_tree: SceneTreeRef,
    mut connected: ResMut<StartButtonConnected>,
    mut menu_assets: ResMut<MenuAssets>,
) {
    if !connected.0 {
        let menu_ui = MenuUi::from_node(scene_tree.get().get_root().unwrap());
        connect_godot_signal(
            &mut menu_ui.start_button.clone(),
            "pressed",
            &mut scene_tree,
        );

        // Store the UI elements in the resource
        menu_assets.message_label = Some(menu_ui.message_label.clone());
        menu_assets.start_button = Some(menu_ui.start_button.clone());
        menu_assets.score_label = Some(menu_ui.score_label.clone());

        connected.0 = true;
    }
}

fn listen_for_start_button(
    mut events: EventReader<GodotSignal>,
    mut app_state: ResMut<NextState<GameState>>,
) {
    for evt in events.read() {
        if evt.name == "pressed" {
            app_state.set(GameState::Countdown);
        }
    }
}

fn hide_play_button(menu_assets: ResMut<MenuAssets>) {
    if let Some(mut start_button) = menu_assets.start_button.clone() {
        start_button.get::<Button>().set_visible(false);
    }
}

fn show_play_button(menu_assets: ResMut<MenuAssets>) {
    if let Some(mut start_button) = menu_assets.start_button.clone() {
        start_button.get::<Button>().set_visible(true);
    }
}
