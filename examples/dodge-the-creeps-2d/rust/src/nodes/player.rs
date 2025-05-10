use godot::classes::{Area2D, IArea2D};
use godot::prelude::*;

#[derive(GodotClass)]
#[class(base=Area2D)]
pub struct Player {
  base: Base<Area2D>,
}

#[godot_api]
impl IArea2D for Player {
    fn init(base: Base<Area2D>) -> Self {
        Self {
            base,
        }
    }

    fn ready(&mut self) {
      godot_print!("Player ready")
    }
}