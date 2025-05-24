use bevy::{
    app::{App, Plugin, Update},
    ecs::{
        component::Component,
        event::EventReader,
        name::Name,
        query::Added,
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Commands, Query, Res, ResMut},
    },
    state::condition::in_state,
    time::{Time, Timer, TimerMode},
};
use godot::{
    builtin::{Transform2D as GodotTransform2D, Vector2},
    classes::{AnimatedSprite2D, Node, PathFollow2D, ResourceLoader, RigidBody2D},
};
use godot_bevy::{
    bridge::{GodotNodeHandle, GodotResourceHandle},
    prelude::{
        connect_godot_signal, FindEntityByNameExt, GodotScene, GodotSignal, NodeTreeView,
        SceneTreeRef, Transform2D,
    },
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
        app.init_resource::<MobAssets>()
            .add_systems(
                Update,
                (spawn_mob, new_mob, kill_mob).run_if(in_state(GameState::InGame)),
            )
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

    // Choose a random location on Path2D.
    let mut mob_spawn_location = entities
        .iter_mut()
        .find_entity_by_name("MobSpawnLocation")
        .unwrap();

    let mut mob_spawn_location = mob_spawn_location.get::<PathFollow2D>();
    mob_spawn_location.set_progress_ratio(fastrand::f32());

    // Set the mob's direction perpendicular to the path direction.
    let mut direction = mob_spawn_location.get_rotation() + PI / 2.0;

    // Add some randomness to the direction.
    direction += fastrand::f32() * PI / 2.0 - PI / 4.0;

    let position = mob_spawn_location.get_position();
    let transform = GodotTransform2D::IDENTITY.translated(position);
    let transform = transform.rotated(direction);

    commands
        .spawn_empty()
        .insert(Mob { direction })
        .insert(Transform2D::from(transform))
        .insert(GodotScene::from_resource(assets.mob_scn.clone()));
}

#[derive(NodeTreeView)]
pub struct MobNodes {
    #[node("AnimatedSprite2D")]
    animated_sprite: GodotNodeHandle,

    #[node("VisibleOnScreenNotifier2D")]
    visibility_notifier: GodotNodeHandle,
}

fn new_mob(
    mut entities: Query<(&Mob, &mut GodotNodeHandle), Added<Mob>>,
    mut scene_tree: SceneTreeRef,
) {
    for (mob_data, mut mob) in entities.iter_mut() {
        let mut mob = mob.get::<RigidBody2D>();

        let velocity = Vector2::new(fastrand::f32() * 100.0 + 150.0, 0.0);
        mob.set_linear_velocity(velocity.rotated(mob_data.direction));

        let mut mob_nodes = MobNodes::from_node(mob);

        let mut animated_sprite = mob_nodes.animated_sprite.get::<AnimatedSprite2D>();
        animated_sprite.play();

        let mob_types = animated_sprite
            .get_sprite_frames()
            .unwrap()
            .get_animation_names();

        let mob_type_index = fastrand::usize(0..mob_types.len());
        animated_sprite.set_animation(mob_types[mob_type_index].arg());

        connect_godot_signal(
            &mut mob_nodes.visibility_notifier,
            "screen_exited",
            &mut scene_tree,
        );
    }
}

fn kill_mob(mut signals: EventReader<GodotSignal>) {
    for signal in signals.read() {
        if signal.name == "screen_exited" {
            GodotNodeHandle::from_instance_id(signal.target)
                .get::<Node>()
                .get_parent()
                .unwrap()
                .queue_free();
        }
    }
}
