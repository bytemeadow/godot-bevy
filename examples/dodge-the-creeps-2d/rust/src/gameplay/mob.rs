use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        component::Component, name::Name, resource::Resource, schedule::IntoScheduleConfigs, system::{Commands, Query, Res, ResMut}
    },
    state::condition::in_state,
    time::{Time, Timer, TimerMode},
};
use godot::{classes::{PathFollow2D, ResourceLoader}, builtin::Transform2D as GodotTransform2D};
use godot_bevy::{
    bridge::{GodotNodeHandle, GodotResourceHandle},
    prelude::{FindEntityByNameExt, GodotScene, Transform2D},
};
use std::f32::consts::PI;

use crate::GameState;

#[derive(Debug, Resource)]
pub struct MobAssets {
    mob_scn: GodotResourceHandle,
}

impl Default for MobAssets {
    fn default() -> Self {
        let mut resource_loader = ResourceLoader::singleton();
        let mob_scn = GodotResourceHandle::new(resource_loader.load("scenes/mob.tscn").unwrap());

        Self { mob_scn }
    }
}

pub struct MobPlugin;

impl Plugin for MobPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, spawn_mob.run_if(in_state(GameState::InGame)))
            .insert_resource(MobSpawnTimer(Timer::from_seconds(
                0.5,
                TimerMode::Repeating,
            )));
    }
}

#[derive(Debug, Component)]
pub struct Mob {
    direction: f32,
}

#[derive(Resource)]
pub struct MobSpawnTimer(Timer);

fn spawn_mob(
    mut commands: Commands,
    time: Res<Time>,
    mut timer: ResMut<MobSpawnTimer>,
    mut entities: Query<(&Name, &mut GodotNodeHandle)>,
    assets: Res<MobAssets>,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    let mut mob_spawn_path_follow = entities
        .iter_mut()
        .find_entity_by_name("MobSpawnLocation")
        .unwrap();

    let mob_spawn_path_follow = mob_spawn_path_follow.get::<PathFollow2D>();

    let mut direction = mob_spawn_path_follow.get_rotation() + PI / 2.0;
    direction += fastrand::f32() * PI / 2.0 - PI / 4.0;

    let position = mob_spawn_path_follow.get_position();

    let transform = GodotTransform2D::IDENTITY.translated(position);
    let transform = transform.rotated(direction as f32);

    commands
        .spawn_empty()
        .insert(Mob { direction })
        .insert(Transform2D(transform))
        .insert(GodotScene::from_resource(assets.mob_scn.clone()));
}
