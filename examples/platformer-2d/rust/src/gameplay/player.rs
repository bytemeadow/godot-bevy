use bevy::app::{App, Plugin};
use bevy::prelude::*;
use godot::{
    classes::{CharacterBody2D, ICharacterBody2D},
    prelude::*,
};
use godot_bevy::prelude::*;
use crate::components::{Speed, JumpVelocity, Player};

#[derive(GodotClass, BevyComponent)]
#[class(base=CharacterBody2D)]
#[bevy_component(PlayerBundle((Speed: speed), (JumpVelocity: jump_velocity), (Player)))]
pub struct Player2D {
    base: Base<CharacterBody2D>,
    #[export]
    speed: f32,
    #[export]
    jump_velocity: f32,
}

#[godot_api]
impl ICharacterBody2D for Player2D {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            base,
            speed: 250.,
            jump_velocity: -400.,
        }
    }

    fn ready(&mut self) {}
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PlayerBundleAutoSyncPlugin)
            .add_systems(Update, (test_player_components /*basic_player_movement*/,));
    }
}

/// Test system to verify our generated components are working
fn test_player_components(
    players: Query<(Entity, &Speed, &JumpVelocity), (With<Player>, With<Speed>, With<JumpVelocity>)>,
) {
    for (entity, speed, jump_velocity) in players.iter() {
        godot_print!("Found player entity {:?} with speed: {} and jump velocity: {}", entity, speed.0, jump_velocity.0);
        debug!(
            "Found player entity {:?} with speed: {} and jump velocity: {}",
            entity, speed.0, jump_velocity.0
        );
    }
}

// /// Basic movement system for demonstration
// fn basic_player_movement(
//     mut players: Query<
//         (&mut GodotNodeHandle, &Speed, &JumpVelocity),
//         (With<Speed>, With<JumpVelocity>),
//     >,
//     keyboard: Res<ButtonInput<KeyCode>>,
//     time: Res<Time>,
// ) {
//     for (mut handle, speed, jump_velocity) in players.iter_mut() {
//         if let Some(player_node) = handle.try_get::<Player2D>() {
//             let mut character_body = player_node.upcast::<CharacterBody2D>();
//             let mut velocity = character_body.get_velocity();

//             // Handle horizontal movement
//             let mut direction = 0.0;
//             if keyboard.pressed(KeyCode::ArrowLeft) || keyboard.pressed(KeyCode::KeyA) {
//                 direction -= 1.0;
//             }
//             if keyboard.pressed(KeyCode::ArrowRight) || keyboard.pressed(KeyCode::KeyD) {
//                 direction += 1.0;
//             }

//             velocity.x = direction * speed.0;

//             // Simple gravity (you'd normally handle this in Godot's physics process)
//             if !character_body.is_on_floor() {
//                 velocity.y += 980.0 * time.delta_secs(); // gravity
//             }

//             // Jump
//             if keyboard.just_pressed(KeyCode::Space) && character_body.is_on_floor() {
//                 velocity.y = jump_velocity.0;
//             }

//             character_body.set_velocity(velocity);
//             character_body.move_and_slide();
//         }
//     }
// }
