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

use crate::{main_menu::MenuAssets, GameState};

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

fn setup_gameover(mut commands: Commands, menu_assets: Res<MenuAssets>) {
    commands.insert_resource(GameoverTimer(Timer::from_seconds(2.0, TimerMode::Once)));

    if let Some(mut message_label) = menu_assets.message_label.clone() {
        message_label.get::<Label>().set_text("Game Over");
    }
}

fn update_gameover_timer(
    mut timer: ResMut<GameoverTimer>,
    time: Res<Time>,
    mut next_state: ResMut<NextState<GameState>>,
    menu_assets: Res<MenuAssets>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    next_state.set(GameState::MainMenu);

    if let Some(mut message_label) = menu_assets.message_label.clone() {
        message_label.get::<Label>().set_text("Dodge the Creeps");
    }
}
