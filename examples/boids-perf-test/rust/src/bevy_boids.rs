use bevy::{
    ecs::{
        component::Component,
        system::{Commands, Query, Res, ResMut},
    },
    math::Vec2,
    prelude::*,
};
use bevy_spatial::{kdtree::KDTree2, AutomaticUpdate, SpatialAccess, SpatialStructure};

use godot::builtin::Color as GodotColor;
use godot::classes::Node as GodotNode;
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

/// Marker component for boids that need colorization
#[derive(Component)]
pub struct NeedsColorization;

/// Component storing boid velocity
#[derive(Component, Default)]
pub struct Velocity(pub Vector2);

#[derive(Component, Default)]
pub struct BoidForce(pub Vector2);

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
            max_speed: 50.0,
            max_force: 5.0,
            perception_radius: 150.0,
            separation_radius: 25.0,
            separation_weight: 1.1,
            alignment_weight: 2.5,
            cohesion_weight: 1.0,
            boundary_weight: 1.0,
        }
    }
}

/// Plugin for boids simulation
pub struct BoidsPlugin;

impl Plugin for BoidsPlugin {
    fn build(&self, app: &mut App) {
        if cfg!(debug_assertions) {
            warn!("Running a debug build, performance will be significantly worse than release");
        } else {
            info!("Running a release build");
        };

        app.add_plugins(
            AutomaticUpdate::<Boid>::new()
                .with_spatial_ds(SpatialStructure::KDTree2)
                .with_frequency(std::time::Duration::from_millis(16)), // Update every 16ms (roughly 60fps)
        )
        .init_resource::<BoidsConfig>()
        .init_resource::<SimulationState>()
        .init_resource::<BoidCount>()
        .add_systems(Startup, load_assets)
        // Game logic systems
        .add_systems(
            Update,
            (
                sync_container_params,
                handle_boid_count,
                stop_simulation,
                colorize_new_boids,
            )
                .chain(),
        )
        // Movement systems
        .add_systems(
            Update,
            (
                sync_transforms,
                boids_calculate_neighborhood_forces,
                boids_apply_forces,
            )
                .chain()
                .run_if(|state: Res<SimulationState>| state.is_running)
                .after(sync_container_params),
        );
    }
}

/// Load the boid scene asset
fn load_assets(mut commands: Commands, server: Res<AssetServer>) {
    let handle: Handle<GodotResource> = server.load("scenes/boid.tscn");
    commands.insert_resource(BoidScene(handle));
}

/// Synchronize parameters from the container to Bevy resources
#[godot_main_thread]
fn sync_container_params(
    mut boid_count: ResMut<BoidCount>,
    mut config: ResMut<BoidsConfig>,
    mut simulation_state: ResMut<SimulationState>,
    container_query: Query<&GodotNodeHandle, With<BoidsContainer>>,
) {
    for handle in container_query.iter() {
        let mut handle_clone = handle.clone();
        if let Some(mut bevy_boids) = handle_clone.try_get::<BevyBoids>() {
            let boids_bind = bevy_boids.bind();

            // Update simulation state
            simulation_state.is_running = boids_bind.is_running;

            // Update world bounds
            let screen_size = boids_bind.screen_size;
            if screen_size.x > 0.0 && screen_size.y > 0.0 {
                config.world_bounds = Vec2::new(screen_size.x, screen_size.y);
            }

            // Update target boid count
            boid_count.target = boids_bind.target_boid_count;

            // Update current count back to Godot node
            let current_count = boid_count.current;
            drop(boids_bind); // Release the bind before getting mutable access
            let mut bevy_boids_mut = bevy_boids.bind_mut();
            bevy_boids_mut.current_boid_count = current_count;
        }
    }
}

/// System that handles spawning and despawning boids
fn handle_boid_count(
    mut commands: Commands,
    mut boid_count: ResMut<BoidCount>,
    boids: Query<(Entity, &GodotNodeHandle), With<Boid>>,
    simulation_state: Res<SimulationState>,
    config: Res<BoidsConfig>,
    boid_scene: Res<BoidScene>,
) {
    // Skip spawning/despawning if simulation isn't running
    if !simulation_state.is_running {
        return;
    }

    // Count current boids
    let current_count = boids.iter().count() as i32;
    boid_count.current = current_count;

    let target_count = boid_count.target;

    // Spawn new boids if needed (max 50 per frame)
    if current_count < target_count {
        let to_spawn = (target_count - current_count).min(50);
        spawn_boids(&mut commands, to_spawn, &config, &boid_scene);
    }
    // Despawn excess boids if needed (max 50 per frame)
    else if current_count > target_count {
        let to_despawn = (current_count - target_count).min(50);
        despawn_boids(&mut commands, to_despawn, &boids);
    }
}

/// Helper function to spawn a batch of boids
fn spawn_boids(commands: &mut Commands, count: i32, config: &BoidsConfig, boid_scene: &BoidScene) {
    for _ in 0..count {
        // Create position and velocity
        let pos = Vector2::new(
            fastrand::f32() * config.world_bounds.x,
            fastrand::f32() * config.world_bounds.y,
        );

        let velocity = Vector2::new(
            (fastrand::f32() - 0.5) * 200.0,
            (fastrand::f32() - 0.5) * 200.0,
        );

        // Create a transform using Godot's Transform2D
        let godot_transform = godot::prelude::Transform2D::IDENTITY.translated(pos);
        let transform = Transform2D::from(godot_transform);

        let entity = commands
            .spawn_empty()
            .insert(GodotScene::from_handle(boid_scene.0.clone()))
            .insert((
                Boid,
                Velocity(velocity),
                transform,
                Transform::default(),
                BoidForce::default(),
            ))
            .id();

        // We'll set the color after the entity is spawned in the next frame
        // by using a marker component
        commands.entity(entity).insert(NeedsColorization);
    }
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
        if let Some(mut node) = handle_clone.try_get::<GodotNode>() {
            node.queue_free();
        }
        commands.entity(entity).despawn();
    }
}

/// Update simulation state and manage cleanup on stop
#[godot_main_thread]
fn stop_simulation(
    simulation_state: Res<SimulationState>,
    mut commands: Commands,
    boids: Query<(Entity, &GodotNodeHandle), With<Boid>>,
) {
    // If simulation was just stopped, clean up all boids
    if !simulation_state.is_running && boids.iter().count() > 0 {
        // Queue all Godot nodes for deletion
        for (entity, handle) in boids.iter() {
            let mut handle_clone = handle.clone();
            if let Some(mut node) = handle_clone.try_get::<GodotNode>() {
                node.queue_free();
            }
            commands.entity(entity).despawn();
        }
    }
}

/// Colorize newly spawned boids (matches GDScript behavior)
#[godot_main_thread]
fn colorize_new_boids(
    mut commands: Commands,
    new_boids: Query<(Entity, &GodotNodeHandle), With<NeedsColorization>>,
) {
    for (entity, handle) in new_boids.iter() {
        let mut handle_clone = handle.clone();

        // Generate random color (matching GDScript)
        let random_color =
            GodotColor::from_rgba(fastrand::f32(), fastrand::f32(), fastrand::f32(), 0.9);

        // Try different node structures (matching GDScript logic)
        if let Some(mut node) = handle_clone.try_get::<Node2D>() {
            // Check for Sprite child node
            if node.has_node("Sprite") {
                let mut sprite = node.get_node_as::<Node2D>("Sprite");
                sprite.set_modulate(random_color);
            }
            // Check for Triangle child node
            else if node.has_node("Triangle") {
                let mut triangle = node.get_node_as::<Node2D>("Triangle");
                triangle.set_modulate(random_color);
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

// system to copy transform
fn sync_transforms(mut query: Query<(&Transform2D, &mut Transform), With<Boid>>) {
    query
        .par_iter_mut()
        .for_each(|(encapsulated_transform, mut vanilla_transform)| {
            // for bevy_spatial's Kd tree to work properly, we need to have a Bevy Transform component
            // directly in entity (as opposed to our  Transform2D which encapsulates bevy and godot
            // transforms). so we replicate Transform2D's internal one to a location friendly for
            // bevy_spatial:
            *vanilla_transform = *encapsulated_transform.as_bevy()
        });
}
// system to calculate/store neighborhood forces
// NOTE: While this doesn't _need_ to be on the main thread, we see a
// significant performance impact (75 -> 53 fps drop) when not on main
#[godot_main_thread]
fn boids_calculate_neighborhood_forces(
    spatial_tree: Res<BoidTree>,
    all_boids: Query<(&Transform, &Velocity), With<Boid>>,
    mut pending_velocity_update_query: Query<
        (Entity, &Transform, &mut BoidForce, &Velocity),
        With<Boid>,
    >,
    config: Res<BoidsConfig>,
) {
    pending_velocity_update_query.iter_mut().for_each(
        |(entity, transform, mut boid_force, velocity)| {
            boid_force.0 = calculate_boid_force_optimized(
                entity,
                transform.translation.xy(),
                velocity.0,
                &spatial_tree,
                all_boids,
                &config,
            );
        },
    );
}

// system to apply forces
fn boids_apply_forces(
    mut boid_transform_query: Query<
        (Entity, &mut Transform2D, &mut Velocity, &BoidForce),
        With<Boid>,
    >,
    time: Res<Time>,
    config: Res<BoidsConfig>,
) {
    let delta = time.delta_secs();

    boid_transform_query
        .iter_mut()
        .for_each(|(_, mut transform, mut velocity, force)| {
            velocity.0 += force.0 * delta;

            // Clamp velocity to max speed only (match GDScript)
            if velocity.0.length() > config.max_speed {
                velocity.0 = velocity.0.normalized() * config.max_speed;
            }

            // Read current position from Transform2D
            let current_pos =
                Vec2::new(transform.as_godot().origin.x, transform.as_godot().origin.y);

            // Calculate new position
            let new_pos = current_pos + Vec2::new(velocity.0.x, velocity.0.y) * delta;
            let bounded_pos = apply_boundary_constraints(new_pos, &config);

            // Write new position to Transform2D
            let mut godot_transform = *transform.as_godot();
            godot_transform.origin = Vector2::new(bounded_pos.x, bounded_pos.y);
            *transform = Transform2D::from(godot_transform);
        });
}

/// Optimized force calculation using k_nearest_neighbour
fn calculate_boid_force_optimized(
    entity: Entity,
    pos: Vec2,
    velocity: Vector2,
    spatial_tree: &BoidTree,
    all_boids: Query<(&Transform, &Velocity), With<Boid>>,
    config: &BoidsConfig,
) -> Vector2 {
    // Use k_nearest_neighbour with a reasonable cap (faster than within_distance)
    const NEIGHBOR_CAP: usize = 10;
    let nearby_entities = spatial_tree.k_nearest_neighbour(pos, NEIGHBOR_CAP);

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
            if let Ok((_, neighbor_velocity)) = all_boids.get(neighbor_entity) {
                // Separation (avoid crowding neighbors)
                if dist_sq < separation_radius_sq && dist_sq > 0.0 {
                    let distance = dist_sq.sqrt();
                    let normalized_diff = diff.normalize();
                    separation += Vector2::new(normalized_diff.x, normalized_diff.y) / distance;
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
        separation =
            (separation / separation_count as f32).normalized() * config.max_speed - velocity;
        let separation_force = if separation.length() > config.max_force {
            separation.normalized() * config.max_force
        } else {
            separation
        };
        total_force += separation_force * config.separation_weight;
    }

    // Apply alignment
    if neighbor_count > 0 {
        avg_vel = (avg_vel / neighbor_count as f32).normalized() * config.max_speed;
        let alignment = avg_vel - velocity;
        let alignment_force = if alignment.length() > config.max_force {
            alignment.normalized() * config.max_force
        } else {
            alignment
        };
        total_force += alignment_force * config.alignment_weight;

        // Apply cohesion
        center_of_mass /= neighbor_count as f32;
        let desired = (center_of_mass - pos).normalize() * config.max_speed;
        let cohesion = Vector2::new(desired.x, desired.y) - velocity;
        let cohesion_force = if cohesion.length() > config.max_force {
            cohesion.normalized() * config.max_force
        } else {
            cohesion
        };
        total_force += cohesion_force * config.cohesion_weight;
    }

    // Apply boundary avoidance
    let boundary = calculate_boundary_avoidance(pos, velocity, config);
    total_force += boundary * config.boundary_weight;

    // Limit total force
    if total_force.length() > config.max_force {
        total_force = total_force.normalized() * config.max_force;
    }

    total_force
}

/// Calculate boundary avoidance force (matches GDScript implementation)
fn calculate_boundary_avoidance(pos: Vec2, velocity: Vector2, config: &BoidsConfig) -> Vector2 {
    let mut steer = Vector2::ZERO;
    let margin = 100.0;

    // Calculate boundary forces (matching GDScript logic)
    if pos.x < margin {
        steer.x += margin - pos.x;
    } else if pos.x > config.world_bounds.x - margin {
        steer.x -= pos.x - (config.world_bounds.x - margin);
    }

    if pos.y < margin {
        steer.y += margin - pos.y;
    } else if pos.y > config.world_bounds.y - margin {
        steer.y -= pos.y - (config.world_bounds.y - margin);
    }

    if steer.length_squared() > 0.0 {
        steer = steer.normalized() * config.max_speed - velocity;
        let max_boundary_force = config.max_force * 2.0; // Double strength like GDScript
        if steer.length() > max_boundary_force {
            steer = steer.normalized() * max_boundary_force;
        }
        return steer;
    }

    Vector2::ZERO
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
        },
    )
}
