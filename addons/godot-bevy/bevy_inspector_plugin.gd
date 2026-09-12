@tool
extends EditorInspectorPlugin

const Proxy = preload("res://addons/godot-bevy/bevy_entity_proxy.gd")
const Section = preload("res://addons/godot-bevy/bevy_inspector_section.gd")

var client
var pane
var remote
var _section
var current_section: WeakRef

func _can_handle(object: Object) -> bool:
	return object is Proxy or (object != null and object.get_class() in [
		"EditorDebuggerRemoteObject", "EditorDebuggerRemoteObjects"])

func _parse_begin(object: Object) -> void:
	_section = Section.new()
	current_section = weakref(_section)
	if object is Proxy:
		_section.configure(client, pane, remote, object.entity, object.session_id)
		return
	var instance_id := ""
	var session: int = client.active_session_id
	if ClassDB.class_exists("EditorDebuggerRemoteObjects"):
		var title_id := ""
		if object.has_method("get_title"):
			var suffix: String = str(object.call("get_title")).get_slice(": ", 1)
			if suffix.is_valid_int():
				title_id = suffix
		if remote.last_session == session and remote.last_ids.size() == 1:
			instance_id = remote.last_ids[0]
			if not title_id.is_empty() and title_id != instance_id:
				instance_id = title_id
		elif not title_id.is_empty():
			instance_id = title_id
		elif remote.last_ids.size() > 1:
			_section.status.text = "Select one remote node to inspect Bevy components"
			return
	elif object.has_method("get_remote_object_id"):
		instance_id = str(object.call("get_remote_object_id"))
	if instance_id.is_empty():
		_section.status.text = "Bevy remote inspection not available on this Godot version"
	else:
		_section.configure(client, pane, remote, {}, session, instance_id)

func _parse_end(_object: Object) -> void:
	if _section != null:
		add_custom_control(_section)
		_section = null

func shutdown() -> void:
	if current_section != null:
		var section = current_section.get_ref()
		if section != null:
			section.set_process(false)
			section.hide()
			section.queue_free()
