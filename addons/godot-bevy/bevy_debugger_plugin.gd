@tool
extends EditorDebuggerPlugin

var client = preload("res://addons/godot-bevy/bevy_rpc_client.gd").new()

func _has_capture(prefix: String) -> bool:
	return prefix == "bevy"

func _setup_session(session_id: int) -> void:
	client.setup_session(session_id, get_session(session_id))

func _capture(message: String, data: Array, session_id: int) -> bool:
	return client._capture(message, data, session_id)

func shutdown() -> void:
	client.shutdown()
