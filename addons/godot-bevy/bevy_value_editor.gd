@tool
extends VBoxContainer

signal edit_requested(params)
signal edit_finished(accepted, reason)
signal entity_link(reference)
signal node_link(instance_id)

var model: Dictionary = {}
var entity: Dictionary = {}
var component := ""
var path: Array = []
var client
var session_id := -1
var editor
var error_label: Label
var children_box: VBoxContainer
var children_editors: Array = []
var fold: Button
var pending := false
var _request_id := -1
var _label := ""
var _shape := ""
var proxy
var property_name := ""
var _fold_key := ""

func configure(value: Dictionary, reference: Dictionary, type_path: String, leaf_path: Array,
		debugger = null, session: int = -1, label: String = "", controller = null) -> void:
	entity = reference.duplicate(true)
	component = type_path
	path = leaf_path.duplicate(true)
	client = debugger
	session_id = session
	_label = label
	proxy = controller
	_fold_key = component + JSON.stringify(path)
	if proxy != null:
		for property in proxy.lookup:
			var entry: Dictionary = proxy.lookup[property]
			if entry.component == component and entry.path == path and entry.role == "value" and entry.model == value:
				property_name = property
				break
		proxy.field_changed.connect(_proxy_changed)
	update_value(value)
	if not property_name.is_empty():
		_proxy_changed(property_name)

func _description(value: Dictionary) -> String:
	var shape: Array = [value.kind, value.type_path, value.writable, value.get("range"), _exact_integer(value)]
	for field in value.get("fields", []):
		shape.append(field.get("name", "") if value.kind in ["struct", "enum"] else "")
	for key in ["items", "entries", "members"]:
		shape.append(value.get(key, []).size())
	shape.append(value.get("unit_variants", []))
	if value.kind == "enum":
		shape.append(value.variant)
	return JSON.stringify(shape)

static func _exact_integer(value: Dictionary) -> bool:
	return value.kind == "integer" and (value.get("value") is String or
		value.type_path in ["i64", "u64", "i128", "u128", "isize", "usize"] or
		abs(float(value.get("value", 0))) > 9007199254740991.0)

func update_value(value: Dictionary) -> void:
	if pending or _has_edit_focus():
		return
	model = value.duplicate(true)
	var shape := _description(model)
	if shape != _shape:
		_shape = shape
		_build()
	else:
		_display()
		var children := _children()
		for i in children_editors.size():
			children_editors[i].update_value(children[i].value)

func _has_edit_focus() -> bool:
	return is_instance_valid(editor) and (editor.has_focus() or
		(editor is SpinBox and editor.get_line_edit().has_focus()))

func _build() -> void:
	var expanded := fold.button_pressed if is_instance_valid(fold) else false
	if proxy != null:
		expanded = proxy.legacy_folds.get(_fold_key, false)
	for child in get_children():
		remove_child(child)
		child.queue_free()
	children_editors.clear()
	editor = null
	fold = null
	children_box = null
	var row := HBoxContainer.new()
	add_child(row)
	var label := Label.new()
	label.text = _label if not _label.is_empty() else component.get_slice("::", component.get_slice_count("::") - 1)
	label.tooltip_text = model.type_path
	row.add_child(label)
	var reason: String = model.get("reason", model.writable.get("reason", ""))
	var writable: bool = model.writable.allowed
	var kind: String = model.kind
	if kind in ["struct", "tuple_struct", "tuple", "list", "array", "map", "set", "enum"]:
		fold = Button.new()
		fold.toggle_mode = true
		fold.button_pressed = expanded
		fold.text = "Fields" if kind != "enum" else model.variant
		row.add_child(fold)
		children_box = VBoxContainer.new()
		children_box.visible = expanded
		children_box.add_theme_constant_override("separation", 4)
		add_child(children_box)
		fold.toggled.connect(func(open):
			children_box.visible = open
			if proxy != null:
				proxy.legacy_folds[_fold_key] = open)
		if not reason.is_empty():
			var explanation := Label.new()
			explanation.text = reason
			explanation.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
			children_box.add_child(explanation)
		for child in _children():
			var control = load("res://addons/godot-bevy/bevy_value_editor.gd").new()
			children_box.add_child(control)
			control.configure(child.value, entity, component, child.path, client, session_id, child.label, proxy)
			control.entity_link.connect(func(reference): entity_link.emit(reference))
			control.node_link.connect(func(id): node_link.emit(id))
			children_editors.append(control)
		if model.get("truncated", 0) > 0:
			var notice := Label.new()
			notice.text = "%d more (read limit)" % model.truncated
			children_box.add_child(notice)
	if kind == "entity":
		editor = LinkButton.new()
		editor.text = "Entity " + model.entity.bits
		editor.pressed.connect(func(): entity_link.emit(model.entity))
	elif kind == "node":
		editor = LinkButton.new()
		editor.text = "Node " + model.instance_id if model.valid else "Freed node " + model.instance_id
		editor.disabled = not model.valid
		editor.pressed.connect(func(): node_link.emit(model.instance_id))
	elif writable and kind in ["integer", "float"] and not _exact_integer(model):
		editor = SpinBox.new()
		editor.step = 1.0 if kind == "integer" else 0.0
		if kind == "float":
			editor.custom_arrow_step = 0.01
		editor.allow_greater = not model.has("range")
		editor.allow_lesser = not model.has("range")
		if model.has("range"):
			editor.min_value = model.range.min
			editor.max_value = model.range.max
		editor.value_changed.connect(func(value): submit_edit(int(value) if model.kind == "integer" else value))
	elif writable and kind == "bool":
		editor = CheckBox.new()
		editor.toggled.connect(submit_edit)
	elif writable and (kind in ["string", "char"] or _exact_integer(model)):
		editor = LineEdit.new()
		editor.text_submitted.connect(submit_edit)
		editor.gui_input.connect(func(event):
			if event is InputEventKey and event.pressed and event.keycode == KEY_ESCAPE:
				if proxy != null and not property_name.is_empty():
					proxy.restore_candidate(property_name)
				_display()
				editor.accept_event())
		if _exact_integer(model):
			editor.tooltip_text = "Exact decimal integer; range checked by the runtime"
	elif writable and kind == "enum" and not model.unit_variants.is_empty():
		editor = OptionButton.new()
		if not model.variant in model.unit_variants:
			editor.add_item(model.variant)
			editor.set_item_disabled(0, true)
		for variant in model.unit_variants:
			editor.add_item(variant)
		editor.item_selected.connect(func(index): submit_edit({"variant": editor.get_item_text(index)}))
	elif children_box == null:
		editor = Label.new()
		editor.text = str(model.get("value", model.get("debug", model.get("id", reason))))
		editor.tooltip_text = reason
	if editor != null:
		editor.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		row.add_child(editor)
		if proxy != null and not property_name.is_empty() and (editor is SpinBox or editor is LineEdit):
			var line: LineEdit = editor.get_line_edit() if editor is SpinBox else editor
			line.text_changed.connect(func(text): proxy.set_candidate(property_name, text, line.caret_column))
			line.focus_entered.connect(func():
				proxy.states[property_name].focused = true
				proxy.selected_property = property_name)
			line.focus_exited.connect(func(): _focus_exited.call_deferred())
	error_label = Label.new()
	error_label.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	error_label.visible = false
	add_child(error_label)
	_display()

func _children() -> Array:
	var out: Array = []
	var kind: String = model.kind
	var base = path.duplicate(true)
	if kind == "enum":
		base.append({"variant": model.variant})
	for i in model.get("fields", []).size():
		var field = model.fields[i]
		var named: bool = kind in ["struct", "enum"]
		var label: String = field.name if named else str(i)
		var segment = {"field": label} if named and not (kind == "enum" and label.is_valid_int()) else {"index": i}
		out.append({"label": label, "value": field.value if named else field, "path": base + [segment]})
	for i in model.get("items", []).size():
		out.append({"label": str(i), "value": model.items[i], "path": base + [{"index": i}]})
	for i in model.get("entries", []).size():
		for key in ["key", "value"]:
			out.append({"label": "%d %s" % [i, key], "value": model.entries[i][key], "path": base})
	for i in model.get("members", []).size():
		out.append({"label": str(i), "value": model.members[i], "path": base})
	return out

func _display() -> void:
	if not is_instance_valid(editor):
		return
	var state: Dictionary = proxy.states.get(property_name, {}) if proxy != null else {}
	if editor is SpinBox:
		editor.editable = not pending and model.writable.allowed
		if state.get("dirty", false):
			editor.get_line_edit().text = state.candidate
		else:
			editor.set_value_no_signal(float(model.value))
	elif editor is CheckBox:
		editor.disabled = pending or not model.writable.allowed
		editor.set_pressed_no_signal(model.value)
	elif editor is LineEdit:
		editor.editable = not pending and model.writable.allowed
		editor.text = state.candidate if state.get("dirty", false) else str(model.value)
	elif editor is OptionButton:
		editor.disabled = pending or not model.writable.allowed
		for i in editor.item_count:
			if editor.get_item_text(i) == model.variant:
				editor.select(i)
	elif editor is Label:
		editor.text = str(model.get("value", model.get("debug", model.get("id", model.get("reason", "")))))

func submit_edit(value) -> void:
	_display()
	if pending:
		return
	if proxy != null and not property_name.is_empty():
		proxy.set(property_name, value.variant if model.kind == "enum" else value)
		return
	if not model.writable.allowed:
		_show_error(model.writable.get("reason", "read-only"))
		return
	pending = true
	_display()
	error_label.text = "Waiting for acknowledgement…"
	error_label.show()
	var params = {"entity": entity.duplicate(true), "component": component,
		"path": path.duplicate(true), "value": value}
	edit_requested.emit(params)
	if client != null:
		_request_id = client.request("godot.mutate_leaf", params, receive_edit, session_id)

func receive_edit(frame: Dictionary) -> void:
	pending = false
	_request_id = -1
	if frame.has("error"):
		var reason: String = frame.error.get("data", {}).get("reason", frame.error.message)
		_display()
		_show_error(reason)
		edit_finished.emit(false, reason)
	else:
		# An acknowledgement replaces even a focused input; periodic reads do not.
		model = frame.result.duplicate(true)
		var shape := _description(model)
		if shape != _shape:
			_shape = shape
			_build()
		_display()
		error_label.text = "Accepted"
		error_label.show()
		edit_finished.emit(true, "")

func _show_error(reason: String) -> void:
	error_label.text = reason
	error_label.show()

func _proxy_changed(property: String) -> void:
	if property != property_name or not proxy.lookup.has(property):
		return
	var state: Dictionary = proxy.states[property]
	var was_pending := pending
	pending = state.pending
	model = proxy.lookup[property].model.duplicate(true)
	if model.has("value"):
		model.value = proxy.get(property)
	var shape := _description(model)
	if shape != _shape:
		_shape = shape
		_build()
	_display()
	error_label.text = state.status
	error_label.visible = not state.status.is_empty()
	if proxy.detached and editor != null:
		if editor is SpinBox or editor is LineEdit:
			editor.editable = false
		elif editor is BaseButton:
			editor.disabled = true
	if was_pending and not pending:
		edit_finished.emit(not state.rejected, state.status if state.rejected else "")

func _focus_exited() -> void:
	if proxy != null and proxy.states.has(property_name) and not _has_edit_focus():
		proxy.states[property_name].focused = false

func _exit_tree() -> void:
	if proxy != null and proxy.field_changed.is_connected(_proxy_changed):
		proxy.field_changed.disconnect(_proxy_changed)
	if client != null and _request_id != -1:
		client.cancel_request(_request_id)
