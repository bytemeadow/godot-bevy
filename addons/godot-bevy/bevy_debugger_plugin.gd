@tool
extends EditorDebuggerPlugin
## Bevy Debugger Message Handler
##
## This plugin captures debug messages from the running game and forwards
## entity/component data to the Bevy Inspector Panel.

# Reference to the inspector panel (set by the main plugin)
var inspector_panel = null

func _has_capture(prefix: String) -> bool:
	return prefix == "bevy"

func _capture(message: String, data: Array, session_id: int) -> bool:
	match message:
		"bevy:entities":
			if inspector_panel and inspector_panel.has_method("update_entities"):
				inspector_panel.update_entities(data)
			return true
		_:
			return false

func _setup_session(session_id: int) -> void:
	# We don't add UI here anymore - we use the dock panel instead
	pass
