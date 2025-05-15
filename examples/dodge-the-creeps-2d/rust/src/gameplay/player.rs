use bevy::prelude::*;

use godot::{
    builtin::{StringName, Vector2},
    classes::{AnimatedSprite2D, Input, Node2D, ResourceLoader},
};
use godot_bevy::prelude::*;

use crate::{nodes::player::Player as GodotPlayerNode, GameState};

#[derive(Debug, Resource)]
pub struct PlayerAssets {
    player_scn: GodotResourceHandle,
}

impl Default for PlayerAssets {
    fn default() -> Self {
        let mut resource_loader = ResourceLoader::singleton();
        let player_scn =
            GodotResourceHandle::new(resource_loader.load("scenes/player.tscn").unwrap());

        Self { player_scn }
    }
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PlayerAssets>()
            .add_systems(OnEnter(GameState::InGame), spawn_player)
            .add_systems(
                Update,
                (player_on_ready, setup_player.after(player_on_ready)),
            )
            .add_systems(
                Update,
                (move_player.as_physics_system()/*, check_player_death */)
                    .run_if(in_state(GameState::InGame)),
            );
    }
}

#[derive(Debug, Component)]
pub struct Player {
    speed: f32,
}

#[derive(Debug, Component)]
pub struct PlayerCreated;

#[derive(Debug, Component)]
pub struct PlayerSetUp;

fn spawn_player(mut commands: Commands, assets: Res<PlayerAssets>) {
    commands.spawn((
        GodotScene::from_resource(assets.player_scn.clone()),
        // This will be replaced by PlayerNode exported property
        Player { speed: 0.0 },
    ));
}

#[derive(NodeTreeView)]
pub struct PlayerStartPosition(
    #[node("/root/Main/StartPosition")]
    GodotNodeHandle
);

fn player_on_ready(
    mut commands: Commands,
    mut player: Query<
        (Entity, &mut Player, &mut GodotNodeHandle),
        (With<Player>, Without<PlayerCreated>),
    >,
) -> Result {
    if let Ok((entity, mut player, mut player_gd)) = player.single_mut() {
        let mut player_gd = player_gd.get::<GodotPlayerNode>();
        player_gd.hide();
        player.speed = player_gd.bind().get_speed();

        let mut start_position = PlayerStartPosition::from_node(player_gd.clone());
        player_gd.set_position(start_position.0.get::<Node2D>().get_position());

        // Mark as initialized so we don't do this again
        commands.entity(entity).insert(PlayerCreated);
    }

    Ok(())
}

fn setup_player(
    mut commands: Commands,
    mut player: Query<(Entity, &mut GodotNodeHandle), (With<Player>, With<PlayerCreated>)>,
) -> Result {
    if let Ok((entity, mut player_gd)) = player.single_mut() {
        let mut player_gd = player_gd.get::<GodotPlayerNode>();
        player_gd.show();

        // Mark as setup so we don't do this again
        commands.entity(entity).insert(PlayerSetUp);
    }

    Ok(())
}

fn move_player(
    mut player: Query<
        (&Player, &mut GodotNodeHandle, &mut Transform2D),
        (With<Player>, With<PlayerSetUp>),
    >,
    mut system_delta: SystemDeltaTimer,
) -> Result {
    if let Ok((player, mut player_gd, mut transform)) = player.single_mut() {
        let player_gd = player_gd.get::<GodotPlayerNode>();
        let screen_size = player_gd.get_viewport_rect().size;
        let mut velocity = Vector2::ZERO;

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

        let mut sprite = player_gd.get_node_as::<AnimatedSprite2D>("AnimatedSprite2D");

        if velocity.length() > 0.0 {
            velocity = velocity.normalized() * player.speed;
            sprite.play();

            if velocity.x != 0.0 {
                sprite.set_animation(&StringName::from("walk"));
                sprite.set_flip_v(false);
                sprite.set_flip_h(velocity.x < 0.0);
            } else if velocity.y != 0.0 {
                sprite.set_animation(&StringName::from("up"));
                sprite.set_flip_v(velocity.y > 0.0);
            }
        } else {
            sprite.stop();
        }

        transform.origin += velocity * system_delta.delta_seconds();
        transform.origin.x = f32::min(f32::max(0.0, transform.origin.x), screen_size.x);
        transform.origin.y = f32::min(f32::max(0.0, transform.origin.y), screen_size.y);
    }

    Ok(())
}
