use bevy::ecs::component::Component;
use godot::prelude::*;
use godot_bevy::prelude::*;

#[derive(Component, Default)]
pub struct ParticleContainer;

#[derive(GodotClass, BevyBundle)]
#[class(base=Node2D)]
#[bevy_bundle((ParticleContainer))]
pub struct ParticleRain {
    base: Base<Node2D>,
    pub is_running: bool,
    pub screen_size: Vector2,
    pub current_particle_count: i32,
    pub target_particle_count: i32,
}

#[godot_api]
impl INode2D for ParticleRain {
    fn init(base: Base<Node2D>) -> Self {
        Self {
            base,
            is_running: false,
            screen_size: Vector2::new(1920.0, 1080.0),
            current_particle_count: 0,
            target_particle_count: 0,
        }
    }

    fn ready(&mut self) {
        let viewport_size = self.base().get_viewport_rect().size;
        self.screen_size = viewport_size;
    }
}

#[godot_api]
impl ParticleRain {
    #[func]
    fn get_particle_count(&self) -> i32 {
        self.current_particle_count
    }

    #[func]
    fn set_target_particle_count(&mut self, count: i32) {
        self.target_particle_count = count;
    }

    #[func]
    pub fn start_benchmark(&mut self, particle_count: i32) {
        self.is_running = true;
        self.target_particle_count = particle_count;
    }

    #[func]
    pub fn stop_benchmark(&mut self) {
        self.is_running = false;
    }
}
