@tool
extends Node

var row: EditorProperty
var proxy
var property_name := ""
var status: Label
var warning: TextureRect
var explanation: Label

func configure(editor: EditorProperty, target, property: String) -> void:
	row = editor
	proxy = target
	property_name = property
	var entry: Dictionary = proxy.lookup[property]
	var bottom := VBoxContainer.new()
	explanation = Label.new()
	explanation.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	bottom.add_child(explanation)
	var message := HBoxContainer.new()
	bottom.add_child(message)
	warning = TextureRect.new()
	warning.stretch_mode = TextureRect.STRETCH_KEEP_CENTERED
	message.add_child(warning)
	status = Label.new()
	status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	status.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	message.add_child(status)
	if entry.label.contains(" › "):
		var copy := LinkButton.new()
		copy.text = "Copy full path"
		copy.tooltip_text = entry.copy_path
		copy.pressed.connect(func(): DisplayServer.clipboard_set(entry.copy_path))
		bottom.add_child(copy)
	row.add_child(bottom)
	row.set_bottom_editor(bottom)
	row.set_meta("bevy_binding", weakref(self))
	proxy.field_changed.connect(_field_changed)
	row.selected.connect(func(property_path, _focusable): proxy.selected_property = String(property_path))

func _ready() -> void:
	proxy.sections_repaired.connect(_restore, CONNECT_ONE_SHOT)

func _restore() -> void:
	if not proxy.lookup.has(property_name):
		return
	var entry: Dictionary = proxy.lookup[property_name]
	row.label = entry.label
	row.tooltip_text = entry.component + "\n" + entry.model.type_path + "\n" + entry.copy_path
	_sync()
	if proxy.selected_property == property_name and row.has_method("select"):
		row.call("select", -1)
	if row.has_method("text_input"):
		var line: LineEdit = row.call("text_input")
		var state: Dictionary = proxy.states[property_name]
		if line != null and state.focused:
			line.grab_focus()
		if line != null and state.dirty:
			line.caret_column = state.caret
			line.select(state.selection_from, state.selection_to)

func _field_changed(property: String) -> void:
	if property == property_name:
		_sync()
		row.update_property()

func _sync() -> void:
	if not proxy.lookup.has(property_name):
		return
	var entry: Dictionary = proxy.lookup[property_name]
	var state: Dictionary = proxy.states[property_name]
	row.set_read_only(proxy.detached or state.pending or bool(entry.property.usage & PROPERTY_USAGE_READ_ONLY))
	explanation.text = entry.reason
	explanation.visible = not entry.reason.is_empty() and not entry.model.kind in ["unsupported", "depth_limit"]
	status.text = state.status
	status.visible = not state.status.is_empty()
	warning.visible = state.rejected
	if is_inside_tree():
		warning.texture = row.get_theme_icon("StatusWarning", "EditorIcons")
		if state.rejected:
			status.add_theme_color_override("font_color", row.get_theme_color("warning_color", "Editor"))
		else:
			status.remove_theme_color_override("font_color")

func _exit_tree() -> void:
	if proxy.field_changed.is_connected(_field_changed):
		proxy.field_changed.disconnect(_field_changed)
	if proxy.sections_repaired.is_connected(_restore):
		proxy.sections_repaired.disconnect(_restore)
