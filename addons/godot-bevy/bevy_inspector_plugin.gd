@tool
extends EditorInspectorPlugin

const Proxy = preload("res://addons/godot-bevy/bevy_entity_proxy.gd")
const Section = preload("res://addons/godot-bevy/bevy_inspector_section.gd")
const PropertyEditor = preload("res://addons/godot-bevy/bevy_property_editor.gd")

var client
var pane
var remote
var _section
var current_section: WeakRef
var _creating_editor := false

func _can_handle(object: Object) -> bool:
	return not _creating_editor and (object is Proxy or (object != null and object.get_class() in [
		"EditorDebuggerRemoteObject", "EditorDebuggerRemoteObjects"]))

func _parse_begin(object: Object) -> void:
	_section = null
	if object is Proxy:
		return
	_section = Section.new()
	current_section = weakref(_section)
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

func _parse_property(object: Object, _type: Variant.Type, name: String, _hint: PropertyHint, _hint_string: String, _usage: int, _wide: bool) -> bool:
	if object is Proxy and object.lookup.has(name):
		# The stock factory searches Inspector plugins again.
		_creating_editor = true
		var editor := PropertyEditor.create(object, name)
		_creating_editor = false
		add_property_editor(name, editor)
		return true
	return object is Proxy and name == "script"

func _parse_end(object: Object) -> void:
	if object is Proxy:
		object.repair_sections.call_deferred()
		return
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
