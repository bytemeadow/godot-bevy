use crate::gameplay::audio::GameSfxChannel;
use crate::{
    GameState,
    commands::{AnimationState, NodeCommand},
};
use bevy::math::{Vec3Swizzles, vec3};
use bevy::transform::components::Transform;
use bevy::{
    app::{App, Plugin, Update},
    asset::Handle,
    ecs::{
        component::Component,
        event::{EventReader, EventWriter},
        name::Name,
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
    classes::{AnimatedSprite2D, Node, PathFollow2D, RigidBody2D},
};
use godot_bevy::{
    interop::GodotNodeHandle,
    prelude::{
        AudioChannel, FindEntityByNameExt, GodotResource, GodotScene, GodotSignal, GodotSignals,
        NodeTreeView, main_thread_system,
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
        app.add_systems(
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

#[main_thread_system]
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

    // Choose a random location on Path2D - still needs main thread access
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
    let mut transform = Transform::default().with_translation(vec3(position.x, position.y, 0.));
    transform.rotate_z(direction);

    commands
        .spawn_empty()
        .insert(Mob { direction })
        .insert(transform)
        .insert(GodotScene::from_handle(assets.mob_scn.clone()))
        .insert(AnimationState::default());
}

#[derive(NodeTreeView)]
pub struct MobNodes {
    #[node("AnimatedSprite2D")]
    animated_sprite: GodotNodeHandle,

    #[node("VisibleOnScreenNotifier2D")]
    visibility_notifier: GodotNodeHandle,
}

#[main_thread_system]
fn new_mob(
    mut entities: Query<(&Mob, &Transform, &mut GodotNodeHandle, &mut AnimationState), Added<Mob>>,
    sfx_channel: Res<AudioChannel<GameSfxChannel>>,
    assets: Res<MobAssets>,
    signals: GodotSignals,
) {
    for (mob_data, transform, mut mob, mut anim_state) in entities.iter_mut() {
        let mut mob = mob.get::<RigidBody2D>();

        let velocity = Vector2::new(fastrand::f32() * 100.0 + 150.0, 0.0);
        mob.set_linear_velocity(velocity.rotated(mob_data.direction));

        let mut mob_nodes = MobNodes::from_node(mob);

        let animated_sprite = mob_nodes.animated_sprite.get::<AnimatedSprite2D>();

        let mob_types = animated_sprite
            .get_sprite_frames()
            .unwrap()
            .get_animation_names();

        let mob_type_index = fastrand::usize(0..mob_types.len());
        let animation_name = mob_types[mob_type_index].clone();

        // Use animation state instead of direct API calls
        anim_state.play(Some(animation_name.into()));

        signals.connect(&mut mob_nodes.visibility_notifier, "screen_exited");

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

#[main_thread_system]
fn kill_mob(mut signals: EventReader<GodotSignal>, _node_commands: EventWriter<NodeCommand>) {
    for signal in signals.read() {
        if signal.name == "screen_exited" {
            // Get the parent node and queue it for destruction via command
            if let Some(node) = signal.target.clone().try_get::<Node>()
                && let Some(mut parent) = node.get_parent()
            {
                // We need the entity ID to send a destroy command
                // For now, still use direct API but this is safer with try_get
                parent.queue_free();
            }
        }
    }
}
