use bevy::{
    app::{App, Plugin},
    ecs::{
        event::Event,
        message::MessageWriter,
        observer::On,
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Res, ResMut},
    },
    state::state::{NextState, OnEnter, OnExit},
};
use godot::classes::CheckButton;
use godot_bevy::interop::signal_names::BaseButtonSignals;
use godot_bevy::prelude::*;

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
            .add_plugins(GodotSignalsPlugin::<StartGameRequested>::default())
            .add_godot_event::<ShowScoreChanged>("show_score_changed", |payload| {
                Some(ShowScoreChanged(payload.try_to::<bool>().ok()?))
            })
            .add_systems(
                OnExit(GameState::Loading),
                (
                    init_menu_assets,
                    connect_start_button.after(init_menu_assets),
                ),
            )
            .add_observer(on_start_game_requested)
            .add_observer(on_show_score_changed)
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

    #[node("/root/Main/HUD/ShowScoreButton")]
    pub show_score_button: GodotNodeHandle,
}

fn init_menu_assets(
    mut menu_assets: ResMut<MenuAssets>,
    mut ui_handles: ResMut<UIHandles>,
    mut scene_tree: SceneTreeRef,
    mut godot: GodotAccess,
) {
    let menu_ui = MenuUi::from_node(scene_tree.get().get_root().unwrap()).unwrap();

    menu_assets.message_label = Some(menu_ui.message_label);
    menu_assets.start_button = Some(menu_ui.start_button);
    menu_assets.score_label = Some(menu_ui.score_label);

    ui_handles.start_button = Some(menu_ui.start_button);
    ui_handles.score_label = Some(menu_ui.score_label);
    ui_handles.message_label = Some(menu_ui.message_label);
    ui_handles.show_score_button = Some(menu_ui.show_score_button);
    godot
        .get::<CheckButton>(menu_ui.show_score_button)
        .set_disabled(false);
}

#[derive(Event, Clone)]
struct ShowScoreChanged(bool);

fn on_show_score_changed(event: On<ShowScoreChanged>, mut ui_commands: MessageWriter<UICommand>) {
    ui_commands.write(UICommand::SetVisible {
        target: UIElement::ScoreLabel,
        visible: event.event().0,
    });
}

#[derive(Event, Debug, Clone)]
struct StartGameRequested;

fn connect_start_button(menu_assets: Res<MenuAssets>, signals: GodotSignals<StartGameRequested>) {
    if let Some(handle) = menu_assets.start_button {
        signals.connect(
            handle,
            BaseButtonSignals::PRESSED,
            None,
            |_args, _node_handle, _ent| Some(StartGameRequested),
        );
    }
}

fn on_start_game_requested(
    _trigger: On<StartGameRequested>,
    mut app_state: ResMut<NextState<GameState>>,
    state: Res<bevy::state::state::State<GameState>>,
) {
    if *state.get() == GameState::MainMenu {
        app_state.set(GameState::Countdown);
    }
}

fn hide_play_button(mut ui_commands: MessageWriter<UICommand>) {
    ui_commands.write(UICommand::SetVisible {
        target: UIElement::StartButton,
        visible: false,
    });
    ui_commands.write(UICommand::SetVisible {
        target: UIElement::ShowScoreButton,
        visible: false,
    });
}

fn show_play_button(mut ui_commands: MessageWriter<UICommand>) {
    ui_commands.write(UICommand::SetVisible {
        target: UIElement::StartButton,
        visible: true,
    });
    ui_commands.write(UICommand::SetVisible {
        target: UIElement::ShowScoreButton,
        visible: true,
    });
}
