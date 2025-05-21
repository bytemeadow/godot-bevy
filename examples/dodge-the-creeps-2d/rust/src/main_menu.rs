use bevy::{app::{App, Plugin, Update}, asset::Assets, ecs::{event::EventReader, resource::Resource, schedule::IntoScheduleConfigs, system::ResMut}, state::{condition::in_state, state::{NextState, OnEnter}}};
use godot::{classes::Node, global::godot_print, obj::Gd};
use godot_bevy::{bridge::{GodotNodeHandle, GodotResourceHandle}, prelude::{connect_godot_signal, GodotSignal, NodeTreeView, SceneTreeRef}};

use crate::GameState;

#[derive(Debug, Resource)]
pub struct MenuAssets {
    message_label: GodotResourceHandle,
    start_button: GodotResourceHandle,
}

pub struct MainMenuPlugin;
impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app
        .add_systems(OnEnter(GameState::MainMenu), connect_start_button)
        .add_systems(Update, listen_for_start_button.run_if(in_state(GameState::MainMenu)));
    }
}

#[derive(NodeTreeView)]
pub struct MenuUi {
    #[node("/root/Main/HUD/Message")]
    message_label: GodotNodeHandle,

    #[node("/root/Main/HUD/StartButton")]
    start_button: GodotNodeHandle,
}

fn connect_start_button(
    mut scene_tree: SceneTreeRef,
) {
    print_scene_tree(&mut scene_tree);
    let mut menu_ui = MenuUi::from_node(scene_tree.get().get_root().unwrap());
    connect_godot_signal(&mut menu_ui.start_button, "pressed", &mut scene_tree);
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

/// Prints the tree structure starting from the given node with proper indentation.
pub fn print_tree_structure(node: Gd<Node>, indent_level: usize) {
    let indent = "  ".repeat(indent_level);
    godot_print!("{}Node: {}", indent, node.get_name());

    for child in node.get_children().iter_shared() {
        print_tree_structure(child, indent_level + 1);
    }
}

/// Prints the entire scene tree structure starting from the root node.
pub fn print_scene_tree(scene_tree: &mut SceneTreeRef) {
    let root = scene_tree.get().get_root().unwrap();
    godot_print!("Scene Tree Structure:");
    print_tree_structure(root.upcast(), 0);
} 