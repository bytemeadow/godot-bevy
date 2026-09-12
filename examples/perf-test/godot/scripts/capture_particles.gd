extends "res://scripts/godot_particles.gd"

var capture_running: bool = false

func _process(delta: float):
	if capture_running:
		super._process(delta)
	elif is_running:
		_update_particle_count()

func capture_prepare(capture_seed: int, count: int, bounds: Vector2, unspawned: bool):
	seed(capture_seed)
	world_bounds = bounds
	target_particle_count = count
	is_running = not unspawned
	capture_running = false

func capture_is_settled(expected: int) -> bool:
	if particle_nodes.size() != expected or particle_positions.size() != expected or particle_velocities.size() != expected:
		return false
	for particle in particle_nodes:
		var sprite: Sprite2D = particle.get_node("Sprite")
		if sprite.texture == null or sprite.texture.get_width() == 0 or sprite.texture.get_height() == 0:
			return false
	return true

func capture_reset(positions: PackedVector2Array, velocities: PackedVector2Array, count: int):
	capture_running = false
	target_particle_count = count
	particle_positions = positions
	particle_velocities = velocities
	for i in range(particle_nodes.size()):
		particle_nodes[i].position = positions[i]
		particle_nodes[i].visible = true
		particle_nodes[i].get_node("Sprite").modulate = Color(1.0, 1.0, 1.0, 0.8)
