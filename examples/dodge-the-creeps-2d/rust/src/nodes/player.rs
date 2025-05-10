use godot::classes::{AnimatedSprite2D, Area2D, IArea2D};
use godot::prelude::*;
use crate::gd_node_as;

#[derive(GodotClass)]
#[class(base=Area2D)]
pub struct Player {
  base: Base<Area2D>,
  screen_size: Vector2,
  #[export]
  speed: f32,
}

#[godot_api]
impl IArea2D for Player {
    fn init(base: Base<Area2D>) -> Self {
        Self {
            base,
            screen_size: Vector2::new(0.0, 0.0),
            speed: 400.,
        }
    }

    fn ready(&mut self) {
      godot_print!("Player ready");
      let viewport = self.base().get_viewport_rect();
      self.screen_size = viewport.size;

      self.base_mut().hide();
    }

    fn physics_process(&mut self, delta: f64) {
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

      let mut sprite = gd_node_as!(self, "AnimatedSprite2D", AnimatedSprite2D);
      
      if velocity.length() > 0.0 {
        velocity = velocity.normalized() * self.speed;
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

      let current_pos = self.base().get_position();
      self.base_mut().set_position(current_pos + (velocity * delta as f32));
    }
}