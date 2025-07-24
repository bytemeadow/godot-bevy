use bevy::{
    app::{Plugin, Update},
    ecs::{
        event::EventWriter,
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

use crate::{GameState, commands::UICommand};

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

fn setup_gameover(mut commands: Commands, mut ui_commands: EventWriter<UICommand>) {
    commands.insert_resource(GameoverTimer(Timer::from_seconds(2.0, TimerMode::Once)));

    ui_commands.write(UICommand::ShowMessage {
        text: "Game Over".to_string(),
    });
}

fn update_gameover_timer(
    mut timer: ResMut<GameoverTimer>,
    time: Res<Time>,
    mut next_state: ResMut<NextState<GameState>>,
    mut ui_commands: EventWriter<UICommand>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    next_state.set(GameState::MainMenu);

    ui_commands.write(UICommand::ShowMessage {
        text: "Dodge the Creeps".to_string(),
    });
}
