use godot::prelude::*;
use godot_bevy::prelude::AttachableComponent;

use crate::components::JumpBoost;

#[derive(AttachableComponent, GodotClass)]
#[class(init, base = Node)]
#[gdbevy(target = JumpBoost)]
pub struct JumpBoostCarrier {
    #[export]
    #[init(val = 1.5)]
    pub multiplier: f32,
}

impl From<&JumpBoostCarrier> for JumpBoost {
    fn from(carrier: &JumpBoostCarrier) -> Self {
        Self {
            multiplier: carrier.multiplier,
        }
    }
}
