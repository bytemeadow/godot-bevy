use std::collections::HashMap;

use bevy::{
    ecs::{
        component::Component,
        schedule::SystemSet,
        system::{Commands, Query, Res, ResMut},
    },
    math::Vec2,
    prelude::*,
};
use fastrand;

use godot::prelude::*;
use godot_bevy::plugins::core::Transform2D;
use godot_bevy::prelude::*;

use crate::container::{BevyBoids, BoidsContainer};

/// Resource that holds the boid scene reference
#[derive(Resource, Debug)]
struct BoidScene(Handle<GodotResource>);

/// Resource tracking simulation state
#[derive(Resource, Default, PartialEq)]
pub struct SimulationState {
    pub is_running: bool,
}

/// Resource tracking boid count
#[derive(Resource, Default)]
pub struct BoidCount {
    pub target: i32,
    pub current: i32,
}


/// Component for individual boid entities
#[derive(Component)]
pub struct Boid;

/// Component storing boid velocity
#[derive(Component)]
pub struct Velocity(pub Vector2);

impl Default for Velocity {
    fn default() -> Self {
        Self(Vector2::ZERO)
    }
}

/// Component storing accumulated steering forces
#[derive(Component, Default)]
pub struct SteeringForces {
    pub separation: Vector2,
    pub alignment: Vector2,
    pub cohesion: Vector2,
    pub boundary: Vector2,
}

/// Component storing grid cell position for spatial optimization
#[derive(Component, Default, Clone, Copy)]
pub struct GridCell {
    pub x: i32,
    pub y: i32,
}

/// Resource for boids simulation parameters
#[derive(Resource)]
pub struct BoidsConfig {
    pub world_bounds: Vec2,
    pub max_speed: f32,
    pub max_force: f32,
    pub perception_radius: f32,
    pub separation_radius: f32,
    pub separation_weight: f32,
    pub alignment_weight: f32,
    pub cohesion_weight: f32,
    pub boundary_weight: f32,
}

impl Default for BoidsConfig {
    fn default() -> Self {
        Self {
            world_bounds: Vec2::new(1920.0, 1080.0),
            max_speed: 150.0,
            max_force: 5.0,
            perception_radius: 50.0,
            separation_radius: 25.0,
            separation_weight: 2.0,
            alignment_weight: 1.0,
            cohesion_weight: 1.0,
            boundary_weight: 3.0,
        }
    }
}

/// Resource for spatial grid optimization
#[derive(Resource)]
pub struct SpatialGrid {
    pub cell_size: f32,
    pub grid: HashMap<(i32, i32), Vec<Entity>>,
}

impl Default for SpatialGrid {
    fn default() -> Self {
        Self {
            cell_size: 75.0,
            grid: HashMap::new(),
        }
    }
}

/// Resource for performance tracking
#[derive(Resource)]
pub struct PerformanceTracker {
    pub frame_count: u32,
    pub last_log_time: f32,
}

impl Default for PerformanceTracker {
    fn default() -> Self {
        Self {
            frame_count: 0,
            last_log_time: 0.0,
        }
    }
}

/// System sets for organizing boids simulation
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum BoidsSystemSet {
    NeighborDetection,
    ForceCalculation,
    VelocityIntegration,
    PositionUpdate,
    Visualization,
}

/// Plugin for boids simulation
pub struct BoidsPlugin;

impl Plugin for BoidsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<BoidsConfig>()
            .init_resource::<SimulationState>()
            .init_resource::<BoidCount>()
            .init_resource::<SpatialGrid>()
            .init_resource::<PerformanceTracker>()
            .add_systems(Startup, load_assets)
            // Game logic systems
            .add_systems(
                Update,
                (
                    sync_container_params,
                    handle_boid_count,
                    update_simulation_state,
                    log_performance,
                )
                    .chain(),
            )
            // Movement systems - use Update schedule like GDScript _process()
            .add_systems(
                Update,
                (
                    build_spatial_grid.in_set(BoidsSystemSet::NeighborDetection),
                    calculate_all_forces
                        .in_set(BoidsSystemSet::ForceCalculation)
                        .after(BoidsSystemSet::NeighborDetection),
                    apply_steering_forces
                        .in_set(BoidsSystemSet::VelocityIntegration)
                        .after(BoidsSystemSet::ForceCalculation),
                    update_boid_transforms
                        .in_set(BoidsSystemSet::PositionUpdate)
                        .after(BoidsSystemSet::VelocityIntegration),
                )
                    .run_if(|state: Res<SimulationState>| state.is_running)
                    .after(sync_container_params), // Run after parameter sync
            );
    }
}

/// Load the boid scene asset
fn load_assets(mut commands: Commands, server: Res<AssetServer>) {
    let handle: Handle<GodotResource> = server.load("scenes/boid.tscn");
    commands.insert_resource(BoidScene(handle));
}

/// Synchronize parameters from the container to Bevy resources (simplified like GDScript)
fn sync_container_params(
    mut boid_count: ResMut<BoidCount>,
    mut config: ResMut<BoidsConfig>,
    mut simulation_state: ResMut<SimulationState>,
    container_query: Query<&GodotNodeHandle, With<BoidsContainer>>,
) {
    for handle in container_query.iter() {
        let mut handle_clone = handle.clone();
        if let Some(bevy_boids) = handle_clone.try_get::<BevyBoids>() {
            let boids_bind = bevy_boids.bind();
            
            // Update simulation state
            simulation_state.is_running = boids_bind.is_running;

            // Update world bounds
            let screen_size = boids_bind.screen_size;
            if screen_size.x > 0.0 && screen_size.y > 0.0 {
                config.world_bounds = Vec2::new(screen_size.x, screen_size.y);
            }

            // Update target boid count (no events, direct like GDScript)
            boid_count.target = boids_bind.target_boid_count;

            // Update current count back to Godot node
            let current_count = boid_count.current;
            drop(boids_bind); // Release the bind before getting mutable access
            let mut node = bevy_boids.upcast::<Node>();
            node.set_meta("current_count", &current_count.to_variant());
        }
    }
}

/// System that handles spawning and despawning boids (simplified like GDScript)
fn handle_boid_count(
    mut commands: Commands,
    mut boid_count: ResMut<BoidCount>,
    boids: Query<(Entity, &GodotNodeHandle), With<Boid>>,
    simulation_state: Res<SimulationState>,
    config: Res<BoidsConfig>,
    boid_scene: Res<BoidScene>,
) {
    // Count current boids
    let current_count = boids.iter().count() as i32;
    boid_count.current = current_count;

    // Skip spawning/despawning if simulation isn't running
    if !simulation_state.is_running {
        return;
    }

    let target_count = boid_count.target;
    
    // Spawn new boids if needed (max 50 per frame like GDScript)
    if current_count < target_count {
        let to_spawn = (target_count - current_count).min(50);
        spawn_boids(&mut commands, to_spawn, &config, &boid_scene);
    }
    // Despawn excess boids if needed (max 50 per frame like GDScript)
    else if current_count > target_count {
        let to_despawn = (current_count - target_count).min(50);
        despawn_boids(&mut commands, to_despawn, &boids);
    }
}

/// Helper function to spawn a batch of boids
fn spawn_boids(
    commands: &mut Commands,
    count: i32,
    config: &BoidsConfig,
    boid_scene: &BoidScene,
) {
    for _ in 0..count {
        // Create position and velocity
        let pos = Vector2::new(
            fastrand::f32() * config.world_bounds.x,
            fastrand::f32() * config.world_bounds.y,
        );

        // Match GDScript initial velocity exactly
        let velocity = Vector2::new(
            (fastrand::f32() - 0.5) * 200.0,
            (fastrand::f32() - 0.5) * 200.0,
        );

        // Create a transform using Godot's Transform2D
        let godot_transform = godot::prelude::Transform2D::IDENTITY.translated(pos);
        let transform = Transform2D::from(godot_transform);

        commands
            .spawn_empty()
            .insert(GodotScene::from_handle(boid_scene.0.clone()))
            .insert((
                Boid,
                Velocity(velocity),
                SteeringForces::default(),
                GridCell::default(),
                transform,
            ));
    }

    godot_print!("Spawned {} boids", count);
}

/// Helper function to despawn a batch of boids
fn despawn_boids(
    commands: &mut Commands,
    count: i32,
    boids: &Query<(Entity, &GodotNodeHandle), With<Boid>>,
) {
    // Get entities to despawn
    let entities_to_despawn: Vec<(Entity, GodotNodeHandle)> = boids
        .iter()
        .take(count as usize)
        .map(|(entity, handle)| (entity, handle.clone()))
        .collect();

    // Despawn each entity and free the Godot node
    for (entity, handle) in entities_to_despawn {
        let mut handle_clone = handle.clone();
        if let Some(mut node) = handle_clone.try_get::<Node>() {
            node.queue_free();
        }
        commands.entity(entity).despawn();
    }

    godot_print!("Despawned {} boids", count);
}

/// Update simulation state and manage cleanup on stop
fn update_simulation_state(
    simulation_state: Res<SimulationState>,
    mut commands: Commands,
    boids: Query<(Entity, &GodotNodeHandle), With<Boid>>,
) {
    // If simulation was just stopped, clean up all boids
    if !simulation_state.is_running && boids.iter().count() > 0 {
        godot_print!("Cleaning up all boids");

        // Queue all Godot nodes for deletion
        for (entity, handle) in boids.iter() {
            let mut handle_clone = handle.clone();
            if let Some(mut node) = handle_clone.try_get::<Node>() {
                node.queue_free();
            }
            commands.entity(entity).despawn();
        }
    }
}

/// Build spatial grid for efficient neighbor queries
fn build_spatial_grid(
    mut spatial_grid: ResMut<SpatialGrid>,
    mut boids: Query<(Entity, &Transform2D, &mut GridCell), With<Boid>>,
) {
    spatial_grid.grid.clear();

    for (entity, transform, mut grid_cell) in boids.iter_mut() {
        let position = transform.as_bevy().translation.truncate();
        let cell_x = (position.x / spatial_grid.cell_size) as i32;
        let cell_y = (position.y / spatial_grid.cell_size) as i32;

        grid_cell.x = cell_x;
        grid_cell.y = cell_y;

        spatial_grid
            .grid
            .entry((cell_x, cell_y))
            .or_insert_with(Vec::new)
            .push(entity);
    }
}

/// Get nearby boids using spatial grid - optimized to reduce allocations
fn get_nearby_boids_optimized(
    boid_pos: Vec2,
    boid_entity: Entity,
    grid_cell: &GridCell,
    perception_radius: f32,
    spatial_grid: &SpatialGrid,
    all_boids: &Query<(&Transform2D, &Velocity), With<Boid>>,
    nearby_buffer: &mut Vec<(Entity, Vec2, Vector2)>,
) {
    nearby_buffer.clear();
    let cell_range = ((perception_radius / spatial_grid.cell_size).ceil() as i32).max(1);
    let perception_radius_sq = perception_radius * perception_radius;

    for dx in -cell_range..=cell_range {
        for dy in -cell_range..=cell_range {
            let check_cell = (grid_cell.x + dx, grid_cell.y + dy);
            if let Some(entities) = spatial_grid.grid.get(&check_cell) {
                for &neighbor_entity in entities {
                    if neighbor_entity != boid_entity {
                        if let Ok((neighbor_transform, neighbor_velocity)) =
                            all_boids.get(neighbor_entity)
                        {
                            let neighbor_pos = neighbor_transform.as_bevy().translation.truncate();
                            let dist_sq = boid_pos.distance_squared(neighbor_pos);
                            if dist_sq < perception_radius_sq {
                                nearby_buffer.push((
                                    neighbor_entity,
                                    neighbor_pos,
                                    neighbor_velocity.0,
                                ));
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Calculate all forces in a single pass - much more efficient than 4 separate systems
fn calculate_all_forces(
    spatial_grid: Res<SpatialGrid>,
    config: Res<BoidsConfig>,
    boids: Query<(Entity, &Transform2D, &Velocity, &GridCell), With<Boid>>,
    all_boids: Query<(&Transform2D, &Velocity), With<Boid>>,
    mut forces: Query<&mut SteeringForces>,
) {
    let margin = 100.0;
    // Reuse buffer to avoid allocations
    let mut nearby_buffer = Vec::with_capacity(50); // Pre-allocate reasonable capacity

    for (entity, transform, velocity, grid_cell) in boids.iter() {
        let position = transform.as_bevy().translation.truncate();
        
        // Get neighbors once, use for all force calculations (no allocation)
        get_nearby_boids_optimized(
            position,
            entity,
            grid_cell,
            config.perception_radius,
            &spatial_grid,
            &all_boids,
            &mut nearby_buffer,
        );

        // Calculate all forces in one pass
        let mut separation = Vector2::ZERO;
        let mut separation_count = 0;
        let mut avg_vel = Vector2::ZERO;
        let mut center_of_mass = Vec2::ZERO;
        let neighbor_count = nearby_buffer.len();

        for (_, neighbor_pos, neighbor_vel) in &nearby_buffer {
            let distance = position.distance(*neighbor_pos);
            
            // Separation (only for close neighbors)
            if distance > 0.0 && distance < config.separation_radius {
                let diff = (position - *neighbor_pos) / distance;
                separation += Vector2::new(diff.x, diff.y);
                separation_count += 1;
            }
            
            // Alignment and cohesion (all neighbors)
            avg_vel += *neighbor_vel;
            center_of_mass += *neighbor_pos;
        }

        // Calculate separation force
        let separation_force = if separation_count > 0 {
            separation = separation / separation_count as f32;
            if separation.length() > 0.0 {
                separation = separation.normalized() * config.max_speed - velocity.0;
                limit_vector(separation, config.max_force)
            } else {
                Vector2::ZERO
            }
        } else {
            Vector2::ZERO
        };

        // Calculate alignment force
        let alignment_force = if neighbor_count > 0 {
            avg_vel = avg_vel / neighbor_count as f32;
            if avg_vel.length() > 0.0 {
                let desired = avg_vel.normalized() * config.max_speed;
                limit_vector(desired - velocity.0, config.max_force)
            } else {
                Vector2::ZERO
            }
        } else {
            Vector2::ZERO
        };

        // Calculate cohesion force
        let cohesion_force = if neighbor_count > 0 {
            center_of_mass /= neighbor_count as f32;
            let desired = (center_of_mass - position).normalize_or_zero() * config.max_speed;
            limit_vector(
                Vector2::new(desired.x, desired.y) - velocity.0,
                config.max_force,
            )
        } else {
            Vector2::ZERO
        };

        // Calculate boundary force
        let mut boundary_force = Vector2::ZERO;
        if position.x < margin {
            boundary_force.x += margin - position.x;
        } else if position.x > config.world_bounds.x - margin {
            boundary_force.x -= position.x - (config.world_bounds.x - margin);
        }

        if position.y < margin {
            boundary_force.y += margin - position.y;
        } else if position.y > config.world_bounds.y - margin {
            boundary_force.y -= position.y - (config.world_bounds.y - margin);
        }

        if boundary_force.length() > 0.0 {
            boundary_force = boundary_force.normalized() * config.max_speed - velocity.0;
            boundary_force = limit_vector(boundary_force, config.max_force * 2.0);
        }

        // Apply all forces at once
        if let Ok(mut force) = forces.get_mut(entity) {
            force.separation = separation_force;
            force.alignment = alignment_force;
            force.cohesion = cohesion_force;
            force.boundary = boundary_force;
        }
    }
}

/// Apply all steering forces to velocity
fn apply_steering_forces(
    mut boids: Query<(&mut Velocity, &SteeringForces), With<Boid>>,
    config: Res<BoidsConfig>,
    time: Res<Time>,
    mut performance: ResMut<PerformanceTracker>,
) {
    performance.frame_count += 1;

    let mut boid_index = 0;
    for (mut velocity, forces) in boids.iter_mut() {
        // Debug logging (same as GDScript) - only for the first boid
        let debug_counter = performance.frame_count % 120;
        let should_debug = debug_counter == 0 && boid_index == 0;

        if should_debug {
            godot_print!("=== BEVY BOID DEBUG ===");
            godot_print!("Delta Time: {:.6}", time.delta_secs());
            godot_print!(
                "Velocity before: {} (length: {:.2})",
                velocity.0,
                velocity.0.length()
            );
        }

        // Combine all forces
        let total_force = forces.separation * config.separation_weight
            + forces.alignment * config.alignment_weight
            + forces.cohesion * config.cohesion_weight
            + forces.boundary * config.boundary_weight;

        let limited_force = limit_vector(total_force, config.max_force);

        if should_debug {
            godot_print!(
                "Force: {} (length: {:.2})",
                limited_force,
                limited_force.length()
            );
        }

        // Update velocity
        velocity.0 += limited_force * time.delta_secs();
        velocity.0 = limit_vector(velocity.0, config.max_speed);

        if should_debug {
            godot_print!(
                "Velocity after: {} (length: {:.2})",
                velocity.0,
                velocity.0.length()
            );
            godot_print!("Max Speed: {:.2}", config.max_speed);
            godot_print!("Max Force: {:.2}", config.max_force);
            godot_print!("===================");
        }
        
        boid_index += 1;
    }
}

/// Update boid transforms (position and rotation) based on velocity
fn update_boid_transforms(
    mut boids: Query<(&Velocity, &mut Transform2D), With<Boid>>,
    config: Res<BoidsConfig>,
    time: Res<Time>,
) {
    for (velocity, mut transform) in boids.iter_mut() {
        let mut godot_transform = transform.as_godot_mut();

        // Update position
        godot_transform.origin += velocity.0 * time.delta_secs();

        // Wrap around boundaries (toroidal world)
        godot_transform.origin.x = (godot_transform.origin.x + config.world_bounds.x)
            % config.world_bounds.x;
        godot_transform.origin.y = (godot_transform.origin.y + config.world_bounds.y)
            % config.world_bounds.y;

        // Handle negative wrapping
        if godot_transform.origin.x < 0.0 {
            godot_transform.origin.x += config.world_bounds.x;
        }
        if godot_transform.origin.y < 0.0 {
            godot_transform.origin.y += config.world_bounds.y;
        }

        // Skip rotation for now to avoid transform conflicts
        // TODO: Add rotation back once position updates are stable
    }
}

/// Log performance metrics
fn log_performance(
    mut performance: ResMut<PerformanceTracker>,
    time: Res<Time>,
    boid_count: Res<BoidCount>,
) {
    let current_time = time.elapsed_secs();
    if current_time - performance.last_log_time >= 1.0 {
        let fps = performance.frame_count as f32 / (current_time - performance.last_log_time);
        godot_print!(
            "🎮 Bevy Boids Performance: {} boids | FPS: {:.1}",
            boid_count.current,
            fps
        );
        performance.last_log_time = current_time;
        performance.frame_count = 0;
    }
}

/// Helper function to limit vector magnitude
fn limit_vector(vec: Vector2, max_length: f32) -> Vector2 {
    let length = vec.length();
    if length > max_length && length > 0.0 {
        vec * (max_length / length)
    } else {
        vec
    }
}