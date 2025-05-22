use bevy::{app::{App, Plugin}, ecs::system::{Commands, Query}, state::state::OnEnter};
use godot::global::godot_print;
use godot_bevy::{bridge::GodotNodeHandle, prelude::{Groups, SceneTreeRef}};

use crate::GameState;


pub struct CountdownPlugin;
impl Plugin for CountdownPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Countdown), (setup_countdown, kill_all_mobs));
    }
}

fn setup_countdown(
    mut commands: Commands,
    mut scene_tree: SceneTreeRef
) {
    godot_print!("Setting up countdown");
}

fn kill_all_mobs(mut entities: Query<(&Groups, &mut GodotNodeHandle)>) {
    godot_print!("Killing all mobs");
}
