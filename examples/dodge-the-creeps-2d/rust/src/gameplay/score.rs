use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        change_detection::DetectChanges,
        name::Name,
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Query, Res, ResMut},
    },
    state::{condition::in_state, state::OnEnter},
    time::{Time, Timer, TimerMode},
};
use godot::classes::Label;
use godot_bevy::{bridge::GodotNodeHandle, prelude::FindEntityByNameExt};

use crate::{GameState, Score};

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

fn update_score_counter(score: Res<Score>, mut entities: Query<(&Name, &mut GodotNodeHandle)>) {
    if score.is_changed() {
        let mut score_counter_label = entities
            .iter_mut()
            .find_entity_by_name("ScoreLabel")
            .unwrap();

        score_counter_label
            .get::<Label>()
            .set_text(&score.0.to_string());
    }
}

fn give_score(time: Res<Time>, mut timer: ResMut<ScoreTimer>, mut score: ResMut<Score>) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        score.0 += 1;
    }
}
