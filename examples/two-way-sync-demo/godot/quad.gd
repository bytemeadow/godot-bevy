extends MeshInstance2D

func _process(delta):
	# Notice that we only change the x coordinate here. The y coordinate is
	# also updated every frame, except it's done in godot bevy, see lib.rs
	position.x = sin(Engine.get_frames_drawn() / 50.) * 100.
