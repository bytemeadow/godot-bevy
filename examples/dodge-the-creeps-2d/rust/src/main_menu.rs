use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        event::{EventReader, EventWriter},
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::ResMut,
    },
    state::{
        condition::in_state,
        state::{NextState, OnEnter, OnExit},
    },
};
use godot_bevy::{
    interop::GodotNodeHandle,
    prelude::{GodotSignal, GodotSignals, NodeTreeView, SceneTreeRef, main_thread_system},
};

use crate::{
    GameState,
    commands::{UICommand, UIElement, UIHandles},
};

#[derive(Resource, Default)]
pub struct MenuAssets {
    pub message_label: Option<GodotNodeHandle>,
    pub start_button: Option<GodotNodeHandle>,
    pub score_label: Option<GodotNodeHandle>,
}
pub struct MainMenuPlugin;
impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuAssets>()
            .add_systems(
                OnExit(GameState::Loading),
                (
                    init_menu_assets,
                    connect_start_button.after(init_menu_assets),
                ),
            )
            .add_systems(
                Update,
                listen_for_start_button.run_if(in_state(GameState::MainMenu)),
            )
            .add_systems(OnExit(GameState::MainMenu), hide_play_button)
            .add_systems(OnEnter(GameState::MainMenu), show_play_button);
    }
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

#[main_thread_system]
fn init_menu_assets(
    mut menu_assets: ResMut<MenuAssets>,
    mut ui_handles: ResMut<UIHandles>,
    mut scene_tree: SceneTreeRef,
) {
    let menu_ui = MenuUi::from_node(scene_tree.get().get_root().unwrap());

    menu_assets.message_label = Some(menu_ui.message_label.clone());
    menu_assets.start_button = Some(menu_ui.start_button.clone());
    menu_assets.score_label = Some(menu_ui.score_label.clone());

    // Initialize UI handles for command system
    ui_handles.start_button = Some(menu_ui.start_button.clone());
    ui_handles.score_label = Some(menu_ui.score_label.clone());
    ui_handles.message_label = Some(menu_ui.message_label.clone());
}

fn connect_start_button(mut menu_assets: ResMut<MenuAssets>, signals: GodotSignals) {
    signals.connect(menu_assets.start_button.as_mut().unwrap(), "pressed");
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

fn hide_play_button(mut ui_commands: EventWriter<UICommand>) {
    ui_commands.write(UICommand::SetVisible {
        target: UIElement::StartButton,
        visible: false,
    });
}

fn show_play_button(mut ui_commands: EventWriter<UICommand>) {
    ui_commands.write(UICommand::SetVisible {
        target: UIElement::StartButton,
        visible: true,
    });
}
