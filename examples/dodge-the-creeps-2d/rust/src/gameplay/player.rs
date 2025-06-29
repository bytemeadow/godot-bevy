use bevy::prelude::*;

use bevy_asset_loader::asset_collection::AssetCollection;
use godot::{
    builtin::{StringName, Vector2},
    classes::{Input, Node2D},
};
use godot_bevy::{
    plugins::core::PhysicsDelta,
    prelude::{main_thread_system, *},
};

use crate::{
    commands::{AnimationState, CachedScreenSize, VisibilityState},
    nodes::player::Player as GodotPlayerNode,
    GameState,
};

#[derive(AssetCollection, Resource, Debug)]
pub struct PlayerAssets {
    #[asset(path = "scenes/player.tscn")]
    player_scene: Handle<GodotResource>,
}
pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnExit(GameState::Loading), spawn_player)
            .add_systems(Update, player_on_ready)
            .add_systems(
                Update,
                check_player_death.run_if(in_state(GameState::InGame)),
            )
            .add_systems(
                PhysicsUpdate,
                move_player.run_if(in_state(GameState::InGame)),
            )
            .add_systems(OnEnter(GameState::Countdown), setup_player)
            .add_systems(
                PhysicsUpdate,
                move_player.run_if(in_state(GameState::Countdown)),
            );
    }
}

#[derive(Debug, Component)]
pub struct Player {
    speed: f32,
}

#[derive(Debug, Component)]
struct PlayerInitialized;

fn spawn_player(mut commands: Commands, assets: Res<PlayerAssets>) {
    commands
        .spawn_empty()
        .insert(GodotScene::from_handle(assets.player_scene.clone()))
        .insert(Player { speed: 0.0 });
}

#[main_thread_system]
fn player_on_ready(
    mut commands: Commands,
    mut player: Query<
        (Entity, &mut Player, &mut GodotNodeHandle),
        (With<Player>, Without<PlayerInitialized>),
    >,
) -> Result {
    if let Ok((entity, mut player_data, mut player)) = player.single_mut() {
        let player = player.get::<GodotPlayerNode>();
        let screen_size = player.get_viewport_rect().size;
        player_data.speed = player.bind().get_speed();

        // Mark as initialized and add command system components
        commands
            .entity(entity)
            .insert(PlayerInitialized)
            .insert(VisibilityState {
                visible: false,
                dirty: true,
            })
            .insert(AnimationState::default())
            .insert(CachedScreenSize { size: screen_size });
    }

    Ok(())
}

#[main_thread_system]
fn setup_player(
    mut player: Query<(Entity, &mut VisibilityState, &mut Transform2D), With<Player>>,
    mut entities: Query<(&Name, &mut GodotNodeHandle), Without<Player>>,
) -> Result {
    if let Ok((_entity, mut visibility, mut transform)) = player.single_mut() {
        // Set player visible using command system
        visibility.set_visible(true);

        // Still need main thread access for getting start position
        let start_position = entities
            .iter_mut()
            .find_entity_by_name("StartPosition")
            .unwrap()
            .get::<Node2D>()
            .get_position();
        transform.as_godot_mut().origin = start_position;
    }

    Ok(())
}

#[main_thread_system]
fn move_player(
    mut player: Query<(
        &Player,
        &CachedScreenSize,
        &mut Transform2D,
        &mut AnimationState,
    )>,
    physics_delta: Res<PhysicsDelta>,
) -> Result {
    if let Ok((player_data, screen_cache, mut transform, mut anim_state)) = player.single_mut() {
        let mut velocity = Vector2::ZERO;

        // Input handling - can be done without Godot API calls by caching input state
        if Input::singleton().is_action_pressed("move_right") {
            velocity.x += 1.0;
        }

        if Input::singleton().is_action_pressed("move_left") {
            velocity.x -= 1.0;
        }

        if Input::singleton().is_action_pressed("move_down") {
            velocity.y += 1.0;
        }

        if Input::singleton().is_action_pressed("move_up") {
            velocity.y -= 1.0;
        }

        // Animation logic using command system
        if velocity.length() > 0.0 {
            velocity = velocity.normalized() * player_data.speed;

            if velocity.x != 0.0 {
                anim_state.play(Some(StringName::from("walk")));
                anim_state.set_flip(velocity.x < 0.0, false);
            } else if velocity.y != 0.0 {
                anim_state.play(Some(StringName::from("up")));
                anim_state.set_flip(false, velocity.y > 0.0);
            }
        } else {
            anim_state.stop();
        }

        // Transform update using cached screen size
        let mut godot_transform = transform.as_godot_mut();
        godot_transform.origin += velocity * physics_delta.delta_seconds;
        godot_transform.origin.x =
            f32::min(f32::max(0.0, godot_transform.origin.x), screen_cache.size.x);
        godot_transform.origin.y =
            f32::min(f32::max(0.0, godot_transform.origin.y), screen_cache.size.y);
    }

    Ok(())
}

#[main_thread_system]
fn check_player_death(
    mut player: Query<(&mut VisibilityState, &Collisions), With<Player>>,
    mut next_state: ResMut<NextState<GameState>>,
) {
    if let Ok((mut visibility, collisions)) = player.single_mut() {
        if collisions.colliding().is_empty() {
            return;
        }

        visibility.set_visible(false);
        next_state.set(GameState::GameOver);
    }
}
