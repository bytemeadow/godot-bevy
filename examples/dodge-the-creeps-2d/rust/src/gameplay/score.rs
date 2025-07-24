use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        change_detection::DetectChanges,
        event::EventWriter,
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Res, ResMut},
    },
    state::{condition::in_state, state::OnEnter},
    time::{Time, Timer, TimerMode},
};

use crate::{
    GameState, Score,
    commands::{UICommand, UIElement},
};

pub struct ScorePlugin;
impl Plugin for ScorePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::Countdown), reset_score)
            .add_systems(Update, update_score_counter)
            .add_systems(Update, give_score.run_if(in_state(GameState::InGame)))
            .insert_resource(ScoreTimer(Timer::from_seconds(1.0, TimerMode::Repeating)));
    }
}

#[derive(Resource)]
pub struct ScoreTimer(Timer);

fn reset_score(mut score: ResMut<Score>) {
    score.0 = 0;
}

fn update_score_counter(score: Res<Score>, mut ui_commands: EventWriter<UICommand>) {
    if score.is_changed() {
        ui_commands.write(UICommand::SetText {
            target: UIElement::ScoreLabel,
            text: score.0.to_string(),
        });
    }
}

fn give_score(time: Res<Time>, mut timer: ResMut<ScoreTimer>, mut score: ResMut<Score>) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        score.0 += 1;
    }
}
