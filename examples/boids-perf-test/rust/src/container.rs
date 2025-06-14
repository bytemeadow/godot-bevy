use bevy::ecs::component::Component;
use godot::prelude::*;
use godot_bevy::prelude::*;

#[derive(Component, Default)]
pub struct BoidsContainer;

#[derive(GodotClass, BevyBundle)]
#[class(base=Node2D)]
#[bevy_bundle((BoidsContainer))]
pub struct BevyBoids {
    base: Base<Node2D>,
    pub is_running: bool,
    pub screen_size: Vector2,
    pub current_boid_count: i32,
    pub target_boid_count: i32,
}

#[godot_api]
impl INode2D for BevyBoids {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            base,
            is_running: false,
            screen_size: Vector2::new(1920.0, 1080.0),
            current_boid_count: 0,
            target_boid_count: 0,
        }
    }

    fn ready(&mut self) {
        let viewport_size = self.base().get_viewport_rect().size;
        self.screen_size = viewport_size;
    }
}

#[godot_api]
impl BevyBoids {
    #[func]
    fn get_boid_count(&self) -> i32 {
        self.current_boid_count
    }

    #[func]
    fn set_target_boid_count(&mut self, count: i32) {
        self.target_boid_count = count;
    }

    #[func]
    pub fn start_benchmark(&mut self, boid_count: i32) {
        self.is_running = true;
        self.target_boid_count = boid_count;
    }

    #[func]
    pub fn stop_benchmark(&mut self) {
        self.is_running = false;
    }
}
