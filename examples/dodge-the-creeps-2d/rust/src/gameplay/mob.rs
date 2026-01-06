use crate::gameplay::audio::GameSfxChannel;
use crate::{GameState, commands::AnimationState};
use bevy::math::{Vec3Swizzles, vec3};
use bevy::transform::components::Transform;
use bevy::{
    app::{App, Plugin, Update},
    asset::Handle,
    ecs::{
        component::Component,
        entity::Entity,
        event::Event,
        name::Name,
        observer::On,
        query::Added,
        resource::Resource,
        schedule::IntoScheduleConfigs,
        system::{Commands, Query, Res, ResMut},
    },
    log::info,
    state::condition::in_state,
    time::{Time, Timer, TimerMode},
};
use bevy_asset_loader::asset_collection::AssetCollection;
use godot::{
    builtin::Vector2,
    classes::{AnimatedSprite2D, PathFollow2D, RigidBody2D},
};
use godot_bevy::interop::VisibleOnScreenNotifier2DSignals;
use godot_bevy::{
    interop::GodotNodeHandle,
    prelude::{
        AudioChannel, FindEntityByNameExt, GodotAccess, GodotResource, GodotScene,
        GodotTypedSignalsPlugin, NodeTreeView,
    },
};
use std::f32::consts::PI;

#[derive(AssetCollection, Resource, Debug)]
pub struct MobAssets {
    #[asset(path = "scenes/mob.tscn")]
    mob_scn: Handle<GodotResource>,

    #[asset(path = "audio/plop.ogg")]
    pub mob_pop: Handle<GodotResource>,
}

pub struct MobPlugin;

impl Plugin for MobPlugin {
    fn build(&self, app: &mut App) {
        app
            // enable typed signal routing for mob screen exit
            .add_plugins(GodotTypedSignalsPlugin::<MobScreenExited>::default())
            .add_systems(
                Update,
                (spawn_mob, new_mob).run_if(in_state(GameState::InGame)),
            )
            // Use observer for mob screen exit
            .add_observer(on_mob_screen_exited)
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
    entities: Query<(&Name, &GodotNodeHandle)>,
    assets: Res<MobAssets>,
    mut godot: GodotAccess,
) {
    timer.0.tick(time.delta());
    if !timer.0.just_finished() {
        return;
    }

    // Choose a random location on Path2D - still needs main thread access
    let mob_spawn_handle = entities
        .iter()
        .find_entity_by_name("MobSpawnLocation")
        .unwrap();

    let mut mob_spawn_location = godot.get::<PathFollow2D>(*mob_spawn_handle);
    mob_spawn_location.set_progress_ratio(fastrand::f32());

    // Set the mob's direction perpendicular to the path direction.
    let mut direction = mob_spawn_location.get_rotation() + PI / 2.0;

    // Add some randomness to the direction.
    direction += fastrand::f32() * PI / 2.0 - PI / 4.0;

    let position = mob_spawn_location.get_position();
    let mut transform = Transform::default().with_translation(vec3(position.x, position.y, 0.));
    transform.rotate_z(direction);

    commands
        .spawn_empty()
        .insert(Mob { direction })
        .insert(transform)
        .insert(
            GodotScene::from_handle(assets.mob_scn.clone()).with_signal_connection(
                MobNodes::VISIBILITY_NOTIFIER_PATH,
                VisibleOnScreenNotifier2DSignals::SCREEN_EXITED,
                |_args, _node_handle, entity| {
                    Some(MobScreenExited {
                        entity: entity.expect("entity was provided"),
                    })
                },
            ),
        )
        .insert(AnimationState::default());
}

#[derive(NodeTreeView)]
pub struct MobNodes {
    #[node("AnimatedSprite2D")]
    animated_sprite: GodotNodeHandle,

    #[node("VisibleOnScreenNotifier2D")]
    _visibility_notifier: GodotNodeHandle,
}

fn new_mob(
    mut entities: Query<(&Mob, &Transform, &GodotNodeHandle, &mut AnimationState), Added<Mob>>,
    sfx_channel: Res<AudioChannel<GameSfxChannel>>,
    assets: Res<MobAssets>,
    mut godot: GodotAccess,
) {
    for (mob_data, transform, mob_handle, mut anim_state) in entities.iter_mut() {
        let mut mob = godot.get::<RigidBody2D>(*mob_handle);

        let velocity = Vector2::new(fastrand::f32() * 100.0 + 150.0, 0.0);
        mob.set_linear_velocity(velocity.rotated(mob_data.direction));

        let mob_nodes = MobNodes::from_node(mob).unwrap();

        let animated_sprite = godot.get::<AnimatedSprite2D>(mob_nodes.animated_sprite);

        let mob_types = animated_sprite
            .get_sprite_frames()
            .unwrap()
            .get_animation_names();

        let mob_type_index = fastrand::usize(0..mob_types.len());
        let animation_name = &mob_types[mob_type_index].clone();

        // Use animation state instead of direct API calls
        anim_state.play(Some(animation_name.into()));

        // Play 2D positional spawn sound at mob's position with fade-in
        let position = transform.translation.xy();

        sfx_channel
            .play_2d(assets.mob_pop.clone(), position)
            .volume(0.9)
            .pitch(0.8 + fastrand::f32() * 0.4);

        info!(
            "Mob spawned at position: {:?} with 2D positional audio and fade-in",
            position
        );
    }
}

#[derive(Event, Debug, Clone, Copy)]
struct MobScreenExited {
    entity: Entity,
}

fn on_mob_screen_exited(trigger: On<MobScreenExited>, mut commands: Commands) {
    let entity = trigger.event().entity;
    commands.entity(entity).despawn();
}
