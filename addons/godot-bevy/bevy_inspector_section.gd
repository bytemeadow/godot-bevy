@tool
extends VBoxContainer

const ValueEditor = preload("res://addons/godot-bevy/bevy_value_editor.gd")

var client
var pane
var remote
var entity: Dictionary = {}
var session_id := -1
var component_editors: Dictionary = {}
var status: Label
var _elapsed := 0.0
var _request_id := -1
var _waiting := false
var _stale := false
var _session_detached := false

func _init() -> void:
	name = "BevySection"
	var title := Label.new()
	title.text = "Bevy"
	add_child(title)
	status = Label.new()
	status.text = "Loading components…"
	add_child(status)

func configure(debugger, panel, adapter, reference: Dictionary, session: int, instance_id: String = "") -> void:
	client = debugger
	pane = panel
	remote = adapter
	entity = reference.duplicate(true)
	session_id = session
	client.active_session_changed.connect(_session_changed)
	_session_changed(client.active_session_id)
	if _session_detached:
		status.text = "Select an entity in this session"
		return
	if not instance_id.is_empty():
		_waiting = true
		_request_id = client.request("godot.entity_for_node", {"instance_id": instance_id}, func(frame):
			_waiting = false
			_request_id = -1
			if frame.has("error"):
				status.text = frame.error.message
				_stale = true
			else:
				entity = frame.result
				refresh()
		, session_id)
	elif not entity.is_empty():
		refresh()

func _process(delta: float) -> void:
	if client == null:
		return
	if _session_detached or not client.is_session_active(session_id) or session_id != client.active_session_id:
		status.text = "no running session" if client.active_session_id == -1 else "Select an entity in this session"
		for control in component_editors.values():
			control.hide()
		return
	_elapsed += delta
	if is_visible_in_tree() and _elapsed >= client.update_interval:
		_elapsed = 0.0
		refresh()

func _session_changed(active: int) -> void:
	if active != session_id:
		_session_detached = true

func refresh() -> void:
	if _waiting or _stale or _session_detached or entity.is_empty():
		return
	_waiting = true
	_request_id = client.request("godot.get_components", {"entity": entity}, _received, session_id)

func _received(frame: Dictionary) -> void:
	_waiting = false
	_request_id = -1
	if frame.has("error"):
		status.text = frame.error.message
		_stale = true
		return
	if _session_detached or session_id != client.active_session_id:
		return
	status.text = "Entity " + entity.bits
	for component in component_editors.keys():
		if not frame.result.has(component):
			component_editors[component].queue_free()
			component_editors.erase(component)
	for component in frame.result:
		if component_editors.has(component):
			component_editors[component].update_value(frame.result[component])
		else:
			var control = ValueEditor.new()
			add_child(control)
			control.configure(frame.result[component], entity, component, [], client, session_id)
			control.entity_link.connect(func(reference): pane.select_entity(reference))
			control.node_link.connect(func(id): remote.select_node(id, session_id))
			component_editors[component] = control

func _exit_tree() -> void:
	if client != null and _request_id != -1:
		client.cancel_request(_request_id)
