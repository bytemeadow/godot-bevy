extends Node

func _ready() -> void:
	EngineDebugger.register_message_capture("entity_probe", _capture)

func _capture(message: String, _data: Array) -> bool:
	if message != "enter_level":
		return false
	_enter_level.call_deferred()
	return true

func _enter_level() -> void:
	var scene = get_tree().current_scene
	var button = scene.get_node_or_null("Options/StartButton") if scene else null
	if button == null:
		push_error("EDITOR_PROBE runtime: StartButton missing")
		return
	button.grab_focus()
	var event := InputEventKey.new()
	event.keycode = KEY_ENTER
	event.pressed = true
	Input.parse_input_event(event)
	await get_tree().process_frame
	event = InputEventKey.new()
	event.keycode = KEY_ENTER
	event.pressed = false
	Input.parse_input_event(event)

func _exit_tree() -> void:
	EngineDebugger.unregister_message_capture("entity_probe")
