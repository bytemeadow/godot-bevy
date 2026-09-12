extends "res://quad.gd"


func _process(_delta):
	pass


func set_capture_frame(frame: int) -> void:
	position.x = sin(frame / 50.) * 100.
