extends Node2D

## Pure Godot particle rain implementation for performance comparison
## This uses GDScript and Godot's built-in systems for particle simulation

class_name GodotBoids

# Simulation parameters
var world_bounds: Vector2 = Vector2(1920, 1080)
var gravity: float = 200.0        # pixels per second^2
var min_speed: float = 50.0       # minimum fall speed
var max_speed: float = 300.0      # maximum fall speed
var horizontal_drift: float = 50.0 # max horizontal drift

# Benchmark state
var target_particle_count: int = 0
var is_running: bool = false

# Optimized data structures - store everything in arrays for cache efficiency
var particle_positions: PackedVector2Array = []
var particle_velocities: PackedVector2Array = []
var particle_nodes: Array[Node2D] = []

# Preloaded particle scene
var particle_scene: PackedScene = preload("res://scenes/particle.tscn")

# Performance tracking
var frame_count: int = 0
var last_performance_log: float = 0.0

func _ready():
	# Set world bounds to match viewport
	var viewport_size = get_viewport().get_visible_rect().size
	world_bounds = viewport_size
	print("🌧️ Godot particles initialized with bounds: ", world_bounds)

func _process(delta):
	if not is_running:
		return

	# Update particle count to match target
	_update_particle_count()

	# Update particles simulation
	_update_particles(delta)

	# Performance logging
	frame_count += 1
	var current_time = Time.get_ticks_msec() / 1000.0
	if current_time - last_performance_log >= 1.0:
		_log_performance()
		last_performance_log = current_time

func start_benchmark(particle_count: int):
	target_particle_count = particle_count
	is_running = true
	frame_count = 0
	last_performance_log = Time.get_ticks_msec() / 1000.0
	print("🚀 Starting Godot particles benchmark with %d particles" % particle_count)

func stop_benchmark():
	is_running = false
	_clear_all_particles()
	print("⏹️ Stopped Godot particles benchmark")

func set_target_particle_count(count: int):
	target_particle_count = count

func get_particle_count() -> int:
	return particle_nodes.size()

# Compatibility methods for legacy interface
func get_boid_count() -> int:
	return get_particle_count()

func set_target_boid_count(count: int):
	set_target_particle_count(count)

func _update_particle_count():
	var current_count = particle_nodes.size()

	# Spawn particles if we need more
	if current_count < target_particle_count:
		var to_spawn = min(target_particle_count - current_count, 50) # Max 50 per frame
		for i in range(to_spawn):
			_spawn_particle()

	# Remove particles if we have too many
	elif current_count > target_particle_count:
		var to_remove = min(current_count - target_particle_count, 50) # Max 50 per frame
		for i in range(to_remove):
			_remove_particle()

func _spawn_particle():
	# Instantiate the particle scene
	var particle_instance = particle_scene.instantiate()

	# Random position at the top of the screen
	var pos = Vector2(randf() * world_bounds.x, -50.0) # Start above screen

	# Create downward velocity with some randomization
	var fall_speed = min_speed + randf() * (max_speed - min_speed)
	var horizontal_speed = (randf() - 0.5) * horizontal_drift
	var vel = Vector2(horizontal_speed, fall_speed)

	# Apply random color for visual variety
	var random_color = Color(randf(), randf(), randf(), 0.8)
	if particle_instance.has_node("Sprite"):
		particle_instance.get_node("Sprite").modulate = random_color
	elif particle_instance is Sprite2D:
		particle_instance.modulate = random_color
	else:
		particle_instance.modulate = random_color

	particle_instance.position = pos
	add_child(particle_instance)

	# Store in optimized arrays
	particle_nodes.append(particle_instance)
	particle_positions.append(pos)
	particle_velocities.append(vel)

func _remove_particle():
	if particle_nodes.size() > 0:
		var particle = particle_nodes.pop_back()
		particle.queue_free()
		particle_positions.resize(particle_positions.size() - 1)
		particle_velocities.resize(particle_velocities.size() - 1)

func _clear_all_particles():
	for particle in particle_nodes:
		particle.queue_free()
	particle_nodes.clear()
	particle_positions.clear()
	particle_velocities.clear()

func _update_particles(delta: float):
	var particle_count = particle_nodes.size()
	if particle_count == 0:
		return

	# Update all particles
	for i in range(particle_count):
		_update_particle_physics(i, delta)

func _update_particle_physics(particle_index: int, delta: float):
	var velocity = particle_velocities[particle_index]
	var position = particle_positions[particle_index]

	# Apply gravity to velocity
	velocity.y += gravity * delta

	# Clamp velocity to reasonable bounds
	if velocity.y > max_speed:
		velocity.y = max_speed

	# Update position
	position += velocity * delta

	# Wrap particles that fall off the bottom back to the top
	if position.y > world_bounds.y + 50.0:
		position.x = randf() * world_bounds.x  # Random x position
		position.y = -50.0  # Above the screen
		
		# Reset velocity for variety
		var fall_speed = min_speed + randf() * (max_speed - min_speed)
		var horizontal_speed = (randf() - 0.5) * horizontal_drift
		velocity = Vector2(horizontal_speed, fall_speed)

	# Store back to arrays
	particle_velocities[particle_index] = velocity
	particle_positions[particle_index] = position

	# Update visual node position
	var particle = particle_nodes[particle_index]
	particle.position = position

func _log_performance():
	# Performance logging disabled for accurate benchmarking
	# var fps = Engine.get_frames_per_second()
	# print("🌧️ Godot Particles: %d particles | FPS: %.1f" % [particle_nodes.size(), fps])
	pass

## Utility functions for external access

func get_simulation_parameters() -> Dictionary:
	return {
		"gravity": gravity,
		"min_speed": min_speed,
		"max_speed": max_speed,
		"horizontal_drift": horizontal_drift,
		"world_bounds": world_bounds
	}

func set_simulation_parameters(params: Dictionary):
	if params.has("gravity"):
		gravity = params.gravity
	if params.has("min_speed"):
		min_speed = params.min_speed
	if params.has("max_speed"):
		max_speed = params.max_speed
	if params.has("horizontal_drift"):
		horizontal_drift = params.horizontal_drift
	if params.has("world_bounds"):
		world_bounds = params.world_bounds