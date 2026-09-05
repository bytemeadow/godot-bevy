use crate::components::{Gravity, JumpVelocity, Player, Speed};
use crate::gameplay::audio::PlaySfxMessage;
use bevy::app::{App, Plugin};
use bevy::prelude::*;
use godot::classes::AnimatedSprite2D;
use godot::classes::CharacterBody2D;
use godot::global::move_toward;
use godot_bevy::prelude::*;

#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PlayerSystemSet {
    InputDetection,
    Movement,
    Animation,
}

/// Event for player input state
#[derive(Message, Debug, Clone)]
pub struct PlayerInputMessage {
    pub movement_direction: f32,
    pub jump_pressed: bool,
    pub is_on_floor: bool,
}

/// Event for player movement state changes
#[derive(Message, Debug, Clone)]
pub struct PlayerMovementMessage {
    pub is_moving: bool,
    pub is_on_floor: bool,
    pub facing_left: bool,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_message::<PlayerInputMessage>()
            .add_message::<PlayerMovementMessage>()
            .add_systems(
                FixedUpdate,
                (
                    detect_player_input.in_set(PlayerSystemSet::InputDetection),
                    apply_player_movement.in_set(PlayerSystemSet::Movement),
                    update_player_animation.in_set(PlayerSystemSet::Animation),
                )
                    .chain(),
            );
    }
}

fn detect_player_input(
    player: Query<&GodotNodeHandle, With<Player>>,
    mut input_events: MessageWriter<PlayerInputMessage>,
    actions: Res<GodotActions>,
    mut godot: GodotAccess,
) {
    if let Ok(handle) = player.single() {
        // Scene transitions can invalidate the Godot node before this system runs.
        let Some(character_body) = godot.try_get::<CharacterBody2D>(*handle) else {
            return;
        };

        let movement_direction = actions.axis("move_left", "move_right");
        let jump_pressed = actions.just_pressed("jump");
        let is_on_floor = character_body.is_on_floor();

        // Always send input events so movement system knows current input state,
        // including when player releases keys (movement_direction = 0.0)
        input_events.write(PlayerInputMessage {
            movement_direction,
            jump_pressed,
            is_on_floor,
        });
    }
}

fn apply_player_movement(
    mut input_events: MessageReader<PlayerInputMessage>,
    player: Query<(&GodotNodeHandle, &Speed, &JumpVelocity, &Gravity), With<Player>>,
    time: Res<Time>,
    mut commands: Commands,
    mut movement_events: MessageWriter<PlayerMovementMessage>,
    mut godot: GodotAccess,
) {
    if let Ok((handle, speed, jump_velocity, gravity)) = player.single() {
        let Some(mut character_body) = godot.try_get::<CharacterBody2D>(*handle) else {
            return;
        };

        let mut velocity = character_body.get_velocity();
        let mut movement_occurred = false;
        let mut last_movement_direction = 0.0;

        if !character_body.is_on_floor() {
            velocity.y += gravity.0 * time.delta_secs();
        }

        let mut processed_input = false;
        for input_event in input_events.read() {
            processed_input = true;
            last_movement_direction = input_event.movement_direction;

            if input_event.jump_pressed && input_event.is_on_floor {
                velocity.y = jump_velocity.0;
                commands.trigger(PlaySfxMessage::PlayerJump);
            }

            if input_event.movement_direction != 0.0 {
                velocity.x = input_event.movement_direction * speed.0;
                movement_occurred = true;
            } else {
                velocity.x = move_toward(velocity.x as f64, 0.0, speed.0 as f64 / 2.0) as f32;
            }
        }

        if !processed_input {
            velocity.x = move_toward(velocity.x as f64, 0.0, speed.0 as f64 / 2.0) as f32;
        }

        character_body.set_velocity(velocity);
        character_body.move_and_slide();

        movement_events.write(PlayerMovementMessage {
            is_moving: movement_occurred,
            is_on_floor: character_body.is_on_floor(),
            facing_left: last_movement_direction < 0.0,
        });
    }
}

fn update_player_animation(
    mut movement_events: MessageReader<PlayerMovementMessage>,
    player: Query<&GodotNodeHandle, With<Player>>,
    mut godot: GodotAccess,
) {
    if let Ok(handle) = player.single() {
        let Some(character_body) = godot.try_get::<CharacterBody2D>(*handle) else {
            return;
        };

        let mut sprite = character_body.get_node_as::<AnimatedSprite2D>("AnimatedSprite2D");

        for movement_event in movement_events.read() {
            sprite.set_flip_h(movement_event.facing_left);

            if !movement_event.is_on_floor {
                sprite.play_ex().name("jump").done();
            } else if movement_event.is_moving {
                sprite.play_ex().name("run").done();
            } else {
                sprite.play_ex().name("idle").done();
            }
        }
    }
}
