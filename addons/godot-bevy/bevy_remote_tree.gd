@tool
extends Node

signal selection_changed(instance_ids, session_id)
signal availability_changed(reason)

var tree: Tree
var pane: Control
var last_ids: Array = []
var last_session := -1
var reason := "Remote sync not available on this Godot version"
var _signal_name := ""
var _selection_serial := 0
var _previous_root_id := 0

func _ready() -> void:
	attach()

func attach() -> void:
	if is_instance_valid(tree):
		if not _signal_name.is_empty() and tree.is_connected(_signal_name, _on_many if _signal_name == "objects_selected" else _on_one):
			reason = ""
		return
	var found = EditorInterface.get_base_control().find_children("*", "EditorDebuggerTree", true, false)
	if found.is_empty():
		_unavailable("EditorDebuggerTree missing")
		return
	tree = found[0] as Tree
	_signal_name = "objects_selected" if ClassDB.class_exists("EditorDebuggerRemoteObjects") else "object_selected"
	if not tree or not tree.has_signal(_signal_name):
		_unavailable("Remote selection signal missing")
		return
	tree.connect(_signal_name, _on_many if _signal_name == "objects_selected" else _on_one)
	reason = ""
	availability_changed.emit(reason)

func _unavailable(detail: String) -> void:
	reason = "Remote sync not available on this Godot version: " + detail
	availability_changed.emit(reason)

func _on_many(ids: Array, debugger: int) -> void:
	last_ids = []
	for id in ids:
		last_ids.append(str(id))
	last_session = debugger
	selection_changed.emit(last_ids, debugger)

func _on_one(id: int, debugger: int) -> void:
	_on_many([id], debugger)

func cancel_selection() -> void:
	_selection_serial += 1

func clear_selection() -> void:
	cancel_selection()
	last_ids.clear()
	last_session = -1

func _show_remote(session_id: int) -> bool:
	var base = EditorInterface.get_base_control()
	# Session indices are the ScriptEditorDebugger tabs in EditorDebuggerNode.
	var debuggers = base.find_children("*", "ScriptEditorDebugger", true, false)
	if session_id < 0 or session_id >= debuggers.size():
		_unavailable("debugger session tab missing")
		return false
	var debugger = debuggers[session_id]
	_previous_root_id = 0
	if debugger.get_parent() is TabContainer:
		var tabs: TabContainer = debugger.get_parent()
		if is_instance_valid(pane) and tabs.is_ancestor_of(pane):
			_unavailable("debugger session shares a tab container with Entities")
			return false
		var tab_index := tabs.get_tab_idx_from_control(debugger)
		if (tabs.current_tab != tab_index or not tree.is_visible_in_tree()) and tree.get_root() != null:
			_previous_root_id = tree.get_root().get_instance_id()
		tabs.current_tab = tab_index
	elif debuggers.size() > 1:
		_unavailable("debugger session selector missing")
		return false
	var docks = base.find_children("*", "SceneTreeDock", true, false)
	for dock in docks:
		if not _show_dock(dock):
			return false
		for button in dock.find_children("*", "Button", true, false):
			if button.text == "Remote" or button.text == tr("Remote"):
				button.emit_signal("pressed")
				return true
	_unavailable("Scene dock Remote button missing")
	return false

func _show_dock(dock: Node) -> bool:
	var ancestor: Node = dock
	while ancestor.get_parent() != null:
		var parent = ancestor.get_parent()
		if parent is TabContainer and is_instance_valid(pane) and parent.is_ancestor_of(pane):
			_unavailable("Scene dock shares a tab container with Entities; move Entities to a separate dock")
			return false
		ancestor = parent
	ancestor = dock
	while ancestor.get_parent() != null:
		if ClassDB.class_exists("EditorDebuggerRemoteObjects") and ancestor.has_method("make_visible"):
			ancestor.call("make_visible")
		var parent = ancestor.get_parent()
		if parent is TabContainer:
			parent.current_tab = parent.get_tab_idx_from_control(ancestor)
		ancestor = parent
	return true

func find_item(instance_id: String, item: TreeItem = null) -> TreeItem:
	if not is_instance_valid(tree):
		return null
	if item == null:
		item = tree.get_root()
	if item == null:
		return null
	if str(item.get_metadata(0)) == instance_id:
		return item
	var child = item.get_first_child()
	while child != null:
		var found = find_item(instance_id, child)
		if found != null:
			return found
		child = child.get_next()
	return null

func node_path(instance_id: String) -> String:
	var item = find_item(instance_id)
	if item == null:
		return ""
	if not item.has_meta("node_path"):
		_unavailable("node path metadata missing on this Godot version")
		return ""
	return str(item.get_meta("node_path"))

func select_node(instance_id: String, session_id: int) -> bool:
	attach()
	if not reason.is_empty() or not _show_remote(session_id):
		return false
	_selection_serial += 1
	var serial := _selection_serial
	var deadline := Time.get_ticks_msec() + 10000
	while is_inside_tree() and serial == _selection_serial and Time.get_ticks_msec() < deadline:
		var root = tree.get_root()
		if root != null and root.get_instance_id() == _previous_root_id:
			await get_tree().process_frame
			continue
		var item = find_item(instance_id)
		if item != null and tree.is_visible_in_tree():
			item.uncollapse_tree()
			tree.deselect_all()
			tree.set_selected(item, 0)
			tree.scroll_to_item(item)
			reason = ""
			availability_changed.emit(reason)
			return true
		await get_tree().process_frame
	if serial == _selection_serial:
		reason = "Remote node not available: tree did not populate or node was freed"
		availability_changed.emit(reason)
	return false

func _exit_tree() -> void:
	clear_selection()
	if is_instance_valid(tree) and tree.has_signal(_signal_name):
		var callback = _on_many if _signal_name == "objects_selected" else _on_one
		if tree.is_connected(_signal_name, callback):
			tree.disconnect(_signal_name, callback)
