@tool
extends EditorProperty

const Binding = preload("res://addons/godot-bevy/bevy_property_binding.gd")

var proxy
var property_name := ""
var input
var _updating := false

static func create(target, property: String) -> EditorProperty:
	var entry: Dictionary = target.lookup[property]
	var stock: bool = entry.model.kind == "bool" or (entry.model.kind == "enum" and entry.accepted in entry.model.unit_variants and not entry.model.unit_variants.is_empty())
	var row: EditorProperty
	if stock and ClassDB.class_has_method("EditorInspector", "instantiate_property_editor"):
		row = load("res://addons/godot-bevy/bevy_property_factory.gd").create(target, entry.property)
	if row == null:
		var custom = load("res://addons/godot-bevy/bevy_property_editor.gd").new()
		custom.configure(target, property)
		row = custom
	var binding = Binding.new()
	binding.configure(row, target, property)
	row.add_child(binding)
	return row

func configure(target, property: String) -> void:
	proxy = target
	property_name = property
	var entry: Dictionary = proxy.lookup[property]
	var model: Dictionary = entry.model
	if entry.property.type in [TYPE_INT, TYPE_FLOAT]:
		var number := SpinBox.new()
		number.step = 1.0 if entry.property.type == TYPE_INT else 0.0
		if entry.property.type == TYPE_FLOAT:
			number.custom_arrow_step = 0.01
		number.allow_greater = not model.has("range")
		number.allow_lesser = not model.has("range")
		if model.has("range"):
			number.min_value = model.range.min
			number.max_value = model.range.max
		number.value_changed.connect(func(value):
			if not _updating:
				emit_changed(property_name, int(value) if entry.property.type == TYPE_INT else value))
		input = number
	elif model.kind == "bool":
		var check := CheckBox.new()
		check.toggled.connect(func(value): emit_changed(property_name, value))
		input = check
	elif model.kind == "enum":
		var choice := OptionButton.new()
		if not model.variant in model.unit_variants:
			choice.add_item(model.variant)
			choice.set_item_disabled(0, true)
		for variant in model.unit_variants:
			choice.add_item(variant)
		choice.item_selected.connect(func(index): emit_changed(property_name, choice.get_item_text(index)))
		input = choice
	elif model.kind in ["entity", "node"]:
		var link := Button.new()
		link.flat = true
		link.clip_text = true
		link.text_overrun_behavior = TextServer.OVERRUN_TRIM_ELLIPSIS
		link.pressed.connect(func():
			var current: Dictionary = proxy.lookup[property_name].model
			if current.kind == "entity":
				proxy.entity_link.emit(current.entity)
			elif current.valid:
				proxy.node_link.emit(current.instance_id))
		input = link
	elif entry.role != "value" or model.kind in ["unsupported", "depth_limit"]:
		var text := Label.new()
		text.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
		text.size_flags_horizontal = Control.SIZE_EXPAND_FILL
		input = text
	else:
		var text := LineEdit.new()
		text.text_submitted.connect(func(value): emit_changed(property_name, value))
		input = text
	add_child(input)
	var line := text_input()
	if line != null:
		add_focusable(line)
		line.text_changed.connect(func(value):
			if not _updating:
				proxy.set_candidate(property_name, value, line.caret_column, line.get_selection_from_column(), line.get_selection_to_column()))
		line.gui_input.connect(_text_input_event)
		line.focus_entered.connect(func(): proxy.states[property_name].focused = true)
		line.focus_exited.connect(func(): _focus_exited.call_deferred())
	elif input.focus_mode != Control.FOCUS_NONE:
		add_focusable(input)
	_update_property()

func text_input() -> LineEdit:
	if input is LineEdit:
		return input
	if input is SpinBox:
		return input.get_line_edit()
	return null

func _text_input_event(event: InputEvent) -> void:
	if event is InputEventKey and event.pressed and event.keycode == KEY_ESCAPE:
		proxy.restore_candidate(property_name)
		_update_property()
		text_input().accept_event()

func _focus_exited() -> void:
	if proxy.states.has(property_name) and not text_input().has_focus():
		proxy.states[property_name].focused = false

func _update_property() -> void:
	if proxy == null or not proxy.lookup.has(property_name):
		return
	var entry: Dictionary = proxy.lookup[property_name]
	var state: Dictionary = proxy.states[property_name]
	_updating = true
	if input is SpinBox:
		if state.dirty:
			input.get_line_edit().text = state.candidate
		else:
			input.set_value_no_signal(float(entry.accepted))
	elif input is LineEdit:
		var value: String = state.candidate if state.dirty else str(entry.accepted)
		if input.text != value:
			input.text = value
	elif input is CheckBox:
		input.set_pressed_no_signal(entry.accepted)
	elif input is OptionButton:
		for i in input.item_count:
			if input.get_item_text(i) == entry.accepted:
				input.select(i)
	else:
		input.text = str(entry.accepted)
	if entry.model.kind in ["entity", "node"]:
		input.disabled = proxy.detached or (entry.model.kind == "node" and not entry.model.valid)
	_updating = false

func _set_read_only(readonly: bool) -> void:
	if input is SpinBox or input is LineEdit:
		input.editable = not readonly
	elif input is BaseButton and not proxy.lookup[property_name].model.kind in ["entity", "node"]:
		input.disabled = readonly

func _exit_tree() -> void:
	if proxy == null or not proxy.states.has(property_name):
		return
	var line := text_input()
	if line != null:
		var state: Dictionary = proxy.states[property_name]
		if state.dirty:
			state.caret = line.caret_column
			state.selection_from = line.get_selection_from_column()
			state.selection_to = line.get_selection_to_column()
