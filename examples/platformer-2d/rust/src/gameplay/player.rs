use crate::components::{Gravity, JumpVelocity, Player, Speed};
use crate::gameplay::audio::PlaySfxEvent;
use bevy::app::{App, Plugin};
use bevy::prelude::*;
use godot::classes::{AnimatedSprite2D, Input, ProjectSettings};
use godot::global::move_toward;
use godot::{
    classes::{CharacterBody2D, ICharacterBody2D},
    prelude::*,
};
use godot_bevy::plugins::core::PhysicsDelta;
use godot_bevy::prelude::*;

#[derive(GodotClass, BevyBundle)]
#[class(base=CharacterBody2D)]
#[bevy_bundle((Speed: speed), (JumpVelocity: jump_velocity), (Gravity: gravity), (Player))]
pub struct Player2D {
    base: Base<CharacterBody2D>,
    #[export]
    speed: f32,
    #[export]
    jump_velocity: f32,
    gravity: f32,
}

#[godot_api]
impl ICharacterBody2D for Player2D {
    fn init(base: Base<CharacterBody2D>) -> Self {
        Self {
            base,
            speed: 250.,
            jump_velocity: -400.,
            gravity: ProjectSettings::singleton()
                .get_setting("physics/2d/default_gravity")
                .try_to::<f32>()
                .unwrap_or(980.0),
        }
    }

    fn ready(&mut self) {}
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(Player2DBundleAutoSyncPlugin)
            .add_systems(PhysicsUpdate, basic_player_movement);
    }
}

fn basic_player_movement(
    mut player: Query<(&mut GodotNodeHandle, &Speed, &JumpVelocity, &Gravity), With<Player>>,
    physics_delta: Res<PhysicsDelta>,
    mut sfx_events: EventWriter<PlaySfxEvent>,
) {
    if let Ok((mut handle, speed, jump_velocity, gravity)) = player.single_mut() {
        // Use try_get to handle case where Godot node might be invalid during scene transitions
        let Some(mut character_body) = handle.try_get::<CharacterBody2D>() else {
            return; // Node is invalid, skip this frame
        };

        let input = Input::singleton();
        let mut sprite = character_body.get_node_as::<AnimatedSprite2D>("AnimatedSprite2D");
        let mut velocity = character_body.get_velocity();

        if !character_body.is_on_floor() {
            velocity.y += gravity.0 * physics_delta.delta_seconds;
        }

        if input.is_action_just_pressed("jump") && character_body.is_on_floor() {
            velocity.y = jump_velocity.0;
            sfx_events.write(PlaySfxEvent::PlayerJump);
        }

        let direction = input.get_axis("move_left", "move_right");
        if direction != 0.0 {
            velocity.x = direction * speed.0;
            sprite.play_ex().name("run").done();
            sprite.set_flip_h(direction == -1.0);
        } else {
            sprite.play_ex().name("idle").done();
            velocity.x = move_toward(velocity.x as f64, 0.0, speed.0 as f64 / 2.0) as f32;
        }

        if !character_body.is_on_floor() {
            sprite.play_ex().name("jump").done();
        }

        character_body.set_velocity(velocity);
        character_body.move_and_slide();
    }
}
