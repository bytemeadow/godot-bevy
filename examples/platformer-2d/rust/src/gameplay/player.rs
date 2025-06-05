use bevy::app::{App, Plugin};
use godot::{classes::{CharacterBody2D, ICharacterBody2D}, prelude::*};

#[derive(GodotClass)]
#[class(base=CharacterBody2D)]
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
        app;
    }
}
