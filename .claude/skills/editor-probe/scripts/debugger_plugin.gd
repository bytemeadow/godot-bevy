@tool
extends EditorPlugin

var cfg: Dictionary
var failures: Array = []
var pane
var client
var remote
var finished := false
var _timing_out := false

func _enter_tree() -> void:
	cfg = JSON.parse_string(FileAccess.get_file_as_string("res://addons/editor_probe/probe.json"))[0]
	_run.call_deferred()
	get_tree().create_timer(180.0).timeout.connect(func():
		if not finished:
			_timeout("debugger probe watchdog")
	)

func _wait(predicate: Callable, label: String, seconds: float = 20.0) -> bool:
	var force_timeout: bool = cfg.get("timeout_checkpoint", "") == label
	var deadline := Time.get_ticks_msec() + (0 if force_timeout else int(seconds * 1000.0))
	while Time.get_ticks_msec() < deadline and not finished:
		if predicate.call():
			return true
		await get_tree().process_frame
	if not finished:
		await _timeout(label)
	return false

func _timeout(label: String) -> void:
	if finished or _timing_out:
		return
	_timing_out = true
	failures.append("timeout: " + label)
	var diagnostics: Dictionary = client.diagnostics() if client != null else {"pending": {}, "last_frames": []}
	print("EDITOR_PROBE timeout_diagnostics=", JSON.stringify(diagnostics))
	if is_instance_valid(pane):
		_show(pane)
	await _shot("timeout-" + label.validate_filename())
	_finish(4)

func _check(condition: bool, assertion: String) -> void:
	if not condition:
		failures.append(assertion)

func _show(control: Node) -> void:
	var ancestor := control
	while ancestor.get_parent() != null:
		if ancestor.has_method("make_visible"):
			ancestor.call("make_visible")
		if ancestor.get_parent() is TabContainer:
			ancestor.get_parent().current_tab = ancestor.get_index()
		ancestor = ancestor.get_parent()

func _shot(step: String) -> void:
	await get_tree().process_frame
	await get_tree().process_frame
	RenderingServer.force_draw()
	var viewport = EditorInterface.get_base_control().get_viewport()
	DirAccess.make_dir_recursive_absolute(cfg.shots)
	var path: String = cfg.shots.path_join(step + ".png")
	_check(viewport.get_texture().get_image().save_png(path) == OK, "save PNG " + step)
	print("EDITOR_PROBE shot=", path)

func _row(name_part: String) -> Dictionary:
	# Exact names first: "Player" also matches "@AudioStreamPlayer@3".
	for row in pane.rows.values():
		if row.name == name_part:
			return row
	for row in pane.rows.values():
		if row.name.contains(name_part):
			return row
	return {}

func _remote_item(name: String) -> TreeItem:
	var stack: Array = [remote.tree.get_root()]
	while not stack.is_empty():
		var item = stack.pop_back()
		if item == null:
			continue
		if item.get_text(0) == name:
			return item
		stack.append_array(item.get_children())
	return null

func _section():
	for node in EditorInterface.get_inspector().find_children("*", "VBoxContainer", true, false):
		if node.get_script() == load("res://addons/godot-bevy/bevy_inspector_section.gd") and node.is_visible_in_tree():
			return node
	return null

func _reveal_inspector(control: Control) -> void:
	await get_tree().process_frame
	await get_tree().process_frame
	var ancestor := control.get_parent()
	while ancestor != null:
		if ancestor is ScrollContainer:
			ancestor.ensure_control_visible(control)
		ancestor = ancestor.get_parent()

func _has_component(section, suffix: String) -> bool:
	if section == null:
		return false
	for component in section.component_editors:
		if component.ends_with(suffix):
			return true
	return false

func _run() -> void:
	await get_tree().process_frame
	for node in EditorInterface.get_base_control().find_children("Entities", "Panel", true, false):
		if node.get_script() == load("res://addons/godot-bevy/bevy_inspector_panel.gd"):
			pane = node
	if pane == null:
		failures.append("Entities pane missing")
		_finish(4)
		return
	client = pane.client
	remote = pane.remote
	_show(pane)
	EditorInterface.play_main_scene()
	if not await _wait(func(): return client.active_session_id >= 0, "active debugger session", 60.0):
		_finish(4)
		return
	_check(pane.session_selector.get_selected_id() == client.active_session_id, "pane picks started session")
	if not await _wait(func(): return not _row("MainMenu").is_empty(), "MainMenu summary"):
		_finish(4)
		return
	_check(pane.hidden_count > 0 and not pane.show_internal.button_pressed, "observers hidden by default")
	for row in pane.rows.values():
		if pane.internal(row):
			_check(not pane.items[row.entity.bits].visible, "internal row stays hidden")
	await _shot("1-connected-dock")
	var menu = _row("MainMenu")
	pane.entity_tree.set_selected(pane.items[menu.entity.bits], 0)
	if not await _wait(func(): return remote.tree != null and remote.tree.get_selected() != null and remote.tree.get_selected().get_text(0) == "MainMenu", "pane selects MainMenu in Remote"):
		_finish(4)
		return
	if not await _wait(func(): return _has_component(_section(), "::GodotNodeHandle"), "Inspector Bevy section contains GodotNodeHandle"):
		_finish(4)
		return
	_check(_has_component(_section(), "::Node2DMarker"), "MainMenu Bevy section contains Node2DMarker")
	_check(pane.entity_tree.get_selected().get_metadata(0) == menu.entity.bits, "pane selection survives its Remote echo")
	_check(pane.is_visible_in_tree() and pane._subscribed_session == client.active_session_id, "pane selection keeps Entities visible and subscribed")
	await _reveal_inspector(_section().get_child(0))
	await _shot("2-main-menu-inspector")
	var singleton = _remote_item("BevyAppSingleton")
	if singleton == null:
		failures.append("BevyAppSingleton Remote row missing")
		_finish(4)
		return
	remote.tree.set_selected(singleton, 0)
	if not await _wait(func(): return pane.selected_entity.get("bits", "") == _row("BevyAppSingleton").get("entity", {}).get("bits", "missing"), "Remote selects BevyAppSingleton in pane"):
		_finish(4)
		return
	_check(pane.is_visible_in_tree(), "independent Remote selection keeps Entities visible")
	_check(pane.entity_tree.get_selected().get_metadata(0) == pane.selected_entity.bits, "independent Remote selection highlights pane row")
	await _shot("2b-remote-to-pane")
	await _shot("3-main-menu-dock")
	EditorInterface.open_scene_from_path("res://scenes/levels/main_menu.tscn")
	if not await _wait(func(): return EditorInterface.get_edited_scene_root() != null and EditorInterface.get_edited_scene_root().scene_file_path == "res://scenes/levels/main_menu.tscn", "local MainMenu scene opens"):
		_finish(4)
		return
	var local_menu = EditorInterface.get_edited_scene_root()
	var local_selection = EditorInterface.get_selection()
	local_selection.clear()
	EditorInterface.edit_node(local_menu)
	local_selection.add_node(local_menu)
	if not await _wait(func(): return pane.selected_entity == menu.entity, "local selection maps to MainMenu entity"):
		_finish(4)
		return
	for i in 30:
		await get_tree().process_frame
	_check(EditorInterface.get_inspector().get_edited_object() == local_menu, "local Scene selection retains local Inspector object")
	_check(pane.is_visible_in_tree() and pane._subscribed_session == client.active_session_id, "local selection retains Entities subscription")
	await _shot("3b-local-selection")
	client.sessions[client.active_session_id].session.send_message("entity_probe:enter_level", [])
	if not await _wait(func(): return not _row("Player").is_empty(), "enter level and receive Player", 40.0):
		_finish(4)
		return
	pane.search_box.text = "Player"
	pane.search_box.text_changed.emit("Player")
	var player = _row("Player")
	_check(pane.items[player.entity.bits].visible, "search finds Player")
	_check(not pane.items[menu.entity.bits].visible if pane.items.has(menu.entity.bits) else true, "search removes unrelated MainMenu")
	await _shot("4-player-search")
	pane.entity_tree.set_selected(pane.items[player.entity.bits], 0)
	if not await _wait(func(): return _has_component(_section(), "::Speed"), "Player Inspector contains Speed"):
		_finish(4)
		return
	var section = _section()
	var component = ""
	for type_path in section.component_editors:
		if type_path.ends_with("::Speed"):
			component = type_path
	var speed = section.component_editors[component]
	speed.fold.button_pressed = true
	var leaf = speed.children_editors[0]
	_check(leaf.editor is SpinBox, "Speed uses SpinBox")
	var accepted: Array = []
	leaf.edit_finished.connect(func(ok, reason): accepted.append([ok, reason]))
	var desired: float = leaf.model.value + 25.0
	leaf.editor.value = desired
	if not await _wait(func(): return not accepted.is_empty(), "numeric edit acknowledgement"):
		_finish(4)
		return
	_check(accepted[0][0] and leaf.editor.value == desired, "acknowledged numeric value is shown")
	await _reveal_inspector(leaf)
	await _shot("5-accepted-edit")
	# Force a stale path after rendering to exercise the real rejected-edit presentation.
	var valid_path: Array = leaf.path.duplicate(true)
	leaf.path = [{"index": 999999}]
	leaf.editor.value = desired + 1.0
	if not await _wait(func(): return accepted.size() == 2, "rejected edit acknowledgement"):
		_finish(4)
		return
	leaf.path = valid_path
	_check(not accepted[1][0] and accepted[1][1] == "path shape changed", "rejection contains exact path-shape reason")
	_check(leaf.error_label.text == accepted[1][1] and leaf.editor.value == desired, "rejection shown inline without changing accepted value")
	await _shot("6-rejected-edit")
	var Proxy = preload("res://addons/godot-bevy/bevy_entity_proxy.gd")
	pane._proxy = Proxy.new(player.entity, client.active_session_id)
	EditorInterface.inspect_object(pane._proxy)
	_check(EditorInterface.get_inspector().get_edited_object() == pane._proxy, "proxy is inspected before shutdown")
	pane.shutdown()
	_check(EditorInterface.get_inspector().get_edited_object() == null and pane._proxy == null, "shutdown clears Inspector before releasing proxy")
	EditorInterface.inspect_object(local_menu)
	pane.shutdown()
	_check(EditorInterface.get_inspector().get_edited_object() == local_menu, "shutdown preserves unrelated Inspector object")
	EditorInterface.stop_playing_scene()
	if not await _wait(func(): return client.active_session_id == -1, "session stops"):
		_finish(4)
		return
	_show(pane)
	_check(pane.status_label.text == "no running session", "stop shows disconnected state")
	await _shot("7-disconnected")
	_finish(0 if failures.is_empty() else 4)

func _finish(code: int) -> void:
	if finished:
		return
	finished = true
	if EditorInterface.is_playing_scene():
		EditorInterface.stop_playing_scene()
		for i in 30:
			await get_tree().process_frame
	print("EDITOR_PROBE verdict=%d class=debugger property=Entities mismatches=%s" % [code, JSON.stringify(failures)])
	print("EDITOR_PROBE complete")
	get_tree().quit(code)
