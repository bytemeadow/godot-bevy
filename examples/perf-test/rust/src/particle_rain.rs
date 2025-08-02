use bevy::{
    ecs::{
        component::Component,
        system::{Commands, Query, Res, ResMut},
    },
    math::Vec2,
    prelude::*,
};

use crate::container::{ParticleContainer, ParticleRain};
use godot::builtin::Color as GodotColor;
use godot::classes::Node as GodotNode;
use godot::prelude::*;
use godot_bevy::prelude::*;

/// Resource tracking simulation state
#[derive(Resource, Default, PartialEq)]
pub struct SimulationState {
    pub is_running: bool,
}

/// Resource tracking particle count
#[derive(Resource, Default)]
pub struct ParticleCount {
    pub target: i32,
    pub current: i32,
}

/// Component for individual particle entities
#[derive(Component, Default)]
pub struct Particle;

/// Marker component for particles that need colorization
#[derive(Component)]
pub struct NeedsColorization;

/// Component storing particle velocity
#[derive(Component, Default)]
pub struct Velocity(pub Vector2);

/// Resource for particle simulation parameters
#[derive(Resource, Debug)]
pub struct ParticleConfig {
    pub world_bounds: Vec2,
    pub gravity: f32,
    pub min_speed: f32,
    pub max_speed: f32,
    pub horizontal_drift: f32,
}

impl Default for ParticleConfig {
    fn default() -> Self {
        Self {
            world_bounds: Vec2::new(1920.0, 1080.0),
            gravity: 200.0,         // pixels per second^2
            min_speed: 50.0,        // minimum fall speed
            max_speed: 300.0,       // maximum fall speed
            horizontal_drift: 50.0, // max horizontal drift
        }
    }
}

/// Resource that holds the particle scene reference
#[derive(Resource, Debug)]
struct ParticleScene(Handle<GodotResource>);

/// Plugin for particle rain simulation
pub struct ParticleRainPlugin;

impl Plugin for ParticleRainPlugin {
    fn build(&self, app: &mut App) {
        if cfg!(debug_assertions) {
            warn!("Running a debug build, performance will be significantly worse than release");
        } else {
            info!("Running a release build");
        };

        app.init_resource::<ParticleConfig>()
            .init_resource::<SimulationState>()
            .init_resource::<ParticleCount>()
            .add_systems(Startup, load_assets)
            // Game logic systems
            .add_systems(
                Update,
                (
                    sync_container_params,
                    handle_particle_count,
                    stop_simulation,
                    colorize_new_particles,
                )
                    .chain(),
            )
            // Movement systems
            .add_systems(
                Update,
                (particle_movement,)
                    .run_if(|state: Res<SimulationState>| state.is_running)
                    .after(sync_container_params),
            );

        // Add custom transform sync systems for Particle entities only
        add_transform_sync_systems! {
            app,
            Particle = bevy_to_godot: With<Particle>
        }
    }
}

/// Load the particle scene asset
fn load_assets(mut commands: Commands, server: Res<AssetServer>) {
    let handle: Handle<GodotResource> = server.load("scenes/particle.tscn");
    commands.insert_resource(ParticleScene(handle));
}

/// Synchronize parameters from the container to Bevy resources
#[main_thread_system]
fn sync_container_params(
    mut particle_count: ResMut<ParticleCount>,
    mut config: ResMut<ParticleConfig>,
    mut simulation_state: ResMut<SimulationState>,
    container_query: Query<&GodotNodeHandle, With<ParticleContainer>>,
) {
    for handle in container_query.iter() {
        let mut handle_clone = handle.clone();
        if let Some(mut particle_rain) = handle_clone.try_get::<ParticleRain>() {
            let rain_bind = particle_rain.bind();

            // Update simulation state
            simulation_state.is_running = rain_bind.is_running;

            // Update world bounds
            let screen_size = rain_bind.screen_size;
            if screen_size.x > 0.0 && screen_size.y > 0.0 {
                config.world_bounds = Vec2::new(screen_size.x, screen_size.y);
            }

            // Update target particle count
            particle_count.target = rain_bind.target_particle_count;

            // Update current count back to Godot node
            let current_count = particle_count.current;
            drop(rain_bind); // Release the bind before getting mutable access
            let mut rain_mut = particle_rain.bind_mut();
            rain_mut.current_particle_count = current_count;
        }
    }
}

/// System that handles spawning and despawning particles
fn handle_particle_count(
    mut commands: Commands,
    mut particle_count: ResMut<ParticleCount>,
    particles: Query<(Entity, &GodotNodeHandle), With<Particle>>,
    simulation_state: Res<SimulationState>,
    config: Res<ParticleConfig>,
    particle_scene: Res<ParticleScene>,
) {
    // Skip spawning/despawning if simulation isn't running
    if !simulation_state.is_running {
        return;
    }

    // Count current particles
    let current_count = particles.iter().count() as i32;
    particle_count.current = current_count;

    let target_count = particle_count.target;

    // Spawn new particles if needed (increased batch size for better amortization)
    if current_count < target_count {
        let to_spawn = (target_count - current_count).min(100);
        spawn_particles(&mut commands, to_spawn, &config, &particle_scene);
    }
    // Despawn excess particles if needed (increased batch size)
    else if current_count > target_count {
        let to_despawn = (current_count - target_count).min(100);
        despawn_particles(&mut commands, to_despawn, &particles);
    }
}

/// Helper function to spawn a batch of particles
fn spawn_particles(
    commands: &mut Commands,
    count: i32,
    config: &ParticleConfig,
    particle_scene: &ParticleScene,
) {
    for _ in 0..count {
        // Create position at the top of the screen with random x
        let pos = Vector2::new(
            fastrand::f32() * config.world_bounds.x,
            -50.0, // Start above the screen
        );

        // Create downward velocity with some randomization
        let fall_speed = config.min_speed + fastrand::f32() * (config.max_speed - config.min_speed);
        let horizontal_speed = (fastrand::f32() - 0.5) * config.horizontal_drift;
        let velocity = Vector2::new(horizontal_speed, fall_speed);
        let transform = Transform::from_translation(Vec3::new(pos.x, pos.y, 0.0));

        let entity = commands
            .spawn_empty()
            .insert(GodotScene::from_handle(particle_scene.0.clone()))
            .insert((Particle, Velocity(velocity), transform))
            .id();

        // We'll set the color after the entity is spawned in the next frame
        // by using a marker component
        commands.entity(entity).insert(NeedsColorization);
    }
}

/// Helper function to despawn a batch of particles
fn despawn_particles(
    commands: &mut Commands,
    count: i32,
    particles: &Query<(Entity, &GodotNodeHandle), With<Particle>>,
) {
    // Get entities to despawn
    let entities_to_despawn: Vec<(Entity, GodotNodeHandle)> = particles
        .iter()
        .take(count as usize)
        .map(|(entity, handle)| (entity, handle.clone()))
        .collect();

    // Despawn each entity and free the Godot node
    for (entity, handle) in entities_to_despawn {
        let mut handle_clone = handle.clone();
        if let Some(mut node) = handle_clone.try_get::<GodotNode>() {
            node.queue_free();
        }
        commands.entity(entity).despawn();
    }
}

/// Update simulation state and manage cleanup on stop
#[main_thread_system]
fn stop_simulation(
    simulation_state: Res<SimulationState>,
    mut commands: Commands,
    particles: Query<(Entity, &GodotNodeHandle), With<Particle>>,
) {
    // If simulation was just stopped, clean up all particles
    if !simulation_state.is_running && particles.iter().count() > 0 {
        // Queue all Godot nodes for deletion
        for (entity, handle) in particles.iter() {
            let mut handle_clone = handle.clone();
            if let Some(mut node) = handle_clone.try_get::<GodotNode>() {
                node.queue_free();
            }
            commands.entity(entity).despawn();
        }
    }
}

/// Colorize newly spawned particles
#[main_thread_system]
fn colorize_new_particles(
    mut commands: Commands,
    new_particles: Query<(Entity, &GodotNodeHandle), With<NeedsColorization>>,
) {
    for (entity, handle) in new_particles.iter() {
        let mut handle_clone = handle.clone();

        // Generate random color (semi-transparent)
        let random_color =
            GodotColor::from_rgba(fastrand::f32(), fastrand::f32(), fastrand::f32(), 0.8);

        // Try different node structures
        if let Some(mut node) = handle_clone.try_get::<Node2D>() {
            // Check for Sprite child node
            if node.has_node("Sprite") {
                let mut sprite = node.get_node_as::<Node2D>("Sprite");
                sprite.set_modulate(random_color);
            }
            // If it's a Sprite2D directly, set its modulate
            else if let Some(mut sprite) = handle_clone.try_get::<godot::classes::Sprite2D>() {
                sprite.set_modulate(random_color);
            }
            // Fallback: set modulate on the main node
            else {
                node.set_modulate(random_color);
            }
        }

        // Remove the marker component
        commands.entity(entity).remove::<NeedsColorization>();
    }
}

/// System to move particles downward and wrap them around
fn particle_movement(
    mut particle_query: Query<(&mut Transform, &mut Velocity), With<Particle>>,
    time: Res<Time>,
    config: Res<ParticleConfig>,
) {
    let delta = time.delta_secs();

    particle_query
        .par_iter_mut()
        .for_each(|(mut transform, mut velocity)| {
            // Apply gravity to velocity
            velocity.0.y += config.gravity * delta;

            // Clamp velocity to reasonable bounds
            if velocity.0.y > config.max_speed {
                velocity.0.y = config.max_speed;
            }

            // Work directly with Bevy Transform - much more efficient!
            let current_pos = transform.translation;

            // Calculate new position using Bevy's Vec3
            let velocity_delta = Vec3::new(velocity.0.x, velocity.0.y, 0.0) * delta;
            let new_pos = current_pos + velocity_delta;

            // Wrap particles that fall off the bottom back to the top
            let wrapped_pos = if new_pos.y > config.world_bounds.y + 50.0 {
                Vec3::new(
                    fastrand::f32() * config.world_bounds.x, // Random x position
                    -50.0,                                   // Above the screen
                    0.0,                                     // Keep z at 0
                )
            } else {
                new_pos
            };

            // Reset velocity if wrapped (for variety)
            if wrapped_pos.y < 0.0 && new_pos.y > config.world_bounds.y {
                let fall_speed =
                    config.min_speed + fastrand::f32() * (config.max_speed - config.min_speed);
                let horizontal_speed = (fastrand::f32() - 0.5) * config.horizontal_drift;
                velocity.0 = Vector2::new(horizontal_speed, fall_speed);
            }

            // Write new position directly to Bevy Transform - no conversions!
            transform.translation = wrapped_pos;
        });
}
