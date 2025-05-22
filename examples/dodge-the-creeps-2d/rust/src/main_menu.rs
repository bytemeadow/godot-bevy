use bevy::{
    app::{App, Plugin, Update},
    ecs::{event::EventReader, schedule::IntoScheduleConfigs, system::ResMut, resource::Resource},
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

#[derive(NodeTreeView)]
pub struct MenuUi {
    #[node("/root/Main/HUD/Message")]
    pub message_label: GodotNodeHandle,

    #[node("/root/Main/HUD/StartButton")]
    pub start_button: GodotNodeHandle,
}

fn connect_start_button(mut scene_tree: SceneTreeRef, mut connected: ResMut<StartButtonConnected>) {
    if !connected.0 {
        let mut menu_ui = MenuUi::from_node(scene_tree.get().get_root().unwrap());
        connect_godot_signal(&mut menu_ui.start_button, "pressed", &mut scene_tree);
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

fn hide_play_button(mut scene_tree: SceneTreeRef) {
    let mut menu_ui = MenuUi::from_node(scene_tree.get().get_root().unwrap());
    menu_ui.start_button.get::<Button>().set_visible(false);
}

fn show_play_button(mut scene_tree: SceneTreeRef) {
    let mut menu_ui = MenuUi::from_node(scene_tree.get().get_root().unwrap());
    menu_ui.start_button.get::<Button>().set_visible(true);
}
