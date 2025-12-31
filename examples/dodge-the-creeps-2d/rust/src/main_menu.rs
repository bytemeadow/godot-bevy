use bevy::prelude::Message;
use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        message::{MessageReader, MessageWriter},
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Query, Res, ResMut},
    },
    state::{
        condition::in_state,
        state::{NextState, OnEnter, OnExit},
    },
};
use godot_bevy::{
    interop::{GodotNodeHandle, GodotNodeId},
    prelude::{
        GodotTypedSignalsPlugin, NodeEntityIndex, NodeTreeView, SceneTreeRef, TypedGodotSignals,
        main_thread_system,
    },
};

use crate::{
    GameState,
    commands::{UICommand, UIElement, UIHandles},
};

#[derive(Resource, Default)]
pub struct MenuAssets {
    pub message_label: Option<GodotNodeId>,
    pub start_button: Option<GodotNodeId>,
    pub score_label: Option<GodotNodeId>,
}
pub struct MainMenuPlugin;
impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MenuAssets>()
            // enable typed signal routing for menu
            .add_plugins(GodotTypedSignalsPlugin::<StartGameRequested>::default())
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
    let menu_ui = MenuUi::from_node(scene_tree.get().get_root().unwrap()).unwrap();

    menu_assets.message_label = Some(menu_ui.message_label.id());
    menu_assets.start_button = Some(menu_ui.start_button.id());
    menu_assets.score_label = Some(menu_ui.score_label.id());

    // Initialize UI handles for command system
    ui_handles.start_button = Some(menu_ui.start_button.id());
    ui_handles.score_label = Some(menu_ui.score_label.id());
    ui_handles.message_label = Some(menu_ui.message_label.id());
}

#[derive(Message, Debug, Clone)]
struct StartGameRequested;

#[main_thread_system]
fn connect_start_button(
    menu_assets: Res<MenuAssets>,
    node_index: Res<NodeEntityIndex>,
    mut nodes: Query<&mut GodotNodeHandle>,
    typed: TypedGodotSignals<StartGameRequested>,
) {
    if let Some(node_id) = menu_assets.start_button
        && let Some(entity) = node_index.get(node_id.instance_id())
        && let Ok(mut handle) = nodes.get_mut(entity)
    {
        typed.connect_map(
            &mut handle,
            "pressed",
            None,
            |_args, _node_id, _ent| Some(StartGameRequested),
        );
    }
}

fn listen_for_start_button(
    mut events: MessageReader<StartGameRequested>,
    mut app_state: ResMut<NextState<GameState>>,
) {
    for _ in events.read() {
        app_state.set(GameState::Countdown);
    }
}

fn hide_play_button(mut ui_commands: MessageWriter<UICommand>) {
    ui_commands.write(UICommand::SetVisible {
        target: UIElement::StartButton,
        visible: false,
    });
}

fn show_play_button(mut ui_commands: MessageWriter<UICommand>) {
    ui_commands.write(UICommand::SetVisible {
        target: UIElement::StartButton,
        visible: true,
    });
}
