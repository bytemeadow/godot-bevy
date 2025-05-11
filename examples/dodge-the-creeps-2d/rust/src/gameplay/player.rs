use bevy::prelude::*;

use godot::{builtin::Vector2, classes::ResourceLoader, global::godot_print};
use godot_bevy::prelude::*;

use crate::{nodes::player::Player as GodotPlayerNode, GameState};

#[derive(Debug, Resource)]
pub struct PlayerAssets {
    player_scn: GodotResourceRef,
}

impl Default for PlayerAssets {
    fn default() -> Self {
        let mut resource_loader = ResourceLoader::singleton();
        let player_scn = GodotResourceRef::new(resource_loader.load("scenes/player.tscn").unwrap());

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
    speed: f64,
}

#[derive(Debug, Component)]
pub struct PlayerInitialized;

fn spawn_player(mut commands: Commands, assets: Res<PlayerAssets>) {
    godot_print!("spawn_player");

    commands.spawn((
        GodotScene::from_resource(assets.player_scn.clone()),
        // This will be replaced by PlayerNode exported property
        Player { speed: 0.0 },
    ));
}

fn player_on_ready(
    mut commands: Commands,
    mut player: Query<(Entity, &mut GodotRef), (With<Player>, Without<PlayerInitialized>)>,
) -> Result {
    if let Ok((entity, mut player_gd)) = player.single_mut() {
        let mut player_gd = player_gd.get::<GodotPlayerNode>();
        player_gd.set_visible(false);

        // TODO: pull start position from scene
        // let mut start_position = PlayerStartPosition::from_node(player);
        // player_gd.set_position(start_position.0.get::<Node2D>().position());
        player_gd.set_position(Vector2::new(240., 450.));

        // Mark as initialized so we don't do this again
        commands.entity(entity).insert(PlayerInitialized);
    }

    Ok(())
}

fn setup_player(
    mut player: Query<&mut GodotRef, (With<Player>, With<PlayerInitialized>)>,
) -> Result {
    if let Ok(mut player_gd) = player.single_mut() {
        let mut player_gd = player_gd.get::<GodotPlayerNode>();
        player_gd.set_visible(true);
    }

    Ok(())
}

fn move_player(
    mut player: Query<(&Player, &mut GodotRef), With<PlayerInitialized>>,
    mut _system_delta: SystemDeltaTimer,
) -> Result {
    godot_print!("move_player");
    Ok(())
}
