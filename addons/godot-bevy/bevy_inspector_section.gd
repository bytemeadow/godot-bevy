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
var proxy
var inspector: WeakRef
var title: Label
var _header: PanelContainer
var _native := false

func _init() -> void:
	name = "BevySection"
	_native = Engine.is_editor_hint() and ClassDB.class_has_method("EditorInspector", "edit")
	title = Label.new()
	title.text = "Bevy"
	if not _native:
		title.text += " (native Inspector requires Godot 4.4)"
	title.horizontal_alignment = HORIZONTAL_ALIGNMENT_CENTER
	title.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
	_header = PanelContainer.new()
	_header.add_child(title)
	add_child(_header)
	status = Label.new()
	status.text = "Loading components…"
	add_child(status)

func _ready() -> void:
	if Engine.is_editor_hint():
		_header.add_theme_stylebox_override("panel", get_theme_stylebox("bg", "EditorInspectorCategory"))
		title.add_theme_font_override("font", get_theme_font("bold", "EditorFonts"))

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
	if _native:
		return
	_elapsed += delta
	if is_visible_in_tree() and _elapsed >= client.update_interval:
		_elapsed = 0.0
		refresh()

func _session_changed(active: int) -> void:
	if active != session_id:
		_session_detached = true
		if proxy != null:
			proxy.invalidate()

func refresh() -> void:
	if _waiting or _stale or _session_detached or entity.is_empty():
		return
	if pane != null:
		if proxy == null:
			proxy = pane.proxy_for(entity, session_id)
			proxy.status_changed.connect(_proxy_status)
			if not _native:
				proxy.components_changed.connect(_legacy_components)
		if _native and inspector == null:
			var embedded = load("res://addons/godot-bevy/bevy_native_inspector.gd").new()
			embedded.proxy = proxy
			inspector = weakref(embedded)
			add_child(embedded)
			_proxy_status()
		elif not _native and not proxy.components.is_empty():
			_legacy_components()
		proxy.refresh()
		return
	_waiting = true
	_request_id = client.request("godot.get_components", {"entity": entity}, _received, session_id)

func _proxy_status() -> void:
	status.text = proxy.status
	status.visible = not _native or proxy.status != "Entity " + str(entity.get("bits", ""))

func _legacy_components() -> void:
	_received({"result": proxy.components})

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
			control.configure(frame.result[component], entity, component, [], client, session_id, "", proxy)
			control.entity_link.connect(func(reference): pane.select_entity(reference))
			control.node_link.connect(func(id): remote.select_node(id, session_id))
			component_editors[component] = control

func _exit_tree() -> void:
	if proxy != null and proxy.status_changed.is_connected(_proxy_status):
		proxy.status_changed.disconnect(_proxy_status)
	if proxy != null and proxy.components_changed.is_connected(_legacy_components):
		proxy.components_changed.disconnect(_legacy_components)
	if client != null and _request_id != -1:
		client.cancel_request(_request_id)
