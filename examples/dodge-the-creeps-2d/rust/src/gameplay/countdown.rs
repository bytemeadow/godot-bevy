use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        entity::Entity,
        event::EventWriter,
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
use godot_bevy::prelude::Groups;

use crate::{
    GameState,
    commands::{NodeCommand, UICommand},
};

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

fn setup_countdown(mut commands: Commands, mut ui_commands: EventWriter<UICommand>) {
    commands.insert_resource(CountdownTimer(Timer::from_seconds(1.0, TimerMode::Once)));

    ui_commands.write(UICommand::ShowMessage {
        text: "Get Ready".to_string(),
    });
}

fn update_countdown(
    mut timer: ResMut<CountdownTimer>,
    time: Res<Time>,
    mut next_state: ResMut<NextState<GameState>>,
    mut ui_commands: EventWriter<UICommand>,
) {
    timer.0.tick(time.delta());
    if timer.0.just_finished() {
        next_state.set(GameState::InGame);

        ui_commands.write(UICommand::ShowMessage {
            text: "".to_string(),
        });
    }
}

fn kill_all_mobs(entities: Query<(Entity, &Groups)>, mut node_commands: EventWriter<NodeCommand>) {
    for (entity, group) in entities.iter() {
        if group.is("mobs") {
            node_commands.write(NodeCommand::Destroy { entity });
        }
    }
}
