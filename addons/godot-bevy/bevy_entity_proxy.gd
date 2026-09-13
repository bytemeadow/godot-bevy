@tool
extends RefCounted
class_name BevyEntityProxy

signal field_changed(property)
signal components_changed
signal status_changed
signal sections_repaired
signal entity_link(reference)
signal node_link(instance_id)

var entity: Dictionary
var session_id: int
var incarnation := -1
var client
var components: Dictionary = {}
var lookup: Dictionary = {}
var states: Dictionary = {}
var selected_property := ""
var legacy_folds: Dictionary = {}
var section_tooltips: Dictionary = {}
var status := "Loading components…"
var detached := false
var _properties: Array[Dictionary] = []
var _inspectors: Array[WeakRef] = []
var _shape := ""
var _read_id := -1
var _revision := 0
var _elapsed := 0.0
var _stale := false

func _init(reference: Dictionary = {}, session: int = -1) -> void:
	entity = reference.duplicate(true)
	session_id = session

func configure(debugger) -> void:
	client = debugger
	incarnation = client.sessions.get(session_id, {}).get("order", -1)
	client.active_session_changed.connect(_session_changed)

func _session_changed(active: int) -> void:
	if active != session_id or not valid_target():
		invalidate()

func valid_target() -> bool:
	return not detached and client != null and client.is_session_active(session_id) and \
		client.active_session_id == session_id and client.sessions.get(session_id, {}).get("order", -1) == incarnation

func _get_property_list() -> Array[Dictionary]:
	return _properties

func _property_can_revert(_property: StringName) -> bool:
	return false

func _get(property: StringName):
	var entry: Dictionary = lookup.get(String(property), {})
	return entry.get("accepted")

func _set(property: StringName, candidate) -> bool:
	var name := String(property)
	if not lookup.has(name):
		return false
	var entry: Dictionary = lookup[name]
	var state: Dictionary = states[name]
	if client != null and not valid_target():
		invalidate()
	if state.pending:
		return true
	if detached or entry.property.usage & PROPERTY_USAGE_READ_ONLY:
		state.status = "Select an entity in this session" if detached else entry.reason
		state.rejected = true
		field_changed.emit(name)
		return true
	state.pending = true
	state.status = "Waiting for acknowledgement…"
	state.rejected = false
	state.candidate = str(entry.accepted)
	state.dirty = false
	_revision += 1
	field_changed.emit(name)
	var value = {"variant": candidate} if entry.model.kind == "enum" else candidate
	state.request = client.request("godot.mutate_leaf", {"entity": entity.duplicate(true),
		"component": entry.component, "path": entry.path.duplicate(true), "value": value},
		_receive_edit.bind(name), session_id)
	return true

func _receive_edit(frame: Dictionary, name: String) -> void:
	if detached or not states.has(name):
		return
	var state: Dictionary = states[name]
	state.pending = false
	state.request = -1
	_revision += 1
	if frame.has("error"):
		state.status = frame.error.get("data", {}).get("reason", frame.error.message)
		state.rejected = true
	else:
		# The response is the accepted leaf, including enum payload/permission changes.
		lookup[name].model.clear()
		lookup[name].model.merge(frame.result.duplicate(true))
		accept_components(components, name)
		state.status = "Accepted"
		state.rejected = false
	state.candidate = str(lookup[name].accepted) if lookup.has(name) else ""
	state.dirty = false
	field_changed.emit(name)
	if _read_id != -1:
		client.cancel_request(_read_id)
		_read_id = -1
	refresh()

func set_candidate(name: String, text: String, caret: int = 0, from: int = 0, to: int = 0) -> void:
	if not states.has(name):
		return
	states[name].candidate = text
	states[name].dirty = true
	states[name].caret = caret
	states[name].selection_from = from
	states[name].selection_to = to

func restore_candidate(name: String) -> void:
	states[name].candidate = str(lookup[name].accepted)
	states[name].dirty = false
	field_changed.emit(name)

func refresh() -> void:
	if _read_id != -1 or _stale or not valid_target() or entity.is_empty():
		return
	_read_id = client.request("godot.get_components", {"entity": entity}, _received.bind(_revision), session_id)

func _received(frame: Dictionary, revision: int) -> void:
	_read_id = -1
	if not valid_target():
		return
	if revision != _revision:
		refresh()
		return
	if frame.has("error"):
		status = frame.error.message
		_stale = true
		status_changed.emit()
		return
	accept_components(frame.result)

func advance(delta: float) -> void:
	if not valid_target():
		invalidate()
		return
	_elapsed += delta
	if _elapsed < client.update_interval:
		return
	_elapsed = 0.0
	for reference in _inspectors:
		var inspector = reference.get_ref()
		if inspector != null and inspector.is_visible_in_tree() and inspector.get_edited_object() == self:
			refresh()
			return

func accept_components(values: Dictionary, acknowledged: String = "") -> void:
	var previous := lookup
	components = values.duplicate(true)
	lookup = {}
	section_tooltips = {}
	_properties = [{"name": "Bevy", "type": TYPE_NIL, "usage": PROPERTY_USAGE_CATEGORY}]
	_add_notice("@entity", "Entity", "Entity " + str(entity.get("bits", "")), "", [], [], "identity")
	var types := components.keys()
	types.sort()
	var counts: Dictionary = {}
	for component in types:
		var short := component_label(component)
		counts[short] = counts.get(short, 0) + 1
	for component in types:
		var short := component_label(component)
		var label: String = component if counts[short] > 1 else short
		label = label.replace("/", "∕")
		var prefix: String = "component_" + component.uri_encode() + "/"
		var model: Dictionary = components[component]
		if not _aggregate(model) or _single_unnamed_leaf(model):
			_properties.append({"name": "", "type": TYPE_NIL, "usage": PROPERTY_USAGE_GROUP})
			var path: Array = [{"index": 0}] if _single_unnamed_leaf(model) else []
			var leaf: Dictionary = model.fields[0] if not path.is_empty() else model
			_add_leaf(prefix.trim_suffix("/") + "@value", label, leaf, component, path, [], false)
			continue
		_properties.append({"name": label, "type": TYPE_NIL, "usage": PROPERTY_USAGE_GROUP, "hint_string": prefix})
		var sections: Array = [{"key": prefix, "label": label}]
		_map(model, component, [], [], prefix, sections, false)
	var signature: Array = [_properties, section_tooltips]
	for name in lookup:
		var entry: Dictionary = lookup[name]
		var model: Dictionary = entry.model
		signature.append([name, model.kind, model.type_path, model.writable, entry.reason, model.get("unit_variants", []), model.get("variant")])
		if not states.has(name):
			states[name] = {"pending": false, "request": -1, "status": "", "rejected": false,
				"candidate": "", "dirty": false, "focused": false, "caret": 0, "selection_from": 0, "selection_to": 0}
		var state: Dictionary = states[name]
		if previous.has(name) and name != acknowledged and (state.pending or state.focused or state.dirty) and entry.property.type == previous[name].property.type:
			entry.accepted = previous[name].accepted
			if model.has("value"):
				model.value = previous[name].accepted
		if not state.dirty and not state.pending:
			state.candidate = str(entry.accepted)
	for name in states.keys():
		if not lookup.has(name):
			if client != null and states[name].request != -1:
				client.cancel_request(states[name].request)
			states.erase(name)
	var shape := JSON.stringify(signature)
	if shape != _shape:
		_shape = shape
		notify_property_list_changed()
	else:
		for name in lookup:
			if not previous.has(name) or previous[name].accepted != lookup[name].accepted:
				_refresh_property(name)
	status = "Entity " + str(entity.get("bits", ""))
	components_changed.emit()
	status_changed.emit()

static func component_label(type_path: String) -> String:
	var qualified := RegEx.new()
	qualified.compile("(?:[A-Za-z_][A-Za-z0-9_]*::)+([A-Za-z_][A-Za-z0-9_]*)")
	return qualified.sub(type_path, "$1", true)

static func _aggregate(model: Dictionary) -> bool:
	return model.kind in ["struct", "tuple_struct", "tuple", "list", "array", "map", "set", "enum"]

static func _single_unnamed_leaf(model: Dictionary) -> bool:
	return model.kind in ["tuple_struct", "tuple"] and model.fields.size() == 1 and not _aggregate(model.fields[0])

static func exact_integer(model: Dictionary) -> bool:
	return model.kind == "integer" and (model.get("value") is String or
		model.type_path in ["i64", "u64", "i128", "u128", "isize", "usize"] or
		abs(float(model.get("value", 0))) > 9007199254740991.0)

func _map(model: Dictionary, component: String, path: Array, labels: Array, prefix: String, sections: Array, readonly: bool) -> void:
	var kind: String = model.kind
	var aggregate := _aggregate(model)
	var ancestry := labels.duplicate()
	if not aggregate and not ancestry.is_empty():
		ancestry.pop_back()
	var visible_ancestry := ancestry.slice(0, 3)
	var nested := sections.slice(0, 1)
	var stem := prefix
	for i in visible_ancestry.size():
		stem += String(visible_ancestry[i]).uri_encode() + "/"
		nested.append({"key": stem, "label": str(visible_ancestry[i])})
	if aggregate:
		if not section_tooltips.has(stem):
			section_tooltips[stem] = component + ("\n" + JSON.stringify(path) if not path.is_empty() else "")
		var first := lookup.size()
		if kind == "enum":
			_add_leaf(stem + _tail(labels, "@variant"), "Variant", model, component, path, nested, readonly)
		if model.get("truncated", 0) > 0:
			_add_notice(stem + _tail(labels, "@truncated"), "Truncated", "%d more (read limit)" % model.truncated, component, path, nested, "truncated")
		var base := path.duplicate(true)
		if kind == "enum":
			base.append({"variant": model.variant})
		for i in model.get("fields", []).size():
			var field = model.fields[i]
			var named: bool = kind in ["struct", "enum"]
			var label: String = field.name if named else str(i)
			var segment = {"field": label} if named and not (kind == "enum" and label.is_valid_int()) else {"index": i}
			_map(field.value if named else field, component, base + [segment], labels + [label], prefix, sections, readonly)
		for i in model.get("items", []).size():
			_map(model.items[i], component, base + [{"index": i}], labels + [str(i)], prefix, sections, readonly)
		for i in model.get("entries", []).size():
			for key in ["key", "value"]:
				_map(model.entries[i][key], component, base, labels + [str(i), key], prefix, sections, true)
		for i in model.get("members", []).size():
			_map(model.members[i], component, base, labels + [str(i)], prefix, sections, true)
		if lookup.size() == first:
			_add_notice(stem + _tail(labels, "@empty"), "Value", "No fields" if kind in ["struct", "tuple_struct", "tuple"] else "Empty", component, path, nested, "empty")
		var reason: String = model.writable.get("reason", "")
		var restricted := false
		var reason_visible := false
		for entry in lookup.values().slice(first):
			if entry.role == "value":
				restricted = restricted or bool(entry.property.usage & PROPERTY_USAGE_READ_ONLY)
				reason_visible = reason_visible or entry.reason == reason
		if restricted and not reason_visible and not reason.is_empty() and not reason.begins_with("kind not editable"):
			section_tooltips[stem] += "\n" + (" › ".join(labels.slice(3)) + ": " if labels.size() > 3 else "") + reason
	else:
		var tail: Array = labels.slice(3) if labels.size() > 4 else labels.slice(labels.size() - 1)
		var label := "Value" if labels.is_empty() else " › ".join(tail)
		# Godot assigns a synthetic null default to property names ending in /<digits>.
		var name := "@value" if labels.is_empty() else "@leaf_" + label.uri_encode()
		if labels.size() > 4:
			name += "@" + JSON.stringify(labels).uri_encode()
		_add_leaf(stem + name, label, model, component, path, nested, readonly)

func _tail(labels: Array, suffix: String) -> String:
	return (JSON.stringify(labels.slice(3)).uri_encode() if labels.size() > 3 else "") + suffix

func _add_notice(name: String, label: String, text: String, component: String, path: Array, sections: Array, role: String) -> void:
	_add_leaf(name, label, {"kind": "string", "type_path": "", "value": text,
		"writable": {"allowed": false}}, component, path, sections, true, role)

func _add_leaf(name: String, label: String, model: Dictionary, component: String, path: Array, sections: Array, readonly: bool, role: String = "value") -> void:
	var kind: String = model.kind
	var type := TYPE_STRING
	if kind == "bool":
		type = TYPE_BOOL
	elif kind == "integer" and not exact_integer(model):
		type = TYPE_INT
	elif kind == "float":
		type = TYPE_FLOAT
	var writable: bool = model.writable.allowed and not readonly and kind in ["bool", "integer", "float", "string", "char", "enum"]
	var property := {"name": name, "type": type, "usage": PROPERTY_USAGE_EDITOR | (0 if writable else PROPERTY_USAGE_READ_ONLY), "hint": PROPERTY_HINT_NONE, "hint_string": ""}
	var reason: String = model.get("reason", model.writable.get("reason", ""))
	if readonly and reason.is_empty() and role == "value":
		reason = "Collection entries are read-only"
	if kind == "enum":
		property.hint = PROPERTY_HINT_ENUM
		property.hint_string = ",".join(model.unit_variants)
	elif model.has("range"):
		if type in [TYPE_INT, TYPE_FLOAT]:
			property.hint = PROPERTY_HINT_RANGE
			property.hint_string = "%s,%s,%s" % [model.range.min, model.range.max, "1" if type == TYPE_INT else "0.01"]
		else:
			reason += ("\n" if not reason.is_empty() else "") + "Bounds: %s to %s" % [model.range.min, model.range.max]
	var accepted = model.get("value", model.get("debug", model.get("id", reason)))
	if kind == "enum":
		accepted = model.variant
	elif kind == "entity":
		accepted = "Entity " + model.entity.bits
	elif kind == "node":
		accepted = ("Node " if model.valid else "Freed node ") + model.instance_id
	if type == TYPE_STRING:
		accepted = str(accepted)
	lookup[name] = {"property": property, "component": component, "path": path.duplicate(true),
		"model": model, "accepted": accepted, "label": label, "sections": sections,
		"reason": reason, "role": role, "copy_path": JSON.stringify(path)}
	_properties.append(property)

func attach_inspector(inspector: EditorInspector) -> void:
	for reference in _inspectors:
		if reference.get_ref() == inspector:
			return
	_inspectors.append(weakref(inspector))
	repair_sections.call_deferred()

func repair_sections() -> void:
	for reference in _inspectors.duplicate():
		var inspector = reference.get_ref()
		if inspector == null or inspector.get_edited_object() != self:
			_inspectors.erase(reference)
			continue
		_repair_children(inspector, 0)
	sections_repaired.emit()

func _repair_children(parent: Node, depth: int) -> void:
	for child in parent.get_children():
		if child.get_class() != "EditorInspectorSection":
			_repair_children(child, depth)
			continue
		var box: VBoxContainer = child.call("get_vbox")
		var entry := _first_entry(box)
		if not entry.is_empty() and depth < entry.sections.size() and not child.has_meta("bevy_section"):
			var section: Dictionary = entry.sections[depth]
			child.set_meta("bevy_section", section.key)
			child.call("setup", section.key, section.label, self,
				child.get_theme_color("prop_subsection", "Editor"), true, 0, 1)
			child.tooltip_text = section_tooltips[section.key]
		_repair_children(box, depth + 1)

func _first_entry(parent: Node) -> Dictionary:
	for child in parent.get_children():
		if child is EditorProperty:
			var name := String(child.get_edited_property())
			if lookup.has(name):
				return lookup[name]
		var nested := _first_entry(child.call("get_vbox") if child.get_class() == "EditorInspectorSection" else child)
		if not nested.is_empty():
			return nested
	return {}

func _refresh_property(name: String) -> void:
	for reference in _inspectors:
		var inspector = reference.get_ref()
		if inspector != null:
			inspector.call("_edit_request_change", self, name)

func invalidate() -> void:
	if detached:
		return
	detached = true
	if client != null:
		if client.active_session_changed.is_connected(_session_changed):
			client.active_session_changed.disconnect(_session_changed)
		if _read_id != -1:
			client.cancel_request(_read_id)
		for state in states.values():
			if state.request != -1:
				client.cancel_request(state.request)
	_read_id = -1
	status = "Select an entity in this session"
	for name in states:
		states[name].pending = false
		states[name].request = -1
		field_changed.emit(name)
	status_changed.emit()

func _hide_script_from_inspector() -> bool:
	return true

func _hide_metadata_from_inspector() -> bool:
	return true

func _dont_undo_redo() -> bool:
	return true
