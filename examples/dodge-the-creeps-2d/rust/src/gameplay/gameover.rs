use bevy::{
    app::{Plugin, Update},
    ecs::{
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Commands, Res, ResMut},
    },
    state::{
        condition::in_state,
        state::{NextState, OnEnter},
    },
    time::{Time, Timer, TimerMode},
};
use godot::classes::Label;
use godot_bevy::prelude::{NodeTreeView, SceneTreeRef};

use crate::{main_menu::MenuUi, GameState};

pub struct GameoverPlugin;
impl Plugin for GameoverPlugin {
    fn build(&self, app: &mut bevy::app::App) {
        app.add_systems(OnEnter(GameState::GameOver), setup_gameover)
            .add_systems(
                Update,
                update_gameover_timer.run_if(in_state(GameState::GameOver)),
            );
    }
}

#[derive(Resource)]
pub struct GameoverTimer(Timer);

fn setup_gameover(mut commands: Commands, mut scene_tree: SceneTreeRef) {
    commands.insert_resource(GameoverTimer(Timer::from_seconds(2.0, TimerMode::Once)));

    let mut menu_ui = MenuUi::from_node(scene_tree.get().get_root().unwrap());
    menu_ui.message_label.get::<Label>().set_text("Game Over");
}

fn update_gameover_timer(
    mut timer: ResMut<GameoverTimer>,
    time: Res<Time>,
    mut next_state: ResMut<NextState<GameState>>,
    mut scene_tree: SceneTreeRef,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    next_state.set(GameState::MainMenu);

    let mut menu_ui = MenuUi::from_node(scene_tree.get().get_root().unwrap());
    menu_ui.message_label.get::<Label>().set_text("Dodge the Creeps");
}
