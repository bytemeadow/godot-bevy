extends Node2D

## Optimized pure Godot boids implementation for performance comparison
## This uses GDScript and Godot's built-in systems for boids simulation

class_name GodotBoids

# Simulation parameters
var world_bounds: Vector2 = Vector2(1920, 1080)
var max_speed: float = 150.0
var max_force: float = 5.0
var perception_radius: float = 50.0
var separation_radius: float = 25.0

# Behavior weights
var separation_weight: float = 2.0
var alignment_weight: float = 1.0
var cohesion_weight: float = 1.0
var boundary_weight: float = 3.0

# Benchmark state
var target_boid_count: int = 0
var is_running: bool = false

# Optimized data structures - store everything in arrays for cache efficiency
var boid_positions: PackedVector2Array = []
var boid_velocities: PackedVector2Array = []
var boid_nodes: Array[Node2D] = []

# Preloaded boid scene
var boid_scene: PackedScene = preload("res://scenes/boid.tscn")

# Spatial grid for optimization
var grid_cell_size: float = 75.0
var spatial_grid: Dictionary = {}

# Performance tracking
var frame_count: int = 0
var last_performance_log: float = 0.0

# Timing data removed for performance

# Pre-allocated arrays to avoid garbage collection
var neighbor_positions: PackedVector2Array = []
var neighbor_velocities: PackedVector2Array = []
var forces: PackedVector2Array = []

func _ready():
	# Set world bounds to match viewport
	var viewport_size = get_viewport().get_visible_rect().size
	world_bounds = viewport_size
	
	# Godot boids initialized

func _process(delta):
	if not is_running:
		return
	
	# Update boid count to match target
	_update_boid_count()
	
	# Update boids simulation
	_update_boids(delta)
	
	# Performance logging
	frame_count += 1
	var current_time = Time.get_ticks_msec() / 1000.0
	if current_time - last_performance_log >= 1.0:
		_log_performance()
		last_performance_log = current_time

func start_benchmark(boid_count: int):
	target_boid_count = boid_count
	is_running = true
	frame_count = 0
	last_performance_log = Time.get_ticks_msec() / 1000.0
	# Starting Godot boids benchmark

func stop_benchmark():
	is_running = false
	_clear_all_boids()
	# Stopped Godot boids benchmark

func set_target_boid_count(count: int):
	target_boid_count = count

func get_boid_count() -> int:
	return boid_nodes.size()

func _update_boid_count():
	var current_count = boid_nodes.size()
	
	# Spawn boids if we need more
	if current_count < target_boid_count:
		var to_spawn = min(target_boid_count - current_count, 50) # Max 50 per frame
		for i in range(to_spawn):
			_spawn_boid()
	
	# Remove boids if we have too many
	elif current_count > target_boid_count:
		var to_remove = min(current_count - target_boid_count, 50) # Max 50 per frame
		for i in range(to_remove):
			_remove_boid()

func _spawn_boid():
	# Instantiate the boid scene
	var boid_instance = boid_scene.instantiate()
	
	# Random position and velocity
	var pos = Vector2(randf() * world_bounds.x, randf() * world_bounds.y)
	var vel = Vector2((randf() - 0.5) * 200.0, (randf() - 0.5) * 200.0)
	
	# Apply random color for visual variety
	if boid_instance.has_node("Sprite"):
		boid_instance.get_node("Sprite").modulate = Color(randf(), randf(), randf(), 0.9)
	elif boid_instance.has_node("Triangle"):
		boid_instance.get_node("Triangle").modulate = Color(randf(), randf(), randf(), 0.8)
	elif boid_instance is Sprite2D:
		boid_instance.modulate = Color(randf(), randf(), randf(), 0.9)
	
	boid_instance.position = pos
	add_child(boid_instance)
	
	# Store in optimized arrays
	boid_nodes.append(boid_instance)
	boid_positions.append(pos)
	boid_velocities.append(vel)

func _remove_boid():
	if boid_nodes.size() > 0:
		var boid = boid_nodes.pop_back()
		boid.queue_free()
		boid_positions.resize(boid_positions.size() - 1)
		boid_velocities.resize(boid_velocities.size() - 1)

func _clear_all_boids():
	for boid in boid_nodes:
		boid.queue_free()
	boid_nodes.clear()
	boid_positions.clear()
	boid_velocities.clear()

func _update_boids(delta: float):
	var boid_count = boid_nodes.size()
	if boid_count == 0:
		return
	
	# Resize forces array if needed
	if forces.size() != boid_count:
		forces.resize(boid_count)
	
	# Phase 1: Build spatial grid for efficient neighbor finding
	_build_spatial_grid()
	
	# Phase 2: Calculate forces for all boids using optimized approach
	for i in range(boid_count):
		forces[i] = _calculate_boid_force_optimized(i)
	
	# Phase 3: Apply forces and update positions
	for i in range(boid_count):
		_update_boid_physics_optimized(i, forces[i], delta)

func _build_spatial_grid():
	spatial_grid.clear()
	
	for i in range(boid_positions.size()):
		var pos = boid_positions[i]
		var cell = Vector2i(int(pos.x / grid_cell_size), int(pos.y / grid_cell_size))
		var key = "%d,%d" % [cell.x, cell.y]
		
		if not spatial_grid.has(key):
			spatial_grid[key] = []
		spatial_grid[key].append(i)

func _get_nearby_boids_optimized(boid_index: int) -> Array[int]:
	var pos = boid_positions[boid_index]
	var center_cell = Vector2i(int(pos.x / grid_cell_size), int(pos.y / grid_cell_size))
	var cell_range = int(ceil(perception_radius / grid_cell_size))
	var nearby: Array[int] = []
	
	for dx in range(-cell_range, cell_range + 1):
		for dy in range(-cell_range, cell_range + 1):
			var cell_key = "%d,%d" % [center_cell.x + dx, center_cell.y + dy]
			if spatial_grid.has(cell_key):
				var cell_boids = spatial_grid[cell_key]
				for neighbor_index in cell_boids:
					if neighbor_index != boid_index:
						var dist_sq = pos.distance_squared_to(boid_positions[neighbor_index])
						if dist_sq < perception_radius * perception_radius:
							nearby.append(neighbor_index)
	
	return nearby

func _calculate_boid_force_optimized(boid_index: int) -> Vector2:
	var pos = boid_positions[boid_index]
	var vel = boid_velocities[boid_index]
	var nearby_indices = _get_nearby_boids_optimized(boid_index)
	
	# Pre-allocate neighbor data arrays
	var neighbor_count = nearby_indices.size()
	if neighbor_count == 0:
		return _calculate_boundary_avoidance_optimized(pos, vel)
	
	# Calculate behavior forces using vectorized operations where possible
	var separation = _calculate_separation_optimized(pos, vel, nearby_indices)
	var alignment = _calculate_alignment_optimized(vel, nearby_indices)
	var cohesion = _calculate_cohesion_optimized(pos, vel, nearby_indices)
	var boundary = _calculate_boundary_avoidance_optimized(pos, vel)
	
	# Combine forces
	var total_force = (
		separation * separation_weight +
		alignment * alignment_weight +
		cohesion * cohesion_weight +
		boundary * boundary_weight
	)
	
	return total_force.limit_length(max_force)

func _calculate_separation_optimized(boid_pos: Vector2, boid_vel: Vector2, nearby_indices: Array[int]) -> Vector2:
	var steer = Vector2.ZERO
	var count = 0
	
	for neighbor_index in nearby_indices:
		var neighbor_pos = boid_positions[neighbor_index]
		var distance = boid_pos.distance_to(neighbor_pos)
		if distance > 0 and distance < separation_radius:
			var diff = (boid_pos - neighbor_pos).normalized() / distance
			steer += diff
			count += 1
	
	if count > 0:
		steer = (steer / count).normalized() * max_speed - boid_vel
		return steer.limit_length(max_force)
	
	return Vector2.ZERO

func _calculate_alignment_optimized(boid_vel: Vector2, nearby_indices: Array[int]) -> Vector2:
	var avg_vel = Vector2.ZERO
	var count = nearby_indices.size()
	
	if count == 0:
		return Vector2.ZERO
	
	for neighbor_index in nearby_indices:
		avg_vel += boid_velocities[neighbor_index]
	
	avg_vel = (avg_vel / count).normalized() * max_speed
	return (avg_vel - boid_vel).limit_length(max_force)

func _calculate_cohesion_optimized(boid_pos: Vector2, boid_vel: Vector2, nearby_indices: Array[int]) -> Vector2:
	var center_of_mass = Vector2.ZERO
	var count = nearby_indices.size()
	
	if count == 0:
		return Vector2.ZERO
	
	for neighbor_index in nearby_indices:
		center_of_mass += boid_positions[neighbor_index]
	
	center_of_mass /= count
	var desired = (center_of_mass - boid_pos).normalized() * max_speed
	return (desired - boid_vel).limit_length(max_force)

func _calculate_boundary_avoidance_optimized(position: Vector2, velocity: Vector2) -> Vector2:
	var steer = Vector2.ZERO
	var margin = 100.0
	
	# Calculate boundary forces
	if position.x < margin:
		steer.x += margin - position.x
	elif position.x > world_bounds.x - margin:
		steer.x -= position.x - (world_bounds.x - margin)
	
	if position.y < margin:
		steer.y += margin - position.y
	elif position.y > world_bounds.y - margin:
		steer.y -= position.y - (world_bounds.y - margin)
	
	if steer.length_squared() > 0:
		steer = steer.normalized() * max_speed - velocity
		return steer.limit_length(max_force * 2.0)
	
	return Vector2.ZERO

func _update_boid_physics_optimized(boid_index: int, force: Vector2, delta: float):
	var velocity = boid_velocities[boid_index]
	var position = boid_positions[boid_index]
	
	# Debug logging removed for performance
	
	# Update velocity
	velocity += force * delta
	velocity = velocity.limit_length(max_speed)
	
	# Debug output removed
	
	# Update position
	position += velocity * delta
	
	# Wrap around boundaries (toroidal world)
	position.x = fmod(position.x + world_bounds.x, world_bounds.x)
	position.y = fmod(position.y + world_bounds.y, world_bounds.y)
	
	# Store back to arrays
	boid_velocities[boid_index] = velocity
	boid_positions[boid_index] = position
	
	# Update visual node position and rotation
	var boid = boid_nodes[boid_index]
	boid.position = position
	if velocity.length_squared() > 0:
		boid.rotation = velocity.angle()
	
	# Debug output removed

func _log_performance():
	# Performance logging disabled for accurate benchmarking
	# var fps = Engine.get_frames_per_second()
	# print("🎮 Godot Boids: %d boids | FPS: %.1f" % [boid_nodes.size(), fps])
	pass

## Utility functions for external access

func get_simulation_parameters() -> Dictionary:
	return {
		"max_speed": max_speed,
		"max_force": max_force,
		"perception_radius": perception_radius,
		"separation_radius": separation_radius,
		"separation_weight": separation_weight,
		"alignment_weight": alignment_weight,
		"cohesion_weight": cohesion_weight,
		"boundary_weight": boundary_weight,
		"world_bounds": world_bounds
	}

func set_simulation_parameters(params: Dictionary):
	if params.has("max_speed"):
		max_speed = params.max_speed
	if params.has("max_force"):
		max_force = params.max_force
	if params.has("perception_radius"):
		perception_radius = params.perception_radius
	if params.has("separation_radius"):
		separation_radius = params.separation_radius
	if params.has("separation_weight"):
		separation_weight = params.separation_weight
	if params.has("alignment_weight"):
		alignment_weight = params.alignment_weight
	if params.has("cohesion_weight"):
		cohesion_weight = params.cohesion_weight
	if params.has("boundary_weight"):
		boundary_weight = params.boundary_weight
	if params.has("world_bounds"):
		world_bounds = params.world_bounds
