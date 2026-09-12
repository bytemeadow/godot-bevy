@tool
extends RefCounted
class_name BevyEntityProxy

var entity: Dictionary
var session_id: int

func _init(reference: Dictionary = {}, session: int = -1) -> void:
	entity = reference.duplicate(true)
	session_id = session
