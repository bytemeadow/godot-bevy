use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Commands, Query, Res, ResMut},
    },
    state::{
        condition::in_state,
        state::{NextState, OnEnter},
    },
    time::{Time, Timer, TimerMode},
};
use godot::classes::{Label, Node};
use godot_bevy::{bridge::GodotNodeHandle, prelude::Groups};

use crate::{main_menu::MenuAssets, GameState};

pub struct CountdownPlugin;
impl Plugin for CountdownPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(GameState::Countdown),
            (setup_countdown, kill_all_mobs),
        )
        .add_systems(
            Update,
            update_countdown.run_if(in_state(GameState::Countdown)),
        );
    }
}

#[derive(Resource)]
pub struct CountdownTimer(Timer);

fn setup_countdown(mut commands: Commands, menu_assets: Res<MenuAssets>) {
    commands.insert_resource(CountdownTimer(Timer::from_seconds(1.0, TimerMode::Once)));

    if let Some(mut message_label) = menu_assets.message_label.clone() {
        message_label.get::<Label>().set_text("Get Ready");
    }
}

fn update_countdown(
    mut timer: ResMut<CountdownTimer>,
    time: Res<Time>,
    mut next_state: ResMut<NextState<GameState>>,
    menu_assets: Res<MenuAssets>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        next_state.set(GameState::InGame);

        if let Some(mut message_label) = menu_assets.message_label.clone() {
            message_label.get::<Label>().set_text("");
        }
    }
}

fn kill_all_mobs(mut entities: Query<(&Groups, &mut GodotNodeHandle)>) {
    for (group, mut reference) in entities.iter_mut() {
        if group.is("mobs") {
            reference.get::<Node>().queue_free();
        }
    }
}
