use crate::components::{Gravity, JumpVelocity, Player, Speed};
use crate::gameplay::audio::PlaySfxEvent;
use bevy::app::{App, Plugin};
use bevy::prelude::*;
use godot::classes::CharacterBody2D;
use godot::classes::{AnimatedSprite2D, Input, ProjectSettings};
use godot::global::move_toward;
use godot_bevy::plugins::core::PhysicsDelta;
use godot_bevy::prelude::*;

/// System sets for player operations with better parallelization
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum PlayerSystemSet {
    /// Input detection (can run in parallel with other input systems)
    InputDetection,
    /// Physics and movement (runs after input detection)
    Movement,
    /// Animation updates (runs after movement)
    Animation,
}

/// Event for player input state
#[derive(Event, Debug, Clone)]
pub struct PlayerInputEvent {
    pub movement_direction: f32,
    pub jump_pressed: bool,
    pub is_on_floor: bool,
}

/// Event for player movement state changes
#[derive(Event, Debug, Clone)]
pub struct PlayerMovementEvent {
    pub is_moving: bool,
    pub is_on_floor: bool,
    pub facing_left: bool,
}

#[derive(Bundle, GodotNode)]
#[godot_node(base(CharacterBody2D), class_name(Player2D))]
pub struct PlayerBundle {
    pub player: Player,

    #[export_fields(value(export_type(f32), default(250.0)))]
    pub speed: Speed,

    #[export_fields(value(export_type(f32), default(-400.0)))]
    pub jump_velocity: JumpVelocity,

    #[export_fields(value(export_type(f32), default(ProjectSettings::singleton()
        .get_setting("physics/2d/default_gravity")
        .try_to::<f32>()
        .unwrap_or(980.0))))]
    pub gravity: Gravity,
}

pub struct PlayerPlugin;

impl Plugin for PlayerPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<PlayerInputEvent>()
            .add_event::<PlayerMovementEvent>()
            .add_systems(
                PhysicsUpdate,
                (
                    // Input detection runs first and can run in parallel with other input systems
                    detect_player_input.in_set(PlayerSystemSet::InputDetection),
                    // Movement runs after input detection
                    apply_player_movement.in_set(PlayerSystemSet::Movement),
                    // Animation runs after movement
                    update_player_animation.in_set(PlayerSystemSet::Animation),
                )
                    .chain(), // Ensure proper execution order
            );
    }
}

/// System that detects player input and converts it to events
///
/// Runs in InputDetection set and can execute in parallel with other input systems.
/// Only reads input and writes events, enabling better parallelization.
#[main_thread_system]
fn detect_player_input(
    mut player: Query<&mut GodotNodeHandle, With<Player>>,
    mut input_events: EventWriter<PlayerInputEvent>,
) {
    if let Ok(mut handle) = player.single_mut() {
        // Use try_get to handle case where Godot node might be invalid during scene transitions
        let Some(character_body) = handle.try_get::<CharacterBody2D>() else {
            return; // Node is invalid, skip this frame
        };

        let input = Input::singleton();
        let movement_direction = input.get_axis("move_left", "move_right");
        let jump_pressed = input.is_action_just_pressed("jump");
        let is_on_floor = character_body.is_on_floor();

        // Always send input events so movement system knows current input state,
        // including when player releases keys (movement_direction = 0.0)
        input_events.write(PlayerInputEvent {
            movement_direction,
            jump_pressed,
            is_on_floor,
        });
    }
}

/// System that handles player movement physics based on input events
///
/// Runs in Movement set after input detection. Handles all physics calculations
/// and movement execution separately from input detection.
#[main_thread_system]
fn apply_player_movement(
    mut input_events: EventReader<PlayerInputEvent>,
    mut player: Query<(&mut GodotNodeHandle, &Speed, &JumpVelocity, &Gravity), With<Player>>,
    physics_delta: Res<PhysicsDelta>,
    mut sfx_events: EventWriter<PlaySfxEvent>,
    mut movement_events: EventWriter<PlayerMovementEvent>,
) {
    if let Ok((mut handle, speed, jump_velocity, gravity)) = player.single_mut() {
        let Some(mut character_body) = handle.try_get::<CharacterBody2D>() else {
            return; // Node is invalid, skip this frame
        };

        let mut velocity = character_body.get_velocity();
        let mut movement_occurred = false;
        let mut last_movement_direction = 0.0;

        // Always apply gravity if not on floor
        if !character_body.is_on_floor() {
            velocity.y += gravity.0 * physics_delta.delta_seconds;
        }

        // Process input events (should always have at least one per frame now)
        let mut processed_input = false;
        for input_event in input_events.read() {
            processed_input = true;
            last_movement_direction = input_event.movement_direction;

            // Handle jumping
            if input_event.jump_pressed && input_event.is_on_floor {
                velocity.y = jump_velocity.0;
                sfx_events.write(PlaySfxEvent::PlayerJump);
            }

            // Handle horizontal movement
            if input_event.movement_direction != 0.0 {
                velocity.x = input_event.movement_direction * speed.0;
                movement_occurred = true;
            } else {
                // Player released movement keys - apply deceleration
                velocity.x = move_toward(velocity.x as f64, 0.0, speed.0 as f64 / 2.0) as f32;
            }
        }

        // Fallback: if no input events, apply deceleration (shouldn't happen normally)
        if !processed_input {
            velocity.x = move_toward(velocity.x as f64, 0.0, speed.0 as f64 / 2.0) as f32;
        }

        // Always apply movement and physics
        character_body.set_velocity(velocity);
        character_body.move_and_slide();

        // Send movement event for animation system
        movement_events.write(PlayerMovementEvent {
            is_moving: movement_occurred,
            is_on_floor: character_body.is_on_floor(),
            facing_left: last_movement_direction < 0.0,
        });
    }
}

/// System that updates player animations based on movement events
///
/// Runs in Animation set after movement. Handles all animation state
/// separately from physics and input.
#[main_thread_system]
fn update_player_animation(
    mut movement_events: EventReader<PlayerMovementEvent>,
    mut player: Query<&mut GodotNodeHandle, With<Player>>,
) {
    if let Ok(mut handle) = player.single_mut() {
        let Some(character_body) = handle.try_get::<CharacterBody2D>() else {
            return; // Node is invalid, skip this frame
        };

        let mut sprite = character_body.get_node_as::<AnimatedSprite2D>("AnimatedSprite2D");

        for movement_event in movement_events.read() {
            // Update sprite direction
            sprite.set_flip_h(movement_event.facing_left);

            // Update animation based on state
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
