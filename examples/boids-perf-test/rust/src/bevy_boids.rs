use std::collections::HashMap;
use std::time::Duration;

use bevy::{
    ecs::{
        component::Component,
        schedule::SystemSet,
        system::{Commands, Query, Res, ResMut, ParamSet},
    },
    math::Vec2,
    prelude::*,
    tasks::ComputeTaskPool,
};
use bevy_spatial::{AutomaticUpdate, SpatialStructure, SpatialAccess, kdtree::KDTree2};
use fastrand;

use godot::prelude::*;
use godot_bevy::plugins::core::Transform2D;
use godot_bevy::prelude::*;

use crate::container::{BevyBoids, BoidsContainer};

// Type alias for our spatial tree
type BoidTree = KDTree2<Boid>;

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


/// Component for individual boid entities - also used for spatial tracking
#[derive(Component, Default)]
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

/// Optimized Structure of Arrays layout with bevy_spatial integration
#[derive(Resource, Default)]
pub struct OptimizedBoidData {
    pub entities: Vec<Entity>,
    pub positions: Vec<Vec2>,
    pub velocities: Vec<Vector2>,
    pub forces: Vec<Vector2>,
    pub grid_cells: Vec<GridCell>,
    /// Map from entity to index for fast lookups
    pub entity_to_index: HashMap<Entity, usize>,
    /// Spatial partitions for parallel processing - each partition contains indices into the main arrays
    pub spatial_partitions: Vec<Vec<usize>>,
    /// Cached Godot node handles to avoid repeated queries
    pub cached_handles: Vec<GodotNodeHandle>,
    /// Tracks if this is the first frame (need to read initial positions)
    pub is_initialized: bool,
}

/// Resource for performance tracking with detailed timing
#[derive(Resource)]
pub struct PerformanceTracker {
    pub frame_count: u32,
    pub last_log_time: f32,
    pub timing_data: TimingData,
}

/// Detailed timing metrics for performance analysis
#[derive(Default)]
pub struct TimingData {
    pub sync_from_ecs_us: u64,
    pub force_calculation_us: u64,
    pub physics_update_us: u64,
    pub sync_to_ecs_us: u64,
    pub total_frame_us: u64,
    pub spatial_partitioning_us: u64,
    pub neighbor_queries_us: u64,
    pub force_computation_us: u64,
    pub parallel_overhead_us: u64,
}

/// Resource to control sync frequency and batching optimization
#[derive(Resource)]
pub struct SyncConfig {
    pub visual_sync_every_n_frames: u32,
    pub current_visual_frame: u32,
    pub batch_size: usize,
    pub use_cached_positions: bool,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            visual_sync_every_n_frames: 2, // Sync every 2 frames to reduce overhead
            current_visual_frame: 0,
            batch_size: 50, // Process boids in batches
            use_cached_positions: true, // Use cached positions instead of reading from Godot
        }
    }
}

impl Default for PerformanceTracker {
    fn default() -> Self {
        Self {
            frame_count: 0,
            last_log_time: 0.0,
            timing_data: TimingData::default(),
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
        app.add_plugins(
                AutomaticUpdate::<Boid>::new()
                    .with_spatial_ds(SpatialStructure::KDTree2)
                    .with_frequency(Duration::from_millis(16)), // Update every 16ms (roughly 60fps)
            )
            .init_resource::<BoidsConfig>()
            .init_resource::<SimulationState>()
            .init_resource::<BoidCount>()
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
                boids_update_with_spatial_tree // Use proper Transform2D sync
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

/// Log performance metrics with detailed ECS overhead analysis
fn log_performance(
    mut performance: ResMut<PerformanceTracker>,
    time: Res<Time>,
    _boid_count: Res<BoidCount>,
    boids: Query<&Transform2D, With<Boid>>,
) {
    let current_time = time.elapsed_secs();
    if current_time - performance.last_log_time >= 1.0 {
        let fps = performance.frame_count as f32 / (current_time - performance.last_log_time);
        let actual_boid_count = boids.iter().count();
        
        godot_print!(
            "🎮 Bevy Boids: {} boids | FPS: {:.1} | bevy_spatial + proper Transform2D sync",
            actual_boid_count,
            fps
        );
        
        // Show detailed timing breakdown
        let timing = &performance.timing_data;
        if timing.total_frame_us > 0 {
            let total_ms = timing.total_frame_us as f32 / 1000.0;
            let data_collection_ms = timing.sync_from_ecs_us as f32 / 1000.0;
            let force_calc_ms = timing.force_calculation_us as f32 / 1000.0;
            let velocity_ms = timing.physics_update_us as f32 / 1000.0;
            let transform_sync_ms = timing.sync_to_ecs_us as f32 / 1000.0;
            let transform_read_ms = timing.spatial_partitioning_us as f32 / 1000.0;
            let transform_write_ms = timing.neighbor_queries_us as f32 / 1000.0;
            
            godot_print!(
                "   📊 TIMING: Total: {:.2}ms | Data: {:.2}ms | Forces: {:.2}ms | Velocity: {:.2}ms | Transform: {:.2}ms",
                total_ms, data_collection_ms, force_calc_ms, velocity_ms, transform_sync_ms
            );
            godot_print!(
                "   📊 DETAIL: Transform Read: {:.2}ms | Transform Write: {:.2}ms",
                transform_read_ms, transform_write_ms
            );
            
            // Show percentages
            if total_ms > 0.0 {
                let data_pct = (data_collection_ms / total_ms) * 100.0;
                let force_pct = (force_calc_ms / total_ms) * 100.0;
                let velocity_pct = (velocity_ms / total_ms) * 100.0;
                let transform_pct = (transform_sync_ms / total_ms) * 100.0;
                
                godot_print!(
                    "   📊 PERCENT: Data: {:.1}% | Forces: {:.1}% | Velocity: {:.1}% | Transform: {:.1}%",
                    data_pct, force_pct, velocity_pct, transform_pct
                );
            }
        }
        
        performance.last_log_time = current_time;
        performance.frame_count = 0;
    }
}

/// Optimized sync with caching - only reads from Godot on first frame, then uses cached data
fn sync_boid_data_from_ecs(
    mut optimized_data: ResMut<OptimizedBoidData>,
    boids: Query<(Entity, &GodotNodeHandle, &Velocity), With<Boid>>,
    mut performance: ResMut<PerformanceTracker>,
    _sync_config: Res<SyncConfig>,
) {
    let start_time = std::time::Instant::now();
    
    // If not initialized or entity count changed, rebuild from scratch
    let current_count = boids.iter().count();
    let needs_rebuild = !optimized_data.is_initialized || 
                       optimized_data.entities.len() != current_count;
    
    if needs_rebuild {
        // Clear and rebuild the optimized data structure
        optimized_data.entities.clear();
        optimized_data.positions.clear();
        optimized_data.velocities.clear();
        optimized_data.forces.clear();
        optimized_data.grid_cells.clear();
        optimized_data.entity_to_index.clear();
        optimized_data.cached_handles.clear();

        // Collect all boid data into arrays
        for (entity, handle, velocity) in boids.iter() {
            let index = optimized_data.entities.len();
            
            // Read initial position from Godot node (only on first frame or rebuild)
            let mut handle_clone = handle.clone();
            let position = if let Some(node) = handle_clone.try_get::<Node2D>() {
                let godot_pos = node.get_position();
                Vec2::new(godot_pos.x, godot_pos.y)
            } else {
                // Fallback to a random position if node access fails
                Vec2::new(
                    fastrand::f32() * 1920.0,
                    fastrand::f32() * 1080.0,
                )
            };
            
            optimized_data.entities.push(entity);
            optimized_data.positions.push(position);
            optimized_data.velocities.push(velocity.0);
            optimized_data.forces.push(Vector2::ZERO);
            optimized_data.grid_cells.push(GridCell::default());
            optimized_data.entity_to_index.insert(entity, index);
            optimized_data.cached_handles.push(handle.clone());
        }
        
        optimized_data.is_initialized = true;
    } else {
        // Just update velocities from ECS (much faster than reading positions from Godot)
        for (i, (_entity, _, velocity)) in boids.iter().enumerate() {
            if i < optimized_data.velocities.len() {
                optimized_data.velocities[i] = velocity.0;
            }
        }
    }
    
    performance.timing_data.sync_from_ecs_us = start_time.elapsed().as_micros() as u64;
}

/// Spatially-partitioned parallel boids update using ComputeTaskPool
fn optimized_boids_update(
    mut optimized_data: ResMut<OptimizedBoidData>,
    config: Res<BoidsConfig>,
    time: Res<Time>,
    mut performance: ResMut<PerformanceTracker>,
) {
    let frame_start = std::time::Instant::now();
    performance.frame_count += 1;
    let boid_count = optimized_data.entities.len();
    
    if boid_count == 0 {
        return;
    }

    // Ensure forces array is properly sized
    if optimized_data.forces.len() != boid_count {
        optimized_data.forces.resize(boid_count, Vector2::ZERO);
    }

    // Create spatial partitions for parallel processing (no manual tree building needed with bevy_spatial)
    let grid_start = std::time::Instant::now();
    create_spatial_partitions(&mut optimized_data, &config);
    performance.timing_data.spatial_partitioning_us = grid_start.elapsed().as_micros() as u64;

    let delta = time.delta_secs();
    let should_debug = performance.frame_count % 120 == 0;

    // Process each spatial partition in parallel using ComputeTaskPool
    let partition_count = optimized_data.spatial_partitions.len();
    if partition_count > 1 && boid_count > 100 {
        // Get the compute task pool for parallel processing
        let compute_pool = ComputeTaskPool::get();
        
        // Parallel force calculation, sequential physics update
        let force_start = std::time::Instant::now();
        parallel_force_calculation(&mut optimized_data, &config, compute_pool, &mut performance);
        performance.timing_data.force_calculation_us = force_start.elapsed().as_micros() as u64;
        
        let physics_start = std::time::Instant::now();
        sequential_physics_update(&mut optimized_data, &config, delta, should_debug);
        performance.timing_data.physics_update_us = physics_start.elapsed().as_micros() as u64;
    } else {
        // Single-threaded processing for small boid counts (overhead not worth it)
        let calc_start = std::time::Instant::now();
        sequential_boids_update(&mut optimized_data, &config, delta, should_debug);
        let calc_time = calc_start.elapsed().as_micros() as u64;
        performance.timing_data.force_calculation_us = calc_time / 2; // Rough split
        performance.timing_data.physics_update_us = calc_time / 2;
    }
    
    performance.timing_data.total_frame_us = frame_start.elapsed().as_micros() as u64;
}


/// Boids update system using bevy_spatial with Transform2D sync (original version)
fn boids_update_with_spatial_tree(
    mut queries: ParamSet<(
        Query<(Entity, &mut Transform2D, &mut Velocity), With<Boid>>,
        Query<(Entity, &Transform2D, &Velocity), With<Boid>>,
    )>,
    spatial_tree: Res<BoidTree>,
    config: Res<BoidsConfig>,
    time: Res<Time>,
    mut performance: ResMut<PerformanceTracker>,
) {
    let frame_start = std::time::Instant::now();
    performance.frame_count += 1;
    let delta = time.delta_secs();
    
    // Phase 1: Data collection from ECS
    let data_collection_start = std::time::Instant::now();
    let (boid_data, forces) = {
        let boid_query = queries.p1();
        let boid_count = boid_query.iter().count();
        if boid_count == 0 {
            return;
        }

        // Collect boid data for processing
        let boid_data: Vec<(Entity, Vec2, Vector2)> = boid_query.iter()
            .map(|(entity, transform, velocity)| {
                let pos = Vec2::new(transform.as_godot().origin.x, transform.as_godot().origin.y);
                (entity, pos, velocity.0)
            })
            .collect();
        
        let data_collection_time = data_collection_start.elapsed().as_micros() as u64;
        
        // Phase 2: Force calculation using bevy_spatial
        let force_calculation_start = std::time::Instant::now();
        let forces: Vec<(Entity, Vector2)> = boid_data.iter()
            .map(|&(entity, pos, velocity)| {
                let force = calculate_boid_force_optimized(
                    entity,
                    pos,
                    velocity,
                    &spatial_tree,
                    &boid_query,
                    &config,
                );
                (entity, force)
            })
            .collect();
            
        performance.timing_data.sync_from_ecs_us = data_collection_time;
        performance.timing_data.force_calculation_us = force_calculation_start.elapsed().as_micros() as u64;
        (boid_data, forces)
    };
    
    // Phase 3: Apply forces and update transforms
    let transform_update_start = std::time::Instant::now();
    let mut boids_mut = queries.p0();
    let mut transform_read_time = 0u64;
    let mut velocity_update_time = 0u64;
    let mut transform_write_time = 0u64;
    
    for (entity, force) in forces {
        if let Ok((_, mut transform, mut velocity)) = boids_mut.get_mut(entity) {
            // Phase 3a: Apply force to velocity
            let vel_start = std::time::Instant::now();
            velocity.0 += force * delta;
            
            // Clamp velocity  
            let speed = velocity.0.length();
            if speed < config.max_speed * 0.1 {
                velocity.0 = velocity.0.normalized() * config.max_speed * 0.1;
            } else if speed > config.max_speed {
                velocity.0 = velocity.0.normalized() * config.max_speed;
            }
            velocity_update_time += vel_start.elapsed().as_micros() as u64;
            
            // Phase 3b: Read current position from Transform2D
            let read_start = std::time::Instant::now();
            let current_pos = Vec2::new(transform.as_godot().origin.x, transform.as_godot().origin.y);
            transform_read_time += read_start.elapsed().as_micros() as u64;
            
            // Calculate new position
            let new_pos = current_pos + Vec2::new(velocity.0.x, velocity.0.y) * delta;
            let bounded_pos = apply_boundary_constraints(new_pos, &config);
            
            // Phase 3c: Write new position to Transform2D
            let write_start = std::time::Instant::now();
            let mut godot_transform = transform.as_godot().clone();
            godot_transform.origin = Vector2::new(bounded_pos.x, bounded_pos.y);
            *transform = Transform2D::from(godot_transform);
            transform_write_time += write_start.elapsed().as_micros() as u64;
        }
    }
    
    performance.timing_data.physics_update_us = velocity_update_time;
    performance.timing_data.sync_to_ecs_us = transform_read_time + transform_write_time;
    performance.timing_data.spatial_partitioning_us = transform_read_time;
    performance.timing_data.neighbor_queries_us = transform_write_time;
    performance.timing_data.total_frame_us = frame_start.elapsed().as_micros() as u64;
}

/// Optimized force calculation using k_nearest_neighbour
fn calculate_boid_force_optimized(
    entity: Entity,
    pos: Vec2,
    velocity: Vector2,
    spatial_tree: &BoidTree,
    boid_query: &Query<(Entity, &Transform2D, &Velocity), With<Boid>>,
    config: &BoidsConfig,
) -> Vector2 {
    // Use k_nearest_neighbour with a reasonable cap (faster than within_distance)
    const NEIGHBOR_CAP: usize = 50;
    let spatial_query_start = std::time::Instant::now();
    let nearby_entities = spatial_tree.k_nearest_neighbour(pos, NEIGHBOR_CAP);
    let _spatial_query_time = spatial_query_start.elapsed().as_micros();
    
    let perception_radius_sq = config.perception_radius * config.perception_radius;
    let separation_radius_sq = config.separation_radius * config.separation_radius;
    let mut separation = Vector2::ZERO;
    let mut separation_count = 0;
    let mut avg_vel = Vector2::ZERO;
    let mut center_of_mass = Vec2::ZERO;
    let mut neighbor_count = 0;
    
    // Process nearby entities
    for &(neighbor_pos, neighbor_entity_opt) in nearby_entities.iter() {
        if let Some(neighbor_entity) = neighbor_entity_opt {
            // Skip self
            if neighbor_entity == entity {
                continue;
            }
            
            let diff = pos - neighbor_pos;
            let dist_sq = diff.length_squared();
            
            // Skip if beyond perception radius
            if dist_sq > perception_radius_sq {
                continue;
            }
            
            // Direct query is faster than HashMap lookup for small neighbor counts
            if let Ok((_, _, neighbor_velocity)) = boid_query.get(neighbor_entity) {
                // Separation (avoid crowding neighbors)
                if dist_sq < separation_radius_sq && dist_sq > 0.0 {
                    let inv_dist = 1.0 / dist_sq.sqrt();
                    separation += Vector2::new(diff.x * inv_dist, diff.y * inv_dist);
                    separation_count += 1;
                }
                
                // Alignment and cohesion
                avg_vel += neighbor_velocity.0;
                center_of_mass += neighbor_pos;
                neighbor_count += 1;
            }
        }
    }
    
    let mut total_force = Vector2::ZERO;
    
    // Apply separation
    if separation_count > 0 {
        separation = separation.normalized() * config.max_force;
        total_force += separation * config.separation_weight;
    }
    
    // Apply alignment
    if neighbor_count > 0 {
        avg_vel /= neighbor_count as f32;
        let alignment = (avg_vel - velocity).normalized() * config.max_force;
        total_force += alignment * config.alignment_weight;
        
        // Apply cohesion
        center_of_mass /= neighbor_count as f32;
        let desired_direction = (center_of_mass - pos).normalize();
        let cohesion = Vector2::new(desired_direction.x, desired_direction.y) * config.max_force;
        total_force += cohesion * config.cohesion_weight;
    }
    
    // Limit total force
    if total_force.length() > config.max_force {
        total_force = total_force.normalized() * config.max_force;
    }
    
    total_force
}

/// Calculate steering forces for a boid using the automatic spatial tree
fn calculate_boid_force_with_spatial_tree(
    _entity: Entity,
    pos: Vec2,
    velocity: Vector2,
    spatial_tree: &BoidTree,
    boid_data: &[(Entity, Vec2, Vector2)],
    config: &BoidsConfig,
) -> Vector2 {
    // Find nearby entities using bevy_spatial
    let nearby_entities = spatial_tree.within_distance(pos, config.perception_radius);
    
    let separation_radius_sq = config.separation_radius * config.separation_radius;
    let mut separation = Vector2::ZERO;
    let mut separation_count = 0;
    let mut avg_vel = Vector2::ZERO;
    let mut center_of_mass = Vec2::ZERO;
    let mut neighbor_count = 0;
    
    // Process nearby entities
    for &(neighbor_pos, neighbor_entity_opt) in nearby_entities.iter() {
        if let Some(neighbor_entity) = neighbor_entity_opt {
            // Find the neighbor in our boid_data (this is inefficient but works for now)
            if let Some((_, _, neighbor_velocity)) = boid_data.iter().find(|(e, _, _)| *e == neighbor_entity) {
                let diff = pos - neighbor_pos;
                let dist_sq = diff.length_squared();
                
                // Skip self
                if dist_sq < 0.001 {
                    continue;
                }
                
                // Separation (avoid crowding neighbors)
                if dist_sq < separation_radius_sq && dist_sq > 0.0 {
                    let normalized_diff = diff.normalize();
                    separation += Vector2::new(normalized_diff.x, normalized_diff.y) / dist_sq.sqrt(); // Stronger when closer
                    separation_count += 1;
                }
                
                // Alignment and cohesion (within perception radius)
                if dist_sq < config.perception_radius * config.perception_radius {
                    avg_vel += *neighbor_velocity;
                    center_of_mass += neighbor_pos;
                    neighbor_count += 1;
                }
            }
        }
    }
    
    let mut total_force = Vector2::ZERO;
    
    // Apply separation
    if separation_count > 0 {
        separation = separation.normalized() * config.max_force;
        total_force += separation * config.separation_weight;
    }
    
    // Apply alignment
    if neighbor_count > 0 {
        avg_vel /= neighbor_count as f32;
        let alignment = (avg_vel - velocity).normalized() * config.max_force;
        total_force += alignment * config.alignment_weight;
        
        // Apply cohesion
        center_of_mass /= neighbor_count as f32;
        let desired_direction = (center_of_mass - pos).normalize();
        let cohesion = Vector2::new(desired_direction.x, desired_direction.y) * config.max_force;
        total_force += cohesion * config.cohesion_weight;
    }
    
    // Apply boundary forces
    let boundary_force = calculate_boundary_force(pos, config);
    total_force += boundary_force * config.boundary_weight;
    
    // Limit total force
    if total_force.length() > config.max_force {
        total_force = total_force.normalized() * config.max_force;
    }
    
    total_force
}

/// Apply boundary constraints with wraparound behavior
fn apply_boundary_constraints(pos: Vec2, config: &BoidsConfig) -> Vec2 {
    Vec2::new(
        if pos.x < 0.0 {
            config.world_bounds.x + pos.x
        } else if pos.x > config.world_bounds.x {
            pos.x - config.world_bounds.x
        } else {
            pos.x
        },
        if pos.y < 0.0 {
            config.world_bounds.y + pos.y
        } else if pos.y > config.world_bounds.y {
            pos.y - config.world_bounds.y
        } else {
            pos.y
        }
    )
}

/// Calculate boundary forces (minimal with wraparound - no strong edge avoidance needed)
fn calculate_boundary_force(_pos: Vec2, _config: &BoidsConfig) -> Vector2 {
    // With wraparound boundaries, we don't need strong boundary forces
    // The boids will teleport to the other side when they reach the edge
    Vector2::ZERO
}

/// Create spatial partitions based on world regions for parallel processing
fn create_spatial_partitions(optimized_data: &mut OptimizedBoidData, config: &BoidsConfig) {
    optimized_data.spatial_partitions.clear();
    
    // Determine number of partitions based on available threads and boid count
    let thread_count = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    let boid_count = optimized_data.entities.len();
    
    if boid_count < 100 {
        // Single partition for small counts
        optimized_data.spatial_partitions.push((0..boid_count).collect());
        return;
    }
    
    // Partition the world into spatial regions
    let partition_count = (thread_count).min(8); // Cap at 8 partitions
    let regions_per_side = (partition_count as f32).sqrt().ceil() as usize;
    let region_width = config.world_bounds.x / regions_per_side as f32;
    let region_height = config.world_bounds.y / regions_per_side as f32;
    
    // Initialize partitions
    for _ in 0..regions_per_side * regions_per_side {
        optimized_data.spatial_partitions.push(Vec::new());
    }
    
    // Assign boids to partitions based on their position
    for i in 0..boid_count {
        let pos = optimized_data.positions[i];
        let region_x = ((pos.x / region_width) as usize).min(regions_per_side - 1);
        let region_y = ((pos.y / region_height) as usize).min(regions_per_side - 1);
        let partition_idx = region_y * regions_per_side + region_x;
        
        optimized_data.spatial_partitions[partition_idx].push(i);
    }
    
    // Remove empty partitions
    optimized_data.spatial_partitions.retain(|partition| !partition.is_empty());
}

/// Parallel force calculation using ComputeTaskPool with detailed timing
fn parallel_force_calculation(
    optimized_data: &mut OptimizedBoidData,
    config: &BoidsConfig,
    compute_pool: &ComputeTaskPool,
    performance: &mut PerformanceTracker,
) {
    let parallel_start = std::time::Instant::now();
    
    // Copy data needed for parallel processing (immutable references)
    let positions = &optimized_data.positions;
    let velocities = &optimized_data.velocities;
    let grid_cells = &optimized_data.grid_cells;
    let partitions = &optimized_data.spatial_partitions;
    
    let neighbor_start = std::time::Instant::now();
    
    // Process partitions in parallel and collect results
    let force_results: Vec<(Vec<(usize, Vector2)>, u64, u64)> = compute_pool.scope(|scope| {
        partitions.iter().map(|partition| {
            let positions = positions;
            let velocities = velocities;
            let grid_cells = grid_cells;
            let config = config;
            
            scope.spawn(async move {
                let mut partition_forces = Vec::with_capacity(partition.len());
                let mut neighbor_query_time = 0u64;
                let mut force_calc_time = 0u64;
                
                for &boid_idx in partition {
                    let boid_start = std::time::Instant::now();
                    let force = calculate_boid_force_grid_based(
                        boid_idx,
                        positions,
                        velocities,
                        grid_cells,
                        config,
                    );
                    force_calc_time += boid_start.elapsed().as_micros() as u64;
                    partition_forces.push((boid_idx, force));
                }
                
                (partition_forces, neighbor_query_time, force_calc_time)
            })
        }).collect()
    });
    
    performance.timing_data.neighbor_queries_us = neighbor_start.elapsed().as_micros() as u64;
    
    // Apply the calculated forces back to the main forces array
    let apply_start = std::time::Instant::now();
    let mut total_force_calc_time = 0u64;
    for (partition_forces, _neighbor_time, force_time) in force_results {
        total_force_calc_time += force_time;
        for (boid_idx, force) in partition_forces {
            optimized_data.forces[boid_idx] = force;
        }
    }
    
    performance.timing_data.force_computation_us = total_force_calc_time;
    performance.timing_data.parallel_overhead_us = parallel_start.elapsed().as_micros() as u64 - total_force_calc_time;
}

/// Sequential physics update (position/velocity integration)
fn sequential_physics_update(
    optimized_data: &mut OptimizedBoidData,
    config: &BoidsConfig,
    delta: f32,
    should_debug: bool,
) {
    let boid_count = optimized_data.entities.len();
    
    for i in 0..boid_count {
        // Debug logging for first boid
        let debug_this_boid = should_debug && i == 0;
        
        if debug_this_boid {
            godot_print!("=== PARALLEL BEVY BOID DEBUG ===");
            godot_print!("Delta Time: {:.6}", delta);
            godot_print!("Velocity before: {} (length: {:.2})", optimized_data.velocities[i], optimized_data.velocities[i].length());
        }

        // Extract values to avoid borrowing conflicts
        let force = optimized_data.forces[i];
        let mut velocity = optimized_data.velocities[i];
        
        // Update velocity
        velocity += force * delta;
        velocity = limit_vector(velocity, config.max_speed);
        optimized_data.velocities[i] = velocity;

        if debug_this_boid {
            godot_print!("Force: {} (length: {:.2})", force, force.length());
            godot_print!("Velocity after: {} (length: {:.2})", velocity, velocity.length());
        }

        // Update position
        let velocity_bevy = Vec2::new(velocity.x, velocity.y);
        optimized_data.positions[i] += velocity_bevy * delta;

        // Wrap around boundaries
        optimized_data.positions[i].x = (optimized_data.positions[i].x + config.world_bounds.x) % config.world_bounds.x;
        optimized_data.positions[i].y = (optimized_data.positions[i].y + config.world_bounds.y) % config.world_bounds.y;

        if optimized_data.positions[i].x < 0.0 {
            optimized_data.positions[i].x += config.world_bounds.x;
        }
        if optimized_data.positions[i].y < 0.0 {
            optimized_data.positions[i].y += config.world_bounds.y;
        }

        if debug_this_boid {
            godot_print!("Position after: {}", optimized_data.positions[i]);
            godot_print!("==============================");
        }
    }
}

/// Fallback sequential processing for small boid counts
fn sequential_boids_update(
    optimized_data: &mut OptimizedBoidData,
    config: &BoidsConfig,
    delta: f32,
    should_debug: bool,
) {
    let boid_count = optimized_data.entities.len();
    
    // Calculate forces sequentially
    for i in 0..boid_count {
        optimized_data.forces[i] = calculate_boid_force_grid_based(
            i,
            &optimized_data.positions,
            &optimized_data.velocities,
            &optimized_data.grid_cells,
            config,
        );
    }
    
    // Update physics sequentially
    sequential_physics_update(optimized_data, config, delta, should_debug);
}

// NOTE: Commented out old function that uses manual spatial tree
/*
/// Calculate forces using bevy_spatial for ultra-fast neighbor queries
fn calculate_boid_force_optimized_arrays(
    boid_index: usize,
    positions: &[Vec2],
    velocities: &[Vector2],
    _grid_cells: &[GridCell],
    spatial_tree: &KDTree2<usize>,
    config: &BoidsConfig,
) -> Vector2 {
    let boid_pos = positions[boid_index];
    let boid_vel = velocities[boid_index];
    
    // Use bevy_spatial's optimized radius search - much faster than manual grid
    let neighbors = spatial_tree.within_distance(boid_pos, config.perception_radius);
    
    let separation_radius_sq = config.separation_radius * config.separation_radius;
    let mut separation = Vector2::ZERO;
    let mut separation_count = 0;
    let mut avg_vel = Vector2::ZERO;
    let mut center_of_mass = Vec2::ZERO;
    let mut neighbor_count = 0;
    
    // Process neighbors from bevy_spatial (already filtered by distance)
    for &neighbor_index in &neighbors {
        if neighbor_index != boid_index {
            let neighbor_pos = positions[neighbor_index];
            let neighbor_vel = velocities[neighbor_index];
            
            let diff_x = boid_pos.x - neighbor_pos.x;
            let diff_y = boid_pos.y - neighbor_pos.y;
            let dist_sq = diff_x * diff_x + diff_y * diff_y;
            
            // Separation calculation (for close neighbors only)
            if dist_sq > 0.0 && dist_sq < separation_radius_sq {
                let inv_distance = 1.0 / dist_sq.sqrt();
                separation += Vector2::new(diff_x * inv_distance, diff_y * inv_distance);
                separation_count += 1;
            }
            
            // Alignment and cohesion (all neighbors within perception)
            avg_vel += neighbor_vel;
            center_of_mass += neighbor_pos;
            neighbor_count += 1;
        }
    }

    // Early exit if no neighbors
    if neighbor_count == 0 {
        return calculate_boundary_force_array(boid_pos, boid_vel, config);
    }

    // Calculate separation force
    let separation_force = if separation_count > 0 {
        separation = separation / separation_count as f32;
        if separation.length_squared() > 0.0 {
            separation = separation.normalized() * config.max_speed - boid_vel;
            limit_vector(separation, config.max_force)
        } else {
            Vector2::ZERO
        }
    } else {
        Vector2::ZERO
    };

    // Calculate alignment force  
    avg_vel = avg_vel / neighbor_count as f32;
    let alignment_force = if avg_vel.length_squared() > 0.0 {
        let desired = avg_vel.normalized() * config.max_speed;
        limit_vector(desired - boid_vel, config.max_force)
    } else {
        Vector2::ZERO
    };

    // Calculate cohesion force
    center_of_mass /= neighbor_count as f32;
    let desired = (center_of_mass - boid_pos).normalize_or_zero() * config.max_speed;
    let cohesion_force = limit_vector(
        Vector2::new(desired.x, desired.y) - boid_vel,
        config.max_force,
    );

    // Calculate boundary force
    let boundary_force = calculate_boundary_force_array(boid_pos, boid_vel, config);

    // Combine all forces (pre-multiply weights to avoid repeated calculations)
    let total_force = separation_force * config.separation_weight
        + alignment_force * config.alignment_weight
        + cohesion_force * config.cohesion_weight
        + boundary_force * config.boundary_weight;

    limit_vector(total_force, config.max_force)
}
*/

/// Calculate boundary avoidance force using direct values
fn calculate_boundary_force_array(position: Vec2, velocity: Vector2, config: &BoidsConfig) -> Vector2 {
    let mut boundary_force = Vector2::ZERO;
    let margin = 100.0;

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
        boundary_force = boundary_force.normalized() * config.max_speed - velocity;
        limit_vector(boundary_force, config.max_force * 2.0)
    } else {
        Vector2::ZERO
    }
}

/// Grid-based force calculation for parallel processing (fallback when spatial tree isn't available)
fn calculate_boid_force_grid_based(
    boid_index: usize,
    positions: &[Vec2],
    velocities: &[Vector2],
    _grid_cells: &[GridCell],
    config: &BoidsConfig,
) -> Vector2 {
    let boid_pos = positions[boid_index];
    let boid_vel = velocities[boid_index];
    
    let perception_radius_sq = config.perception_radius * config.perception_radius;
    let separation_radius_sq = config.separation_radius * config.separation_radius;
    let mut separation = Vector2::ZERO;
    let mut separation_count = 0;
    let mut avg_vel = Vector2::ZERO;
    let mut center_of_mass = Vec2::ZERO;
    let mut neighbor_count = 0;
    
    // Check all other boids (brute force approach for parallel processing)
    for (i, &neighbor_pos) in positions.iter().enumerate() {
        if i != boid_index {
            let neighbor_vel = velocities[i];
            
            let diff_x = boid_pos.x - neighbor_pos.x;
            let diff_y = boid_pos.y - neighbor_pos.y;
            let dist_sq = diff_x * diff_x + diff_y * diff_y;
            
            // Skip if outside perception radius
            if dist_sq > perception_radius_sq {
                continue;
            }
            
            // Separation (avoid crowding neighbors)
            if dist_sq < separation_radius_sq && dist_sq > 0.0 {
                let distance = dist_sq.sqrt();
                let normalized_diff = Vector2::new(diff_x, diff_y) / distance;
                separation += normalized_diff / distance; // Stronger when closer
                separation_count += 1;
            }
            
            // Alignment and cohesion (within perception radius)
            avg_vel += neighbor_vel;
            center_of_mass += neighbor_pos;
            neighbor_count += 1;
        }
    }
    
    let mut total_force = Vector2::ZERO;
    
    // Apply separation
    if separation_count > 0 {
        separation = separation.normalized() * config.max_force;
        total_force += separation * config.separation_weight;
    }
    
    // Apply alignment
    if neighbor_count > 0 {
        avg_vel /= neighbor_count as f32;
        let alignment = (avg_vel - boid_vel).normalized() * config.max_force;
        total_force += alignment * config.alignment_weight;
        
        // Apply cohesion
        center_of_mass /= neighbor_count as f32;
        let desired_direction = (center_of_mass - boid_pos).normalize();
        let cohesion = Vector2::new(desired_direction.x, desired_direction.y) * config.max_force;
        total_force += cohesion * config.cohesion_weight;
    }
    
    // Apply boundary forces
    let boundary_force = calculate_boundary_force_simple(boid_pos, boid_vel, config);
    total_force += boundary_force * config.boundary_weight;
    
    // Limit total force
    if total_force.length() > config.max_force {
        total_force = total_force.normalized() * config.max_force;
    }
    
    total_force
}

/// Simple boundary force calculation (minimal with wraparound)
fn calculate_boundary_force_simple(_pos: Vec2, _velocity: Vector2, _config: &BoidsConfig) -> Vector2 {
    // With wraparound boundaries, we don't need boundary forces
    Vector2::ZERO
}

/// Optimized sync that attempts to minimize transform overhead
fn sync_boid_data_to_ecs_optimized(
    optimized_data: Res<OptimizedBoidData>,
    mut boids: Query<(&mut Transform2D, &mut Velocity, &GodotNodeHandle), With<Boid>>,
    mut performance: ResMut<PerformanceTracker>,
    mut sync_config: ResMut<SyncConfig>,
) {
    let start_time = std::time::Instant::now();
    
    // Update frame counter for visual sync frequency control
    sync_config.current_visual_frame += 1;
    let should_sync_visual = sync_config.current_visual_frame >= sync_config.visual_sync_every_n_frames;
    if should_sync_visual {
        sync_config.current_visual_frame = 0;
    }
    
    for (i, &entity) in optimized_data.entities.iter().enumerate() {
        if let Ok((mut transform, mut velocity, handle)) = boids.get_mut(entity) {
            // Always update velocity (needed for logic)
            velocity.0 = optimized_data.velocities[i];
            
            // Only update visual position at reduced frequency if configured
            if should_sync_visual {
                // Option 1: Try to update Godot node directly to bypass some transform overhead
                let mut handle_clone = handle.clone();
                if let Some(mut node) = handle_clone.try_get::<Node2D>() {
                    // Direct position update on Godot node
                    let new_pos = Vector2::new(
                        optimized_data.positions[i].x,
                        optimized_data.positions[i].y,
                    );
                    node.set_position(new_pos);
                    
                    // Also keep the ECS transform in sync for consistency
                    let mut godot_transform = transform.as_godot_mut();
                    godot_transform.origin = new_pos;
                } else {
                    // Fallback: Update through ECS transform only
                    let mut godot_transform = transform.as_godot_mut();
                    godot_transform.origin = Vector2::new(
                        optimized_data.positions[i].x,
                        optimized_data.positions[i].y,
                    );
                }
            } else {
                // Still need to keep ECS transform somewhat in sync for logic
                let mut godot_transform = transform.as_godot_mut();
                godot_transform.origin = Vector2::new(
                    optimized_data.positions[i].x,
                    optimized_data.positions[i].y,
                );
            }
        }
    }
    
    performance.timing_data.sync_to_ecs_us = start_time.elapsed().as_micros() as u64;
}

/// Optimized Godot sync with batching and reduced frequency
fn sync_boid_data_to_godot_only(
    optimized_data: Res<OptimizedBoidData>,
    _boids: Query<&GodotNodeHandle, With<Boid>>,
    mut performance: ResMut<PerformanceTracker>,
    mut sync_config: ResMut<SyncConfig>,
) {
    let start_time = std::time::Instant::now();
    
    // Update frame counter and check if we should sync visuals
    sync_config.current_visual_frame += 1;
    let should_sync_visual = sync_config.current_visual_frame >= sync_config.visual_sync_every_n_frames;
    if should_sync_visual {
        sync_config.current_visual_frame = 0;
    }
    
    // Only perform visual updates at reduced frequency
    if should_sync_visual && optimized_data.is_initialized {
        let batch_size = sync_config.batch_size;
        let boid_count = optimized_data.cached_handles.len();
        
        // Process boids in batches to reduce overhead
        for batch_start in (0..boid_count).step_by(batch_size) {
            let batch_end = (batch_start + batch_size).min(boid_count);
            
            // Process this batch
            for i in batch_start..batch_end {
                if i < optimized_data.cached_handles.len() {
                    // Use cached handle instead of querying ECS
                    let mut handle_clone = optimized_data.cached_handles[i].clone();
                    if let Some(mut node) = handle_clone.try_get::<Node2D>() {
                        // Pre-compute position once
                        let new_pos = Vector2::new(
                            optimized_data.positions[i].x,
                            optimized_data.positions[i].y,
                        );
                        
                        // Single position update call
                        node.set_position(new_pos);
                        
                        // Optional rotation update (less frequent)
                        if sync_config.current_visual_frame == 0 { // Only every N frames
                            let velocity = optimized_data.velocities[i];
                            if velocity.length() > 0.1 {
                                node.set_rotation(velocity.angle());
                            }
                        }
                    }
                }
            }
        }
    }
    
    performance.timing_data.sync_to_ecs_us = start_time.elapsed().as_micros() as u64;
}

/// Sync optimized data back to ECS components (original version for comparison)
fn sync_boid_data_to_ecs(
    optimized_data: Res<OptimizedBoidData>,
    mut boids: Query<(&mut Transform2D, &mut Velocity), With<Boid>>,
    mut performance: ResMut<PerformanceTracker>,
) {
    let start_time = std::time::Instant::now();
    
    for (i, &entity) in optimized_data.entities.iter().enumerate() {
        if let Ok((mut transform, mut velocity)) = boids.get_mut(entity) {
            // Update velocity
            velocity.0 = optimized_data.velocities[i];
            
            // Update transform position (keep the ECS transform in sync)
            let mut godot_transform = transform.as_godot_mut();
            godot_transform.origin = Vector2::new(
                optimized_data.positions[i].x,
                optimized_data.positions[i].y,
            );
        }
    }
    
    performance.timing_data.sync_to_ecs_us = start_time.elapsed().as_micros() as u64;
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