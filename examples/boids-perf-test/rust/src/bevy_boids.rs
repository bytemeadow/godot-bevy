use bevy::ecs::component::Component;
use godot::prelude::*;
use godot_bevy::prelude::*;

#[derive(Component, Default)]
pub struct BoidsContainer {}

#[derive(GodotClass, BevyBundle)]
#[class(base=Node2D)]
#[bevy_bundle((BoidsContainer), autosync=true)]
pub struct BevyBoids {
    base: Base<Node2D>,
    pub is_running: bool,
}

#[godot_api]
impl INode2D for BevyBoids {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            base,
            is_running: false,
        }
    }
}

#[godot_api]
impl BevyBoids {
    #[func]
    fn get_boid_count(&self) -> i32 {
        0
    }

    #[func]
    fn set_target_boid_count(&self, count: i32) {
        godot_print!("Setting target boid count to {}", count);
    }

    #[func]
    fn start_benchmark(&self, boid_count: i32) {
        godot_print!("Starting benchmark with {} boids", boid_count);
    }

    #[func]
    fn stop_benchmark(&self) {
        godot_print!("Stopping benchmark");
    }
}
