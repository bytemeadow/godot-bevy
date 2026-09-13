@tool
extends Panel

const Proxy = preload("res://addons/godot-bevy/bevy_entity_proxy.gd")

signal entity_selected(reference)

var client
var remote
var entity_tree: Tree
var status_label: Label
var hidden_label: Label
var remote_status: Label
var session_selector: OptionButton
var search_box: LineEdit
var show_internal: CheckBox
var rows: Dictionary = {}
var items: Dictionary = {}
var hidden_count := 0
var selected_entity: Dictionary = {}
var _root: TreeItem
var _subscribed_session := -1
var _search_expansion: Dictionary = {}
var _searching := false
var _candidates: OptionButton
var _search_serial := 0
var _search_matches: Dictionary = {}
var _query_ids: Array = []
var _selection_serial := 0
var _selecting := false
var _proxy: RefCounted
var proxies: Dictionary = {}
var _remote_selection_echoes: Dictionary = {}
var _snapshot_rows: Dictionary = {}
var _snapshot_deltas: Array = []
var _snapshot_index := 0
var _pending_selection: Dictionary = {}

func _ready() -> void:
	custom_minimum_size = Vector2(240, 220)
	var box := VBoxContainer.new()
	box.set_anchors_and_offsets_preset(Control.PRESET_FULL_RECT)
	add_child(box)
	session_selector = OptionButton.new()
	session_selector.item_selected.connect(_choose_session)
	box.add_child(session_selector)
	status_label = Label.new()
	status_label.text = "no running session"
	box.add_child(status_label)
	_candidates = OptionButton.new()
	_candidates.hide()
	_candidates.item_selected.connect(func(index): select_entity(_candidates.get_item_metadata(index)))
	box.add_child(_candidates)
	search_box = LineEdit.new()
	search_box.placeholder_text = "Filter entities"
	search_box.tooltip_text = "Search names, decimal entity IDs, component type paths or exact runtime node paths"
	search_box.clear_button_enabled = true
	if Engine.is_editor_hint():
		search_box.right_icon = EditorInterface.get_editor_theme().get_icon("Search", "EditorIcons")
	search_box.text_changed.connect(_search)
	box.add_child(search_box)
	var filters := HBoxContainer.new()
	box.add_child(filters)
	hidden_label = Label.new()
	filters.add_child(hidden_label)
	show_internal = CheckBox.new()
	show_internal.text = "Show internal"
	show_internal.toggled.connect(func(_value): _filter())
	filters.add_child(show_internal)
	entity_tree = Tree.new()
	entity_tree.hide_root = true
	entity_tree.size_flags_vertical = Control.SIZE_EXPAND_FILL
	entity_tree.item_selected.connect(_on_selected)
	box.add_child(entity_tree)
	_root = entity_tree.create_item()
	remote_status = Label.new()
	remote_status.autowrap_mode = TextServer.AUTOWRAP_WORD_SMART
	box.add_child(remote_status)
	visibility_changed.connect(_subscription_visibility)
	_filter()

func setup(debugger, adapter) -> void:
	client = debugger
	remote = adapter
	client.sessions_changed.connect(_sessions_changed)
	client.active_session_changed.connect(_session_changed)
	client.summary_received.connect(_summary)
	client.interval_changed.connect(_interval_changed)
	client.connection_error.connect(func(reason): status_label.text = reason)
	remote.selection_changed.connect(_remote_selected)
	remote.availability_changed.connect(func(reason): remote_status.text = reason)
	remote_status.text = remote.reason
	_sessions_changed()
	_session_changed(client.active_session_id)

func _sessions_changed() -> void:
	for key in proxies.keys():
		if not proxies[key].valid_target():
			proxies[key].invalidate()
			proxies.erase(key)
	session_selector.clear()
	for id in client.sessions:
		if client.is_session_active(id):
			session_selector.add_item("Session %d" % (id + 1), id)
			if id == client.active_session_id:
				session_selector.select(session_selector.item_count - 1)
	session_selector.disabled = session_selector.item_count == 0
	if session_selector.disabled:
		session_selector.add_item("no running session", -1)

func _choose_session(index: int) -> void:
	client.select_session(session_selector.get_item_id(index))

func _session_changed(_id: int) -> void:
	_unsubscribe()
	_selection_serial += 1
	_cancel_queries()
	selected_entity = {}
	_remote_selection_echoes.clear()
	_clear_proxy()
	_clear_proxies()
	_search_expansion.clear()
	_search_matches.clear()
	_searching = false
	_candidates.hide()
	rows.clear()
	_sync_items()
	if remote.last_session != _id:
		remote.clear_selection()
	_sessions_changed()
	_subscription_visibility()

func _unsubscribe() -> void:
	if client != null and _subscribed_session != -1 and client.is_session_active(_subscribed_session):
		client.request("godot.unsubscribe", {}, Callable(), _subscribed_session)
	_subscribed_session = -1
	_reset_snapshot()

func _interval_changed() -> void:
	_unsubscribe()
	_subscription_visibility()

func _subscription_visibility() -> void:
	if client == null:
		return
	if not is_visible_in_tree() or not client.is_session_active(client.active_session_id):
		_unsubscribe()
		if client.active_session_id == -1:
			status_label.text = "no running session"
		return
	if not client.config_ready:
		var reason: String = client.sessions[client.active_session_id].error
		status_label.text = reason if not reason.is_empty() else "Connecting…"
		return
	if _subscribed_session == client.active_session_id:
		return
	_subscribed_session = client.active_session_id
	var session := _subscribed_session
	status_label.text = "Connecting…"
	client.request("godot.subscribe", {"interval_s": client.update_interval}, func(frame):
		if session == _subscribed_session and frame.has("error"):
			status_label.text = frame.error.message
			_subscribed_session = -1
	, session)

func _summary(session_id: int, params: Dictionary) -> void:
	if session_id == _subscribed_session:
		apply_summary(params)

func apply_summary(params: Dictionary) -> void:
	if params.get("subscription_ended", false):
		_subscribed_session = -1
		_end_subscription(params.get("reason", "Subscription ended"))
		return
	var pending: Dictionary = {}
	if params.get("snapshot", false):
		var index: int = params.get("snapshot_index", 0)
		if index == 0:
			_reset_snapshot()
			_cancel_queries()
			if not params.get("snapshot_complete", true):
				cancel_inspection()
		if index != _snapshot_index:
			_end_subscription("Snapshot interrupted: missing or out-of-order data")
			return
		for row in params.get("added", []):
			_snapshot_rows[row.entity.bits] = row
		_snapshot_index += 1
		if not params.get("snapshot_complete", true):
			status_label.text = "Loading snapshot (%d entities received)…" % _snapshot_rows.size()
			return
		for delta in _snapshot_deltas:
			_apply_delta(_snapshot_rows, delta)
		rows = _snapshot_rows
		pending = _pending_selection
		_reset_snapshot()
	elif _snapshot_index > 0:
		_snapshot_deltas.append(params)
		return
	else:
		_apply_delta(rows, params)
	_sync_items()
	_search(search_box.text)
	status_label.text = "%d entities" % rows.size()
	if not pending.is_empty() and client != null and pending.session == client.active_session_id:
		_ensure_selection(pending.reference, pending.session, pending.inspect)

func _end_subscription(reason: String) -> void:
	_unsubscribe()
	cancel_inspection()
	_search(search_box.text)
	status_label.text = "%s. Reopen the pane to retry." % reason

func _reset_snapshot() -> void:
	_snapshot_rows = {}
	_snapshot_deltas.clear()
	_snapshot_index = 0
	_pending_selection = {}

func _apply_delta(target: Dictionary, params: Dictionary) -> void:
	for reference in params.get("removed", []):
		target.erase(reference.bits)
	for row in params.get("added", []) + params.get("updated", []):
		target[row.entity.bits] = row

func _sync_items() -> void:
	var scroll := entity_tree.get_scroll()
	var selected = entity_tree.get_selected()
	# Freeing and reparenting items makes the Tree emit item_selected on its own; that is not a
	# user selection and must not overwrite selected_entity.
	var was_selecting := _selecting
	_selecting = true
	var selected_bits = selected.get_metadata(0) if selected != null else null
	# Move surviving descendants before freeing a removed parent.
	for bits in items.keys():
		if not rows.has(bits):
			for child in items[bits].get_children():
				items[bits].remove_child(child)
				_root.add_child(child)
			items[bits].free()
			items.erase(bits)
	for bits in rows:
		if not items.has(bits):
			items[bits] = entity_tree.create_item(_root)
			items[bits].collapsed = true
		var item: TreeItem = items[bits]
		var row: Dictionary = rows[bits]
		item.set_metadata(0, bits)
		item.set_text(0, row.name if not row.name.is_empty() else "Entity " + bits)
		item.set_tooltip_text(0, bits + "\n" + "\n".join(row.components))
		item.set_icon(0, _entity_icon(row))
	var parents: Dictionary = {}
	for bits in rows:
		var parent = rows[bits].get("parent")
		var parent_item = _root
		if parent is Dictionary and items.has(parent.bits) and not _has_cycle(bits):
			parent_item = items[parent.bits]
		parents[bits] = parent_item
		var item: TreeItem = items[bits]
		if item.get_parent() != parent_item and item.get_parent() != _root:
			item.get_parent().remove_child(item)
			_root.add_child(item)
	for bits in items:
		var item: TreeItem = items[bits]
		if item.get_parent() != parents[bits]:
			item.get_parent().remove_child(item)
			parents[bits].add_child(item)
	if selected_bits != null and items.has(selected_bits) and entity_tree.get_selected() != items[selected_bits]:
		items[selected_bits].deselect(0)
		items[selected_bits].select(0)
	_selecting = was_selecting
	_restore_scroll.call_deferred(scroll)
	_filter()

func _restore_scroll(scroll: Vector2) -> void:
	for child in entity_tree.get_children(true):
		if child is VScrollBar:
			child.value = scroll.y
		elif child is HScrollBar:
			child.value = scroll.x

func _has_cycle(bits: String) -> bool:
	var seen: Dictionary = {}
	while rows.has(bits):
		if seen.has(bits):
			return true
		seen[bits] = true
		var parent = rows[bits].get("parent")
		if not parent is Dictionary:
			return false
		bits = parent.bits
	return false

static func internal(row: Dictionary) -> bool:
	if not row.get("name", "").is_empty() or row.get("has_node", false) or row.get("parent") != null:
		return false
	if "bevy_ecs::name::Name" in row.components:
		return false
	if "bevy_ecs::resource::IsResource" in row.components or "bevy_ecs::observer::Observer" in row.components:
		return true
	for component in row.components:
		if not component.begins_with("bevy_ecs::"):
			return false
	return true

func _entity_icon(row: Dictionary) -> Texture2D:
	if not Engine.is_editor_hint():
		return null
	var theme := EditorInterface.get_editor_theme()
	var icon := _entity_icon_name(row, theme)
	return theme.get_icon(icon, "EditorIcons") if not icon.is_empty() else null

func _entity_icon_name(row: Dictionary, theme: Theme) -> String:
	if not row.has_node:
		for icon in ["Object", "Resource", "Circle"]:
			if theme.has_icon(icon, "EditorIcons"):
				return icon
		return ""
	var best := ""
	for component in row.components:
		var short: String = component.get_slice("::", component.get_slice_count("::") - 1)
		if short.ends_with("Marker"):
			var type = short.trim_suffix("Marker")
			if not ClassDB.class_exists(type) or not ClassDB.is_parent_class(type, "Node") or not theme.has_icon(type, "EditorIcons"):
				continue
			if best.is_empty() or ClassDB.is_parent_class(type, best):
				best = type
	if best.is_empty():
		for icon in ["Node", "Godot"]:
			if theme.has_icon(icon, "EditorIcons"):
				return icon
	return best

func _cancel_queries() -> void:
	_search_serial += 1
	if client != null:
		for id in _query_ids:
			client.cancel_request(id)
	_query_ids.clear()

func _search(text: String) -> void:
	if text.is_empty() and _searching:
		for bits in _search_expansion:
			if items.has(bits):
				items[bits].collapsed = _search_expansion[bits]
		_search_expansion.clear()
	elif not text.is_empty() and not _searching:
		for bits in items:
			_search_expansion[bits] = items[bits].collapsed
	_searching = not text.is_empty()
	_cancel_queries()
	_search_matches.clear()
	_filter()
	if text.is_empty() or _snapshot_index > 0 or client == null or client.active_session_id == -1:
		return
	var filters: Array = []
	if text.is_valid_int():
		filters.append({})
	elif text.contains("/"):
		filters.append({"node_path": text})
	else:
		var types: Dictionary = {}
		if text.contains("::"):
			types[text] = true
		for row in rows.values():
			for component in row.components:
				if component.to_lower().contains(text.to_lower()):
					types[component] = true
		for component in types:
			filters.append({"component": component})
	for params in filters:
		_query_page(params, 0, _search_serial, text)

func _query_page(filter: Dictionary, page: int, serial: int, text: String) -> void:
	var params = filter.duplicate()
	params.merge({"page": page, "page_size": 256})
	var id = client.request("godot.query", params, func(frame):
		if serial != _search_serial:
			return
		if frame.has("error"):
			status_label.text = frame.error.message
			return
		for row in frame.result.entities:
			if not text.is_valid_int() or row.entity.bits.contains(text):
				_search_matches[row.entity.bits] = true
		_filter()
		if (page + 1) * 256 < frame.result.total:
			_query_page(filter, page + 1, serial, text)
	)
	_query_ids.append(id)

func _filter() -> void:
	if entity_tree == null:
		return
	hidden_count = 0
	var visible_bits: Dictionary = {}
	var text := search_box.text.to_lower()
	for bits in rows:
		var row: Dictionary = rows[bits]
		if internal(row) and not show_internal.button_pressed:
			continue
		if text.is_empty() or row.name.to_lower().contains(text) or _search_matches.has(bits):
			var current: String = bits
			var seen: Dictionary = {}
			while rows.has(current) and not seen.has(current):
				seen[current] = true
				visible_bits[current] = true
				var parent = rows[current].get("parent")
				if not parent is Dictionary:
					break
				current = parent.bits
				if not text.is_empty() and items.has(current):
					items[current].collapsed = false
	for bits in items:
		items[bits].visible = visible_bits.has(bits)
		if internal(rows[bits]) and not items[bits].visible and not show_internal.button_pressed:
			hidden_count += 1
	hidden_label.text = "%d internal hidden" % (0 if show_internal.button_pressed else hidden_count)
	# Hiding a selected TreeItem clears the Tree's selection; restore it once visible again.
	var selected_bits: String = selected_entity.get("bits", "")
	if items.has(selected_bits) and items[selected_bits].visible and entity_tree.get_selected() != items[selected_bits]:
		_selecting = true
		items[selected_bits].deselect(0)
		items[selected_bits].select(0)
		_selecting = false

func _on_selected() -> void:
	if _selecting or entity_tree.get_selected() == null:
		return
	var bits = entity_tree.get_selected().get_metadata(0)
	if rows.has(bits):
		select_entity(rows[bits].entity)

func cancel_inspection() -> void:
	_selection_serial += 1
	_pending_selection = {}
	if remote != null:
		remote.cancel_selection()

func select_entity(reference: Dictionary, inspect: bool = true) -> void:
	cancel_inspection()
	var bits: String = reference.bits
	if not rows.has(bits):
		if client != null:
			_ensure_selection(reference, client.active_session_id, inspect)
		return
	if rows[bits].entity != reference:
		status_label.text = "Entity not available in this session"
		return
	if inspect or not items[bits].visible:
		if internal(rows[bits]):
			show_internal.button_pressed = true
		if not items[bits].visible:
			search_box.text = ""
			_search("")
		items[bits].uncollapse_tree()
		entity_tree.scroll_to_item(items[bits])
	selected_entity = reference.duplicate()
	_selecting = true
	var ancestor: TreeItem = items[bits].get_parent()
	while ancestor != null and ancestor != _root:
		ancestor.collapsed = false
		ancestor = ancestor.get_parent()
	items[bits].select(0)
	_selecting = false
	entity_selected.emit(reference)
	if not inspect or client == null:
		return
	var serial := _selection_serial
	var session: int = client.active_session_id
	if rows[bits].has_node:
		var components: Array = []
		for component in rows[bits].components:
			if component.ends_with("::GodotNodeHandle"):
				components.append(component)
		client.request("godot.get_components", {"entity": reference, "components": components}, func(frame):
			if serial != _selection_serial or session != client.active_session_id:
				return
			if frame.has("error"):
				status_label.text = frame.error.message
				return
			for value in frame.result.values():
				if value.kind == "node" and value.valid:
					var key := "%d:%s" % [session, value.instance_id]
					_remote_selection_echoes[key] = serial
					var selected: bool = await remote.select_node(value.instance_id, session)
					if selected:
						# Godot 4.6 queues objects_selected from Tree.set_selected.
						_forget_remote_echo.call_deferred(key, serial)
					else:
						_forget_remote_echo(key, serial)
					if not selected and serial == _selection_serial and session == client.active_session_id:
						_inspect_proxy(reference, session)
		)
	else:
		_inspect_proxy(reference, session)

func _forget_remote_echo(key: String, serial: int) -> void:
	if _remote_selection_echoes.get(key) == serial:
		_remote_selection_echoes.erase(key)

func _inspect_proxy(reference: Dictionary, session: int) -> void:
	_clear_proxy()
	_proxy = proxy_for(reference, session)
	EditorInterface.inspect_object(_proxy)
	_proxy.attach_inspector(EditorInterface.get_inspector())
	_proxy.refresh()

func proxy_for(reference: Dictionary, session: int):
	var key := "%d:%s:%s" % [session, reference.bits, reference.generation]
	if proxies.has(key) and not proxies[key].valid_target():
		proxies[key].invalidate()
		proxies.erase(key)
	if not proxies.has(key):
		var proxy = Proxy.new(reference, session)
		proxy.configure(client)
		proxy.entity_link.connect(func(target): select_entity(target))
		proxy.node_link.connect(func(id):
			if remote != null:
				remote.select_node(id, session))
		proxies[key] = proxy
	return proxies[key]

func _process(delta: float) -> void:
	for proxy in proxies.values():
		proxy.advance(delta)

func _clear_proxies() -> void:
	for proxy in proxies.values():
		proxy.invalidate()
	proxies.clear()

func _clear_proxy() -> void:
	if _proxy != null and Engine.is_editor_hint() and EditorInterface.get_inspector().get_edited_object() == _proxy:
		EditorInterface.inspect_object(null)
	_proxy = null

func _remote_selected(ids: Array, session: int) -> void:
	if ids.size() != 1 or not client.is_session_active(session):
		return
	var key := "%d:%s" % [session, ids[0]]
	if _remote_selection_echoes.has(key):
		_remote_selection_echoes.erase(key)
		return
	client.select_session(session)
	cancel_inspection()
	var serial := _selection_serial
	client.request("godot.entity_for_node", {"instance_id": ids[0]}, func(frame):
		if session == client.active_session_id and serial == _selection_serial and not frame.has("error"):
			_ensure_selection(frame.result, session, false)
	, session)

func shutdown() -> void:
	_unsubscribe()
	_cancel_queries()
	_selection_serial += 1
	_remote_selection_echoes.clear()
	if remote != null:
		remote.cancel_selection()
	_clear_proxy()
	_clear_proxies()
	selected_entity = {}
	rows.clear()
	_sync_items()
	status_label.text = "no running session"

func show_candidates(candidates: Array) -> void:
	_candidates.clear()
	_candidates.visible = not candidates.is_empty()
	status_label.text = "no entity" if candidates.is_empty() else "%d runtime candidates: choose an entity" % candidates.size()
	for candidate in candidates:
		var row = rows.get(candidate.bits, {})
		_candidates.add_item("%s (%s)" % [row.get("name", "Entity"), candidate.bits])
		_candidates.set_item_metadata(_candidates.item_count - 1, candidate)
	_candidates.select(-1)

func _ensure_selection(reference: Dictionary, session: int, inspect: bool) -> void:
	if rows.has(reference.bits):
		select_entity(reference, inspect)
	elif _snapshot_index == 0:
		_query_selection(reference, session, inspect, 0, _selection_serial)
	else:
		_pending_selection = {"reference": reference.duplicate(), "session": session, "inspect": inspect}

func _query_selection(reference: Dictionary, session: int, inspect: bool, page: int, serial: int) -> void:
	client.request("godot.query", {"page": page, "page_size": 256}, func(frame):
		if session != client.active_session_id or serial != _selection_serial or frame.has("error"):
			return
		for row in frame.result.entities:
			rows[row.entity.bits] = row
		if (page + 1) * 256 < frame.result.total:
			_query_selection(reference, session, inspect, page + 1, serial)
		else:
			_sync_items()
			if rows.has(reference.bits):
				select_entity(reference, inspect)
			else:
				status_label.text = "Entity not available in this session"
	, session)
