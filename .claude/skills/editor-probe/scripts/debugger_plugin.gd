@tool
extends EditorPlugin

var cfg: Dictionary
var failures: Array = []
var pane
var client
var remote
var finished := false
var _timing_out := false
var checkpoints: Array = []

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
	checkpoints.append(step)
	print("EDITOR_PROBE shot=", path)
	print("EDITOR_PROBE checkpoint=", step)

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
	await get_tree().process_frame
	await get_tree().process_frame

func _has_component(section, suffix: String) -> bool:
	if section == null:
		return false
	for component in (section.proxy.components if section.proxy != null else section.component_editors):
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
	_check(pane.search_box.right_icon != null and pane.search_box.clear_button_enabled, "entity search uses editor icon and clear button")
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
	_check(pane.hidden_label.text == "%d internal hidden" % pane.hidden_count, "hidden count explicitly names internal entities")
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
	await _layout_checkpoints(section)
	var proxy = section.proxy
	if proxy == null:
		_finish(4)
		return
	var embedded = section.inspector.get_ref()
	var name := _property(proxy, "::Speed")
	var leaf = _property_row(embedded, name)
	_check(leaf != null and leaf.has_method("text_input"), "Speed exposes an editable numeric field")
	if leaf == null:
		_finish(4)
		return
	_check(leaf.label == "Speed" and proxy.lookup[name].sections.is_empty() and not name.contains("/"), "Speed is one component-labelled row without a tuple group")
	_check(proxy.has_method("_property_can_revert") and not proxy.property_can_revert(name), "Speed has no runtime default revert")
	await _reveal_inspector(leaf)
	var desired: float = proxy.get(name) + 25.0
	leaf.input.value = desired
	_check(proxy.states[name].pending and not leaf.input.editable and _binding(leaf).status.text == "Waiting for acknowledgement…" and _binding(leaf).status.is_visible_in_tree(), "pending edit disables its field in the submission frame")
	var request_id: int = proxy.states[name].request
	var response: Array = []
	var deliver: Callable = client._pending[request_id].callback
	# Hold only editor-side delivery so the pending screenshot cannot race the runtime.
	client._pending[request_id].callback = func(frame): response.append(frame)
	await _shot("5-pending-edit")
	if not await _wait(func(): return not response.is_empty(), "numeric edit acknowledgement"):
		_finish(4)
		return
	deliver.call(response[0])
	_check(not response[0].has("error") and proxy.get(name) == desired and leaf.input.value == desired, "acknowledged numeric value is shown")
	_check(not proxy.states[name].pending and leaf.input.editable and _binding(leaf).status.text == "Accepted", "accepted field re-enables and displays acknowledgement")
	await _shot("5-accepted-edit")
	var valid_path: Array = proxy.lookup[name].path.duplicate(true)
	proxy.lookup[name].path = [{"index": 999999}]
	leaf.input.value = desired + 1.0
	if not await _wait(func(): return not proxy.states[name].pending, "rejected edit acknowledgement"):
		_finish(4)
		return
	proxy.lookup[name].path = valid_path
	_check(proxy.states[name].rejected and proxy.states[name].status == "path shape changed", "rejection contains exact path-shape reason")
	_check(_binding(leaf).status.text == "path shape changed" and _binding(leaf).status.is_visible_in_tree() and _binding(leaf).warning.is_visible_in_tree() and _binding(leaf).warning.texture != null and leaf.input.value == desired, "rejection reason is present in the tree without changing accepted value")
	await _shot("6-rejected-edit")
	await _presentation_checkpoints()
	pane._proxy = pane.proxy_for(player.entity, client.active_session_id)
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
	for checkpoint in cfg.get("layout_checkpoints", []):
		_check(checkpoint in checkpoints, "missing layout checkpoint " + checkpoint)
	if not failures.is_empty() and code == 0:
		code = 4
	finished = true
	if EditorInterface.is_playing_scene():
		EditorInterface.stop_playing_scene()
		for i in 30:
			await get_tree().process_frame
	print("EDITOR_PROBE verdict=%d class=debugger property=Entities mismatches=%s" % [code, JSON.stringify(failures)])
	print("EDITOR_PROBE complete")
	get_tree().quit(code)

func _property(proxy, suffix: String, role: String = "value") -> String:
	for name in proxy.lookup:
		var entry: Dictionary = proxy.lookup[name]
		if entry.component.ends_with(suffix) and entry.role == role:
			return name
	_check(false, "missing mapped property " + suffix + " " + role)
	return ""

func _property_row(root: Node, property: String):
	for row in root.find_children("*", "EditorProperty", true, false):
		if String(row.get_edited_property()) == property:
			return row
	return null

func _unfold(root: Node, proxy) -> void:
	proxy.repair_sections()
	for section in root.find_children("*", "EditorInspectorSection", true, false):
		if section.has_meta("bevy_section"):
			section.unfold()

func _value_control(row: EditorProperty) -> Control:
	if row.has_method("text_input"):
		return row.input
	# Godot 4.6 creates left/right accessory containers before the value editor.
	for child in row.get_children().slice(2):
		if child is Control and child.visible and child.size.x > 0 and is_zero_approx(child.position.y):
			return child
	_check(false, "row has a value control " + String(row.get_edited_property()))
	return null

func _on_screen(control: Control) -> bool:
	return control.is_visible_in_tree() and control.get_global_rect().has_area() and control.get_global_rect().intersects(EditorInterface.get_inspector().get_global_rect())

func _native_row(inspector: EditorInspector) -> EditorProperty:
	for row in inspector.find_children("*", "EditorProperty", true, false):
		if row.get_edited_object() == inspector.get_edited_object() and not row.has_meta("bevy_binding") and row.is_visible_in_tree() and row.size.x > 0 and row.draw_label:
			return row
	return null

func _columns_aligned(bevy: Control, native: Control) -> bool:
	return bevy != null and native != null and bevy.size.x > 0 and native.size.x > 0 and absf(bevy.global_position.x - native.global_position.x) <= 1.0

func _binding(row: EditorProperty):
	return row.get_meta("bevy_binding").get_ref()

func _geometry(control: Control) -> Dictionary:
	var rect := control.get_global_rect()
	var result := {"class": control.get_class(), "name": String(control.name),
		"x": rect.position.x, "y": rect.position.y, "width": rect.size.x, "height": rect.size.y,
		"visible": control.is_visible_in_tree()}
	if control.get_parent() is Control:
		var parent_rect: Rect2 = control.get_parent().get_global_rect()
		result.left_inset = rect.position.x - parent_rect.position.x
		result.right_inset = parent_rect.end.x - rect.end.x
	if control is EditorProperty:
		result.property = String(control.get_edited_property())
		result.name_split_ratio = control.name_split_ratio
		result.calculated_value_x = rect.end.x - int(rect.size.x * (1.0 - control.name_split_ratio))
	if control is BoxContainer:
		result.separation = control.get_theme_constant("separation")
	if control is ScrollContainer or control is PanelContainer:
		var panel := control.get_theme_stylebox("panel")
		result.panel_margins = [panel.get_content_margin(SIDE_LEFT), panel.get_content_margin(SIDE_RIGHT)]
	return result

func _geometry_chain(control: Control, outer: EditorInspector) -> Array:
	var result: Array = []
	var ancestor: Node = control
	while ancestor != null:
		if ancestor is Control:
			result.append(_geometry(ancestor))
		if ancestor == outer:
			break
		ancestor = ancestor.get_parent()
	return result

func _column_checkpoint(section, native: EditorProperty, step: String) -> void:
	var outer := EditorInterface.get_inspector()
	var embedded: EditorInspector = section.inspector.get_ref()
	var identity: EditorProperty = _property_row(embedded, "@entity")
	_check(identity != null and native != null, step + ": native and Bevy reference rows exist")
	if identity == null or native == null:
		return
	await _reveal_inspector(identity)
	_check(outer.is_ancestor_of(identity) and outer.is_ancestor_of(native) and identity.get_viewport() == native.get_viewport(), step + ": reference rows share the outer Inspector and viewport")
	_check(_on_screen(identity), step + ": Bevy identity is on screen after scrolling settles")
	var children: Array = []
	for child in section.get_children():
		if child is Control:
			children.append(_geometry(child))
	var reference := String(native.get_edited_property())
	var native_value: Control = _value_control(native)
	for property in ["@entity", _property(section.proxy, "::Speed")]:
		var row: EditorProperty = _property_row(embedded, property)
		_check(row != null, step + ": Bevy column row exists " + property)
		if row == null:
			continue
		var value: Control = _value_control(row)
		_check(_columns_aligned(value, native_value), step + ": Bevy " + property + " value-column x equals native row " + reference + " (within one pixel)")
		if value == null or native_value == null:
			continue
		print("EDITOR_PROBE column_geometry=", JSON.stringify({"checkpoint": step,
			"native": _geometry_chain(native_value, outer), "bevy": _geometry_chain(value, outer),
			"section_children": children, "offset": value.global_position.x - native_value.global_position.x}))
		var original_x: float = value.position.x
		value.position.x = native_value.global_position.x - value.get_parent().global_position.x + 8.0
		_check(not _columns_aligned(value, native_value), step + ": column assertion rejects an eight-pixel offset for " + property)
		value.position.x = original_x
	await _shot(step)

func _dock_split(inspector: EditorInspector) -> Dictionary:
	var child: Node = inspector
	while child.get_parent() != null:
		var parent := child.get_parent()
		if parent is SplitContainer and not parent.vertical:
			var controls: Array = []
			for sibling in parent.get_children():
				if sibling is Control and sibling.is_visible_in_tree():
					controls.append(sibling)
			var index: int = controls.find(child)
			if controls.size() > 1 and index >= 0:
				return {"control": parent, "index": maxi(0, index - 1), "direction": 1 if index == 0 else -1}
		child = parent
	return {}

func _layout_checkpoints(section) -> void:
	_check(section.inspector != null, "node-backed section contains native Inspector")
	if section.inspector == null:
		return
	var embedded = section.inspector.get_ref()
	var proxy = section.proxy
	_check(embedded.horizontal_scroll_mode == ScrollContainer.SCROLL_MODE_DISABLED and embedded.vertical_scroll_mode == ScrollContainer.SCROLL_MODE_DISABLED, "embedded Inspector sizes to content on both axes")
	_unfold(embedded, proxy)
	for i in 4:
		await get_tree().process_frame
	var outer := EditorInterface.get_inspector()
	var native: EditorProperty = _native_row(outer)
	await _column_checkpoint(section, native, "8-node-backed-open")
	if native == null:
		return
	var split := _dock_split(outer)
	_check(not split.is_empty(), "Inspector dock has a horizontal split to resize")
	if split.is_empty():
		return
	var identity: EditorProperty = _property_row(embedded, "@entity")
	if identity == null:
		return
	var row_id: int = identity.get_instance_id()
	var inspector_id: int = embedded.get_instance_id()
	var default_width: float = outer.size.x
	var offsets: PackedInt32Array = split.control.get_split_offsets()
	var wide_offsets := offsets.duplicate()
	wide_offsets[split.index] += split.direction * 180
	split.control.set_split_offsets(wide_offsets)
	for i in 4:
		await get_tree().process_frame
	_check(outer.size.x >= default_width + 100.0, "wide checkpoint actually widens the Inspector by at least 100 pixels")
	await _column_checkpoint(section, native, "8-node-backed-wide")
	split.control.set_split_offsets(offsets)
	for i in 4:
		await get_tree().process_frame
	_check(absf(outer.size.x - default_width) <= 1.0, "narrow checkpoint restores the default Inspector width")
	await _column_checkpoint(section, native, "8-node-backed-narrow")
	var current: EditorInspector = section.inspector.get_ref()
	var current_identity: EditorProperty = _property_row(current, "@entity")
	_check(current.get_instance_id() == inspector_id and current_identity != null and current_identity.get_instance_id() == row_id, "dock resize aligns existing rows without rebuilding the embedded Inspector")
	var group: Node = native.get_parent()
	while group != outer and group.get_class() != "EditorInspectorSection":
		group = group.get_parent()
	_check(group != outer, "native reference has a group to fold")
	if group != outer:
		group.call("fold")
		for i in 4:
			await get_tree().process_frame
		var replacement: EditorProperty = _native_row(outer)
		if native.is_visible_in_tree():
			# Godot leaves the remote object's sections unfoldable, so this scenario cannot be
			# reached from the editor either; the resize checkpoints above cover realignment.
			# The manifest does not require 8-native-reference-folded for this reason.
			print("EDITOR_PROBE skipped=native-reference-fold reason=remote sections are not foldable")
		else:
			_check(replacement != null and replacement != native, "folding the native reference selects a different visible row")
			await _column_checkpoint(section, replacement, "8-native-reference-folded")
			group.call("unfold")
			for i in 4:
				await get_tree().process_frame
			_check(_columns_aligned(_value_control(current_identity), _value_control(native)), "unfolding the native reference restores its value column")

func _presentation_checkpoints() -> void:
	var pure: Dictionary = {}
	for row in pane.rows.values():
		if not row.has_node and not row.components.is_empty():
			pure = row
			break
	_check(not pure.is_empty(), "runtime has a pure-ECS entity for selection checkpoint")
	if pure.is_empty():
		return
	pane.search_box.clear()
	pane.search_box.text_changed.emit("")
	pane.show_internal.button_pressed = true
	pane.select_entity(pure.entity)
	if not await _wait(func(): return pane._proxy != null and pane._proxy.entity == pure.entity and not pane._proxy.components.is_empty(), "pure-ECS proxy properties"):
		return
	_check(EditorInterface.get_inspector().get_edited_object() == pane._proxy, "pure-ECS entity uses the main Inspector proxy")
	_unfold(EditorInterface.get_inspector(), pane._proxy)
	await _shot("9-pure-ecs")
	var Fixtures = preload("res://addons/godot-bevy/test/bevy_inspector_fixtures.gd")
	var Proxy = preload("res://addons/godot-bevy/bevy_entity_proxy.gd")
	var Tests = preload("res://addons/godot-bevy/test/bevy_inspector_tests.gd")
	var fixture_client = Tests.FakeClient.new()
	var fixture = Proxy.new(Fixtures.entity_ref("presentation fixture"), 0)
	fixture.configure(fixture_client)
	fixture.accept_components(Fixtures.presentation())
	EditorInterface.inspect_object(fixture)
	fixture.attach_inspector(EditorInterface.get_inspector())
	await get_tree().process_frame
	await get_tree().process_frame
	_unfold(EditorInterface.get_inspector(), fixture)
	await get_tree().process_frame
	for row in EditorInterface.get_inspector().find_children("*", "EditorProperty", true, false):
		if row.has_meta("bevy_binding"):
			_check(not row.label in ["Type", "Whole value"], "component metadata does not create property rows: " + row.label)
	var generic := "bevy_state::state::resources::PreviousState<platformer_2d_example::GameState>"
	var generic_group := false
	for section in EditorInterface.get_inspector().find_children("*", "EditorInspectorSection", true, false):
		if section.get_meta("bevy_section", "") == "component_" + generic.uri_encode() + "/":
			generic_group = section.tooltip_text == generic
	_check(generic_group, "PreviousState<GameState> group keeps the full qualified type in its tooltip")
	var chain: Array = []
	var prefix := "component_" + "game::VeryLongComponentNameThatMustRemainReadableInANarrowInspector".uri_encode() + "/"
	for key in [prefix, prefix + "one/", prefix + "one/two/"]:
		for section in EditorInterface.get_inspector().find_children("*", "EditorInspectorSection", true, false):
			if section.get_meta("bevy_section", "") == key:
				chain.append(section)
	_check(chain.size() == 3, "component and two nested sections exist")
	if chain.size() == 3:
		await _reveal_inspector(chain[0])
		var first: float = chain[1].global_position.x - chain[0].global_position.x
		var second: float = chain[2].global_position.x - chain[1].global_position.x
		_check(first > 0 and second > 0 and is_equal_approx(first, second), "each nesting level has a non-zero consistent inset")
		chain[2].fold()
		_check(not chain[2].get_vbox().visible, "fold state is readable and fold is not a no-op")
		chain[2].unfold()
		_check(chain[2].get_vbox().visible, "unfold changes child visibility")
		chain[2].fold()
		var same_section: int = chain[2].get_instance_id()
		var values: Dictionary = Fixtures.presentation()
		values["game::Range"].value = 0.25
		fixture.accept_components(values)
		await get_tree().process_frame
		_check(fixture.get(_property(fixture, "::Range")) == 0.25 and is_instance_valid(chain[2]) and EditorInterface.get_inspector().is_ancestor_of(chain[2]) and chain[2].get_instance_id() == same_section and not chain[2].get_vbox().visible, "fold survives a value refresh without replacing rows")
		if is_instance_valid(chain[2]):
			chain[2].unfold()
	await _shot("10-nesting-two-levels-long-name")
	var readonly = _property_row(EditorInterface.get_inspector(), _property(fixture, "::ReadOnly"))
	var unsupported = _property_row(EditorInterface.get_inspector(), _property(fixture, "::Unsupported"))
	_check(readonly != null and unsupported != null, "read-only and unsupported rows exist")
	if readonly != null and unsupported != null:
		await _reveal_inspector(readonly)
		_check(not readonly.input.editable and _binding(readonly).explanation.text == "InspectorReadOnly" and _binding(readonly).explanation.is_visible_in_tree(), "read-only field is disabled with visible explanation")
		await _shot("11-read-only")
		await _reveal_inspector(unsupported)
		_check(unsupported.input.text == "component not registered" and _on_screen(unsupported.input), "unsupported reason is present in the tree")
	await _shot("11-unsupported")
	for suffix in ["::Bool", "::UnitEnum"]:
		var stock_name := _property(fixture, suffix)
		var stock_row = _property_row(EditorInterface.get_inspector(), stock_name)
		_check(stock_row != null and not stock_row.has_method("text_input"), "factory supplies stock leaf editor " + suffix)
		if stock_row == null:
			continue
		stock_row.emit_changed(stock_name, false if suffix == "::Bool" else "Idle")
		_check(fixture.states[stock_name].pending and stock_row.is_read_only(), "stock leaf disables immediately while pending " + suffix)
		fixture_client.reply(fixture.states[stock_name].request, {"error": {"message": "fixture rejection"}})
		_check(_binding(stock_row).status.text == "fixture rejection" and _binding(stock_row).status.is_visible_in_tree() and not stock_row.is_read_only(), "stock leaf shows rejection and re-enables " + suffix)
	var number_name := _property(fixture, "::Range")
	var number_row = _property_row(EditorInterface.get_inspector(), number_name)
	_check(number_row != null, "unquantized numeric row exists")
	if number_row != null:
		await _reveal_inspector(number_row)
		var number_line: LineEdit = number_row.text_input()
		number_line.grab_focus()
		var before: int = fixture_client.frames.size()
		number_line.text = "0.001"
		number_line.text_submitted.emit(number_line.text)
		if await _wait(func(): return fixture.states[number_name].pending and fixture.states[number_name].request > before, "typed numeric mutation"):
			var mutation: Dictionary = fixture_client.last_frame("godot.mutate_leaf")
			_check(mutation.get("id", -1) == fixture.states[number_name].request and mutation.get("params", {}) == {"entity": fixture.entity, "component": "game::Range", "path": [], "value": 0.001}, "typed off-grid number is submitted without quantization")
			fixture_client.reply(fixture.states[number_name].request, {"result": Fixtures.presentation()["game::Range"]})
			_check(number_row.input.value == 0.125, "native numeric row displays exact acknowledged off-grid value")
		else:
			return
	var text_name := _property(fixture, "::Text")
	var text_row = _property_row(EditorInterface.get_inspector(), text_name)
	_check(text_row != null, "commit-on-Enter text row exists")
	if text_row != null:
		await _reveal_inspector(text_row)
		var line: LineEdit = text_row.text_input()
		line.grab_focus()
		line.text = "half typed"
		line.text_changed.emit(line.text)
		var row_id: int = text_row.get_instance_id()
		var values: Dictionary = Fixtures.presentation()
		values["game::Range"].value = 0.5
		fixture.accept_components(values)
		await get_tree().process_frame
		_check(is_instance_valid(text_row) and _property_row(EditorInterface.get_inspector(), text_name) == text_row and text_row.get_instance_id() == row_id and line.has_focus() and line.text == "half typed" and fixture.get(text_name) == "accepted text", "value refresh retains focused candidate without committing")
		var escape := InputEventKey.new()
		escape.keycode = KEY_ESCAPE
		escape.pressed = true
		line.gui_input.emit(escape)
		_check(line.text == "accepted text" and not fixture.states[text_name].dirty, "Escape restores accepted text")
		var before: int = fixture_client.frames.size()
		line.text = "commit with Enter"
		line.text_changed.emit(line.text)
		_check(fixture_client.frames.size() == before and fixture.get(text_name) == "accepted text", "typing text does not mutate accepted value")
		line.text_submitted.emit(line.text)
		var mutation: Dictionary = fixture_client.last_frame("godot.mutate_leaf")
		_check(fixture_client.frames.size() == before + 1 and mutation.get("id", -1) == fixture.states[text_name].request and mutation.get("params", {}) == {"entity": fixture.entity, "component": "game::Text", "path": [], "value": "commit with Enter"} and not line.editable and line.text == "accepted text", "Enter submits text once and restores accepted display while pending")
		fixture_client.reply(fixture.states[text_name].request, {"result": Fixtures.scalar("string", "runtime accepted")})
		_check(line.text == "runtime accepted" and line.editable, "text acknowledgement replaces focused input from response")
		line.text = "candidate across rebuild"
		line.text_changed.emit(line.text)
		line.caret_column = 6
		values["game::Range"].writable = {"allowed": false, "reason": "InspectorReadOnly"}
		var folded_id := -1
		if chain.size() == 3 and is_instance_valid(chain[2]):
			chain[2].fold()
			folded_id = chain[2].get_instance_id()
		fixture.accept_components(values)
		await get_tree().process_frame
		await get_tree().process_frame
		var folded_after_rebuild := false
		for section in EditorInterface.get_inspector().find_children("*", "EditorInspectorSection", true, false):
			if section.get_meta("bevy_section", "") == prefix + "one/two/":
				folded_after_rebuild = folded_id != -1 and section.get_instance_id() != folded_id and not section.get_vbox().visible
		_check(folded_after_rebuild, "section setup restores proxy fold state after a permission rebuild")
		_unfold(EditorInterface.get_inspector(), fixture)
		text_row = _property_row(EditorInterface.get_inspector(), text_name)
		_check(text_row != null and text_row.get_instance_id() != row_id and text_row.text_input().text == "candidate across rebuild" and text_row.text_input().has_focus() and text_row.text_input().caret_column == 6 and fixture.selected_property == text_name, "proxy owns candidate, focus, caret and selection across a permission rebuild")
	await _shot("12-text-candidate")
	EditorInterface.inspect_object(null)
	fixture.invalidate()
