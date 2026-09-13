extends RefCounted

const Fixtures = preload("res://addons/godot-bevy/test/bevy_inspector_fixtures.gd")
const PanelScene = preload("res://addons/godot-bevy/bevy_inspector_panel.tscn")
const ValueEditor = preload("res://addons/godot-bevy/bevy_value_editor.gd")
const RemoteTree = preload("res://addons/godot-bevy/bevy_remote_tree.gd")
const RpcClient = preload("res://addons/godot-bevy/bevy_rpc_client.gd")
const Proxy = preload("res://addons/godot-bevy/bevy_entity_proxy.gd")

class FakeClient extends RefCounted:
	signal active_session_changed(session)
	var frames: Array = []
	var callbacks: Dictionary = {}
	var active_session_id := 0
	var update_interval := 0.5
	var sessions := {0: {"order": 1}}
	func is_session_active(session: int) -> bool:
		return session == active_session_id and session != -1
	func select_session(session: int) -> void:
		active_session_id = session
	func request(method: String, params: Dictionary, callback: Callable = Callable(), _session: int = -1) -> int:
		var id := frames.size() + 1
		frames.append({"jsonrpc": "2.0", "id": id, "method": method, "params": params})
		callbacks[id] = callback
		return id
	func cancel_request(id: int) -> void:
		callbacks.erase(id)
	func last_frame(method: String) -> Dictionary:
		for i in range(frames.size() - 1, -1, -1):
			if frames[i].method == method:
				return frames[i]
		return {}
	func reply(id: int, frame: Dictionary) -> void:
		var callback: Callable = callbacks[id]
		callbacks.erase(id)
		callback.call(frame)

class FakeSession extends RefCounted:
	signal started
	signal stopped
	var sent: Array = []
	func is_active() -> bool:
		return false
	func send_message(message: String, data: Array) -> void:
		sent.append([message, data])

class FakeRemote extends Node:
	signal selection_changed(ids, session)
	signal availability_changed(reason)
	var reason := "test adapter"
	var last_session := -1
	var requested: Array = []
	func select_node(id: String, session: int) -> bool:
		requested.append([id, session])
		selection_changed.emit.call_deferred([id], session)
		return true
	func cancel_selection() -> void:
		pass
	func clear_selection() -> void:
		last_session = -1

var failures: Array = []
var started: Array = []
var completed: Array = []
var finished := false

func check(condition: bool, assertion: String) -> void:
	if not condition:
		failures.append(assertion)
		push_error("BEVY_INSPECTOR_TEST " + assertion)

func run(host: Node) -> int:
	if "--bevy-inspector-test-abort" in OS.get_cmdline_user_args():
		await _run_test("intentional abort", _intentional_abort)
		await _run_test("intentional async abort", _intentional_async_abort.bind(host))
	else:
		await _run_test("tree", _tree.bind(host))
		await _run_test("values", _values.bind(host))
		await _run_test("proxy mapping", _proxy_mapping)
		await _run_test("component presentation", _component_presentation)
		await _run_test("proxy edits", _proxy_edits)
		await _run_test("proxy paths", _proxy_paths)
		await _run_test("proxy ownership", _proxy_ownership.bind(host))
		await _run_test("client", _client)
		await _run_test("handshake", _handshake)
		await _run_test("subscriptions", _subscriptions.bind(host))
		await _run_test("chunked snapshots", _chunked_snapshots.bind(host))
		await _run_test("snapshot recovery", _snapshot_recovery.bind(host))
		await _run_test("snapshot selection", _snapshot_selection.bind(host))
		await _run_test("selection echo", _selection_echo.bind(host))
		await _run_test("local cancellation", _local_cancellation.bind(host))
		await _run_test("dock tabs", _dock_tabs.bind(host))
		await _run_test("detached inspector", _detached_inspector.bind(host))
	finished = true
	print("BEVY_INSPECTOR_TEST complete failures=%d started=%d completed=%d" % [failures.size(), started.size(), completed.size()])
	return 0 if failures.is_empty() else 1

func _run_test(label: String, test: Callable) -> void:
	started.append(label)
	await test.call()
	check(label in completed, label + " aborted before completion")

func _intentional_abort() -> void:
	var missing: Dictionary = {}
	var value = missing["intentional script error"]
	check(value != null, "unreachable after script error")
	completed.append("intentional abort")

func _intentional_async_abort(host: Node) -> void:
	await host.get_tree().process_frame
	var missing: Dictionary = {}
	var value = missing["intentional async script error"]
	check(value != null, "unreachable after async script error")
	completed.append("intentional async abort")

func _tree(host: Node) -> void:
	var panel = PanelScene.instantiate()
	host.add_child(panel)
	panel.apply_summary(Fixtures.snapshot().params)
	check(panel.items.size() == 7, "snapshot creates seven entity items")
	check(panel.items["4294967298"].get_parent() == panel.items["4294967297"], "snapshot nests Player under MainMenu")
	check(panel.search_box.placeholder_text == "Filter entities" and panel.search_box.clear_button_enabled and panel.search_box.tooltip_text.contains("component"), "search has concise placeholder, clear button and supported-search tooltip")
	var theme := Theme.new()
	for icon in ["Object", "Node", "Node2D"]:
		theme.set_icon(icon, "EditorIcons", GradientTexture2D.new())
	var node := Fixtures.row("1", "", ["game::MissingClassMarker", "godot_bevy::NodeMarker", "godot_bevy::Node2DMarker", "godot_bevy::CharacterBody2DMarker"], null, true)
	check(panel._entity_icon_name(node, theme) == "Node2D", "icon keeps the most specific class with an available theme icon")
	check(panel._entity_icon_name(Fixtures.row("2", "", ["game::MissingClassMarker"], null, true), theme) == "Node", "invalid marker class uses node fallback")
	check(panel._entity_icon_name(Fixtures.row("3", "", ["game::Node2DMarker"]), theme) == "Object", "pure-ECS icon remains distinct even with a node-like marker")
	check(panel.hidden_count == 3 and panel.hidden_label.text == "3 internal hidden", "default internal filter counts observer, resource and empty entity")
	check(not panel.items["4294967299"].visible and panel.items["4294967303"].visible, "internal observer hidden but empty Name retained")
	check(not panel.items["4294967300"].visible, "resource with its own component type is hidden")
	for marker in ["bevy_ecs::observer::Observer", "bevy_ecs::resource::IsResource"]:
		check(panel.internal(Fixtures.row("1", "", ["game::OwnType", marker])), "marker hides mixed component types: " + marker)
		check(not panel.internal(Fixtures.row("1", "named", [marker])), "named marker entity remains visible")
		check(not panel.internal(Fixtures.row("1", "", [marker], null, true)), "node-backed marker entity remains visible")
		check(not panel.internal(Fixtures.row("1", "", [marker], Fixtures.entity_ref("2"))), "parented marker entity remains visible")
		check(not panel.internal(Fixtures.row("1", "", [marker, "bevy_ecs::name::Name"])), "empty Name marker entity remains visible")
	panel.items["4294967297"].collapsed = false
	panel.select_entity(Fixtures.entity_ref("4294967298"), false)
	var original = panel.items["4294967298"]
	panel.apply_summary(Fixtures.delta().params)
	check(panel.items["4294967298"] == original, "delta retains TreeItem identity")
	check(panel.entity_tree.get_selected() == original, "delta retains selection")
	check(not panel.items["4294967297"].collapsed, "delta retains ancestor expansion")
	check(original.get_text(0) == "Player renamed" and not panel.items.has("4294967302"), "delta updates and removes rows")
	panel.search_box.text = "Child"
	panel._search("Child")
	check(panel.items["4294967304"].visible and panel.items["4294967297"].visible, "name search shows matches and ancestors")
	check(not panel.items["4294967303"].visible, "name search hides unrelated rows")
	check(not panel.items["4294967298"].collapsed, "search expands matching ancestors")
	panel.search_box.text = ""
	panel._search("")
	check(panel.items["4294967298"].collapsed, "clearing search restores expansion")
	panel.show_internal.button_pressed = true
	check(panel.items["4294967299"].visible and panel.hidden_label.text == "0 internal hidden", "toggle reveals internal set")
	var fake = FakeClient.new()
	panel.client = fake
	panel.search_box.text = "game::Speed"
	panel._search("game::Speed")
	check(fake.frames.back().method == "godot.query" and fake.frames.back().params.component == "game::Speed", "component search uses query")
	fake.reply(fake.frames.size(), {"result": {"total": 1, "entities": [Fixtures.row("4294967298", "Player", ["game::Speed"], Fixtures.entity_ref("4294967297"))]}})
	check(panel.items["4294967298"].visible and panel.items["4294967297"].visible, "query matches retain ancestors")
	panel.search_box.text = "/root/MainMenu"
	panel._search(panel.search_box.text)
	check(fake.frames.back().params.node_path == "/root/MainMenu", "node path search uses query")
	panel.search_box.text = "4294967298"
	panel._search(panel.search_box.text)
	check(fake.frames.back().method == "godot.query" and fake.frames.back().params == {"page": 0, "page_size": 256}, "ID search sends unfiltered page zero")
	fake.reply(fake.frames.back().id, {"result": {"total": 257, "entities": [Fixtures.row("4294967303", "", ["bevy_ecs::name::Name"])]}})
	check(fake.frames.back().params == {"page": 1, "page_size": 256}, "ID search requests next unfiltered page")
	fake.reply(fake.frames.back().id, {"result": {"total": 257, "entities": [Fixtures.row("4294967298", "Player", ["game::Speed"], Fixtures.entity_ref("4294967297"))]}})
	check(panel.items["4294967298"].visible and panel.items["4294967297"].visible and not panel.items["4294967303"].visible, "ID result narrows by bits and retains ancestors across pages")
	panel.client = null
	panel.search_box.text = ""
	panel._search("")
	panel.apply_summary({"snapshot": false, "added": [], "removed": [Fixtures.entity_ref("4294967297")], "updated": [Fixtures.row("4294967298", "Player", ["game::Speed"])]})
	check(panel.items["4294967298"] == original and original.get_parent() == panel._root, "removing parent retains and reparents surviving child")
	check(panel.entity_tree.get_selected() == original, "reparent retains selection")
	panel.apply_summary({"updated": [Fixtures.row("4294967298", "Player", ["game::Speed"], Fixtures.entity_ref("4294967303"))]})
	check(panel.entity_tree.get_selected() == original, "moving selected item retains selection")
	panel.select_entity(Fixtures.entity_ref("4294967304"), false)
	var child = panel.items["4294967304"]
	panel.apply_summary({"updated": [Fixtures.row("4294967298", "Player", ["game::Speed"])]})
	check(panel.entity_tree.get_selected() == child, "moving selected ancestor retains descendant selection")
	panel.show_internal.button_pressed = false
	panel.apply_summary({"added": [Fixtures.row("90", "", [])]})
	check(not panel.items["90"].visible, "empty entity starts hidden")
	var empty_item = panel.items["90"]
	panel.apply_summary({"updated": [Fixtures.row("90", "", ["game::Speed"])]})
	check(panel.items["90"] == empty_item and empty_item.visible and empty_item.get_tooltip_text(0).contains("game::Speed"), "membership addition refreshes component list and default filter in place")
	panel.apply_summary({"updated": [Fixtures.row("90", "", [])]})
	check(panel.items["90"] == empty_item and not empty_item.visible and not empty_item.get_tooltip_text(0).contains("game::Speed"), "last game component removal refreshes component list and hides entity in place")
	panel.free()
	completed.append("tree")

func _values(host: Node) -> void:
	var scalar_kinds := ["integer", "float", "bool", "string", "char", "enum", "entity", "node",
		"asset", "opaque", "unsupported", "depth_limit"]
	for kind in Fixtures.values():
		var fake = FakeClient.new()
		var control = ValueEditor.new()
		host.add_child(control)
		var value: Dictionary = Fixtures.values()[kind]
		control.configure(value, Fixtures.entity_ref("4294967298"), "game::Speed", [], fake, 0)
		if kind in scalar_kinds:
			check(control.editor != null, kind + " exposes its scalar value or navigation")
		else:
			check(control.fold != null and control.children_editors.size() > 0, kind + " exposes its nested values")
		if kind in ["integer", "float", "bool", "string", "char", "enum"]:
			var edit = {"integer": 9, "float": 8.5, "bool": false, "string": "edited", "char": "a", "enum": {"variant": "Idle"}}[kind]
			if control.editor is SpinBox:
				control.editor.value = edit
			elif control.editor is CheckBox:
				control.editor.button_pressed = edit
			elif control.editor is LineEdit:
				control.editor.text_submitted.emit(edit)
			else:
				control.editor.item_selected.emit(1)
			check(fake.frames[0] == {"jsonrpc": "2.0", "id": 1, "method": "godot.mutate_leaf", "params": {
				"entity": Fixtures.entity_ref("4294967298"), "component": "game::Speed", "path": [], "value": edit}}, kind + " emits exact mutate frame")
			fake.reply(1, {"error": {"message": "decode failure"}})
			check(control.model == value and control.error_label.text == "decode failure", kind + " rejected edit retains accepted model and shows reason")
			if control.editor is SpinBox:
				check(control.editor.value == float(value.value), kind + " rejected edit restores shown number")
			elif control.editor is CheckBox:
				check(control.editor.button_pressed == value.value, "rejected bool restores checkbox")
			elif control.editor is LineEdit:
				check(control.editor.text == value.value, kind + " rejected edit restores shown text")
			else:
				check(control.editor.get_item_text(control.editor.selected) == value.variant, "rejected enum restores selection")
		elif kind in ["struct", "tuple_struct", "tuple", "list", "array"]:
			var child = control.children_editors[0]
			child.editor.value = 9.0
			var segment = {"field": "speed"} if kind == "struct" else {"index": 0}
			check(fake.frames[0].params.path == [segment], kind + " leaf emits exact path")
		elif kind in ["entity", "node"]:
			var links: Array = []
			control.entity_link.connect(func(reference): links.append(reference))
			control.node_link.connect(func(id): links.append(id))
			control.editor.pressed.emit()
			check(links == [value.entity if kind == "entity" else value.instance_id], kind + " link preserves exact reference")
		elif kind in ["map", "set"]:
			check(not control.children_editors[0].model.writable.allowed and fake.frames.is_empty(), kind + " is read-only")
		control.free()
	var wide = Fixtures.scalar("integer", "340282366920938463463374607431768211455")
	wide.type_path = "u128"
	var fake = FakeClient.new()
	var control = ValueEditor.new()
	host.add_child(control)
	control.configure(wide, Fixtures.entity_ref("4294967298"), "game::Wide", [], fake, 0)
	check(control.editor is LineEdit and control.editor.text == wide.value, "wide integer displays exact decimal")
	control.editor.text_submitted.emit(wide.value)
	check(fake.frames[0].params.value == wide.value and fake.frames[0].params.value is String, "wide integer edit stays decimal String")
	fake.reply(1, {"result": wide})
	check(control.editor.text == wide.value and control.error_label.text == "Accepted", "accepted wide value displayed exactly")
	control.free()
	var nested = Fixtures.aggregate("enum", {"variant": "Moving", "unit_variants": [], "fields": [{"name": "speed", "value": Fixtures.scalar("float", 4.0)}]})
	control = ValueEditor.new()
	host.add_child(control)
	fake = FakeClient.new()
	control.configure(nested, Fixtures.entity_ref("4294967298"), "game::Mode", [{"field": "mode"}], fake, 0)
	control.children_editors[0].editor.value = 8.0
	check(fake.frames[0].params.path == [{"field": "mode"}, {"variant": "Moving"}, {"field": "speed"}], "enum payload path includes active variant guard")
	control.free()
	var ranged = Fixtures.scalar("integer", 4)
	ranged.range = {"min": 1.0, "max": 10.0}
	control = ValueEditor.new()
	host.add_child(control)
	control.configure(ranged, Fixtures.entity_ref("4294967298"), "game::Range", [])
	check(control.editor.min_value == 1.0 and control.editor.max_value == 10.0 and control.editor.step == 1.0, "numeric input honors InspectorRange and integer step")
	control.free()
	var off_grid = Fixtures.scalar("float", 0.125)
	control = ValueEditor.new()
	host.add_child(control)
	fake = FakeClient.new()
	control.configure(off_grid, Fixtures.entity_ref("4294967298"), "game::Speed", [], fake, 0)
	check(control.editor.value == 0.125, "off-grid float displays exactly")
	control.editor.value = 0.001
	check(fake.frames[0].params.value == 0.001, "off-grid float submits exactly")
	fake.reply(1, {"result": off_grid})
	check(control.editor.value == 0.125, "off-grid float acknowledgement displays exactly")
	control.editor.value = 0.25
	fake.reply(2, {"error": {"message": "rejected"}})
	check(control.editor.value == 0.125, "off-grid float rejection restores exact accepted value")
	control.editor.value = 0.0625
	fake.reply(3, {"result": Fixtures.scalar("float", 0.0625)})
	control.editor.value = 0.125
	check(fake.frames[3].params.value == 0.125, "0.125 submission is unchanged")
	control.free()
	completed.append("values")

func _client() -> void:
	var client = RpcClient.new()
	var first = FakeSession.new()
	var second = FakeSession.new()
	client.setup_session(0, first)
	client.setup_session(1, second)
	first.started.emit()
	second.started.emit()
	client.select_session(0)
	var replies: Array = []
	var typed: Array[int] = [1, 2]
	var id = client.request("test", {"name": &"Speed", "path": NodePath("/root"), "items": typed}, func(frame): replies.append(frame))
	var params: Dictionary = first.sent[0][1][0].params
	check(params.name is String and params.path is String and not params.items.is_typed(), "client normalizes names, paths and typed arrays")
	client._capture("bevy:rpc", [{"jsonrpc": "2.0", "id": id, "result": {}}], 1)
	check(replies.is_empty(), "reply from another session cannot complete request")
	client._capture("bevy:rpc", [{"jsonrpc": "2.0", "id": id, "result": {}}], 0)
	check(replies.size() == 1, "matching session and id complete request")
	second.stopped.emit()
	second.started.emit()
	check(client.active_session_id == 1, "newest started session selected")
	second.stopped.emit()
	check(client.active_session_id == 0 and not client.is_session_active(1), "stopped session detaches and falls back to running session")
	for i in 6:
		client._capture("bevy:rpc", [{"jsonrpc": "2.0", "method": "test", "params": {"n": i}}], 0)
	var diagnostics = client.diagnostics()
	check(diagnostics.last_frames.size() == 5 and diagnostics.last_frames[0].data[0].params.n == 1 and diagnostics.last_frames[4].data[0].params.n == 5, "diagnostics retain last five received frames")
	var pending_id = client.request("godot.query", {"page": 7}, func(_frame): pass)
	diagnostics = client.diagnostics()
	check(diagnostics.pending[pending_id].frame.params.page == 7 and diagnostics.pending[pending_id].session == 0, "diagnostics expose pending request frames and sessions")
	client.shutdown()
	check(client._pending.is_empty() and client.sessions.is_empty() and not first.started.is_connected(client._started.bind(0)), "shutdown clears requests and disconnects sessions")
	completed.append("client")

func _subscriptions(host: Node) -> void:
	var panel = PanelScene.instantiate()
	host.add_child(panel)
	var client = RpcClient.new()
	var session = FakeSession.new()
	var adapter = FakeRemote.new()
	host.add_child(adapter)
	client.setup_session(0, session)
	panel.setup(client, adapter)
	session.started.emit()
	check(session.sent.is_empty(), "session start waits for ready or retry interval")
	client._capture("bevy:rpc", [{"jsonrpc": "2.0", "method": "godot.ready", "params": {"enabled": true, "update_interval": 0.25}}], 0)
	check(session.sent.size() == 1 and session.sent[0][1][0].method == "godot.debugger_config", "subscription waits for runtime config")
	var config_id = session.sent[0][1][0].id
	client._capture("bevy:rpc", [{"jsonrpc": "2.0", "id": config_id, "result": {"enabled": true, "update_interval": 0.25}}], 0)
	check(session.sent.back()[1][0].method == "godot.subscribe" and session.sent.back()[1][0].params.interval_s == 0.25, "visible pane subscribes at configured interval")
	client._capture("bevy:rpc", [Fixtures.snapshot()], 0)
	check(panel.items.size() == 7, "id-less summary routed to subscribed pane")
	panel.hide()
	check(session.sent.back()[1][0].method == "godot.unsubscribe", "hidden pane unsubscribes")
	panel.show()
	check(session.sent.back()[1][0].method == "godot.subscribe", "shown pane resubscribes")
	client._stopped(0)
	check(panel.status_label.text == "no running session" and panel.items.is_empty(), "stopped session disconnects pane")
	session.started.emit()
	client.advance(20.0)
	check(panel.status_label.text.contains("20 s"), "pane reports unanswered endpoint")
	panel.hide()
	panel.show()
	check(panel.status_label.text.contains("20 s") and panel._subscribed_session == -1, "reopening pane preserves terminal connection error")
	session.stopped.emit()
	panel.shutdown()
	panel.free()
	adapter.free()
	client.shutdown()
	completed.append("subscriptions")

func _chunked_snapshots(host: Node) -> void:
	var panel = PanelScene.instantiate()
	var single = PanelScene.instantiate()
	host.add_child(panel)
	host.add_child(single)
	var client = RpcClient.new()
	var session = FakeSession.new()
	var adapter = FakeRemote.new()
	host.add_child(adapter)
	client.setup_session(0, session)
	panel.setup(client, adapter)
	session.started.emit()
	client._capture("bevy:rpc", [{"jsonrpc": "2.0", "method": "godot.ready", "params": {}}], 0)
	client._capture("bevy:rpc", [{"jsonrpc": "2.0", "id": session.sent.back()[1][0].id, "result": {"update_interval": 0.5}}], 0)
	var snapshot: Dictionary = Fixtures.snapshot().params
	var all_rows: Array = snapshot.added.duplicate()
	all_rows.reverse()
	var chunks: Array = []
	for index in 4:
		chunks.append({"jsonrpc": "2.0", "method": "godot.summary", "params": {
			"snapshot": true, "snapshot_index": index, "snapshot_complete": index == 3,
			"added": all_rows.slice(index * 2, mini(index * 2 + 2, all_rows.size())), "removed": [], "updated": []}})
	for index in 3:
		client._capture("bevy:rpc", [chunks[index]], 0)
		check(panel.rows.is_empty() and panel.items.is_empty(), "partial snapshot is not exposed as a complete world")
		check(panel.status_label.text == "Loading snapshot (%d entities received)…" % ((index + 1) * 2), "partial snapshot reports received count and loading state")
		if index == 0:
			var before_selection: int = session.sent.size()
			panel._ensure_selection(Fixtures.entity_ref("4294967298"), 0, false)
			check(session.sent.size() == before_selection, "missing snapshot selection waits without querying")
	client._capture("bevy:rpc", [chunks[3]], 0)
	check(panel.selected_entity == Fixtures.entity_ref("4294967298"), "missing snapshot selection replays after the final chunk")
	single.apply_summary(snapshot)
	check(panel.rows == single.rows and panel.items.size() == single.items.size(), "chunked and single snapshot contain the same complete world")
	for bits in single.items:
		check(panel.items.has(bits), "chunked snapshot contains entity " + bits)
		if not panel.items.has(bits):
			continue
		var item: TreeItem = panel.items[bits]
		var expected: TreeItem = single.items[bits]
		check(item.get_parent().get_metadata(0) == expected.get_parent().get_metadata(0) and item.get_text(0) == expected.get_text(0) and item.visible == expected.visible and item.get_tooltip_text(0) == expected.get_tooltip_text(0), "chunked snapshot has the same hierarchy, labels, components and filter for " + bits)
	check(panel.hidden_count == single.hidden_count and panel.status_label.text == "7 entities", "final snapshot chunk commits the full counts")
	panel.select_entity(Fixtures.entity_ref("4294967298"), false)
	var original = panel.items["4294967298"]
	panel.items["4294967297"].collapsed = false
	client._capture("bevy:rpc", [chunks[0]], 0)
	check(panel.items["4294967298"] == original and panel.rows == single.rows, "resnapshot keeps the last complete tree while loading")
	panel.search_box.text = "game::Speed"
	var sent: int = session.sent.size()
	panel._search(panel.search_box.text)
	check(session.sent.size() == sent, "nonempty query search is suppressed during snapshot loading")
	client._capture("bevy:rpc", [Fixtures.delta()], 0)
	check(panel.rows == single.rows, "interleaved delta waits for snapshot completion")
	for index in range(1, 4):
		client._capture("bevy:rpc", [chunks[index]], 0)
		check(session.sent.size() == sent + (1 if index == 3 else 0), "query search resumes only on the final chunk")
	check(session.sent.back()[1][0].method == "godot.query" and session.sent.back()[1][0].params.component == "game::Speed", "completed snapshot reissues the component search")
	panel.search_box.text = ""
	panel._search("")
	single.apply_summary(Fixtures.delta().params)
	check(panel.rows == single.rows and panel.items["4294967298"] == original and panel.entity_tree.get_selected() == original and not panel.items["4294967297"].collapsed, "snapshot commit replays interleaved deltas and preserves tree identity, selection and expansion")
	client._capture("bevy:rpc", [chunks[0]], 0)
	client._capture("bevy:rpc", [Fixtures.delta()], 0)
	panel._ensure_selection(Fixtures.entity_ref("999"), 0, false)
	check(not panel._snapshot_rows.is_empty() and not panel._snapshot_deltas.is_empty(), "stop fixture has both staged rows and deltas")
	session.stopped.emit()
	check(panel.status_label.text == "no running session" and panel.rows.is_empty() and panel.items.is_empty() and panel.selected_entity.is_empty(), "stop mid-snapshot leaves a disconnected pane with no half-built tree")
	check(panel._snapshot_rows.is_empty() and panel._snapshot_deltas.is_empty() and panel._snapshot_index == 0, "stop discards every staged snapshot chunk and delta")
	check(panel._pending_selection.is_empty(), "stop discards the pending snapshot selection")
	client._capture("bevy:rpc", [chunks[3]], 0)
	check(panel.rows.is_empty() and panel.items.is_empty() and panel.status_label.text == "no running session", "RPC client drops late chunks from a stopped session")
	session.started.emit()
	client._capture("bevy:rpc", [{"jsonrpc": "2.0", "method": "godot.ready", "params": {}}], 0)
	client._capture("bevy:rpc", [{"jsonrpc": "2.0", "id": session.sent.back()[1][0].id, "result": {"update_interval": 0.5}}], 0)
	client._capture("bevy:rpc", [chunks[3]], 0)
	check(panel.rows.is_empty() and panel.items.is_empty(), "restarted session rejects a continuation without its first chunk")
	panel.hide()
	panel.show()
	client._capture("bevy:rpc", [chunks[0]], 0)
	check(not panel._snapshot_rows.is_empty(), "client shutdown fixture has an unfinished snapshot")
	client.shutdown()
	check(panel.status_label.text == "no running session" and panel.rows.is_empty() and panel.items.is_empty() and panel._snapshot_rows.is_empty(), "client shutdown discards an unfinished snapshot")
	panel.shutdown()
	panel.free()
	single.free()
	adapter.free()
	completed.append("chunked snapshots")

func _snapshot_recovery(host: Node) -> void:
	var panel = PanelScene.instantiate()
	host.add_child(panel)
	var client = RpcClient.new()
	var session = FakeSession.new()
	var adapter = FakeRemote.new()
	host.add_child(adapter)
	client.setup_session(0, session)
	panel.setup(client, adapter)
	session.started.emit()
	client._capture("bevy:rpc", [{"jsonrpc": "2.0", "method": "godot.ready", "params": {}}], 0)
	client._capture("bevy:rpc", [{"jsonrpc": "2.0", "id": session.sent.back()[1][0].id, "result": {"update_interval": 0.5}}], 0)
	client._capture("bevy:rpc", [Fixtures.snapshot()], 0)
	panel.select_entity(Fixtures.entity_ref("4294967298"), false)
	var complete: Dictionary = panel.rows.duplicate(true)
	var selected: TreeItem = panel.entity_tree.get_selected()
	var partial := {"jsonrpc": "2.0", "method": "godot.summary", "params": {
		"snapshot": true, "snapshot_index": 0, "snapshot_complete": false,
		"added": [Fixtures.row("999", "Partial", ["game::Speed"])]}}
	for ending in [
		{"subscription_ended": true, "reason": "debugger disabled"},
		{"snapshot": true, "snapshot_index": 2, "snapshot_complete": true, "added": []},
	]:
		client._capture("bevy:rpc", [partial], 0)
		client._capture("bevy:rpc", [Fixtures.delta()], 0)
		panel.select_entity(Fixtures.entity_ref("999"), false)
		check(not panel._snapshot_rows.is_empty() and not panel._snapshot_deltas.is_empty() and not panel._pending_selection.is_empty(), "recovery fixture has staged rows, deltas and selection")
		client._capture("bevy:rpc", [{"jsonrpc": "2.0", "method": "godot.summary", "params": ending}], 0)
		check(panel._subscribed_session == -1 and panel._snapshot_index == 0 and panel._snapshot_rows.is_empty() and panel._snapshot_deltas.is_empty() and panel._pending_selection.is_empty(), "terminal or gap discards the unfinished subscription")
		check(panel.rows == complete and panel.entity_tree.get_selected() == selected and panel.selected_entity == Fixtures.entity_ref("4294967298") and not panel.items.has("999"), "terminal or gap retains the last complete tree and selection")
		check(panel.status_label.text.contains("Reopen") and not panel.status_label.text.contains("Loading"), "interrupted snapshot explains how to retry")
		if ending.has("snapshot_index"):
			check(session.sent.back()[1][0].method == "godot.unsubscribe", "snapshot sequence gap cancels the runtime stream")
		client._capture("bevy:rpc", [{"jsonrpc": "2.0", "method": "godot.summary", "params": {"snapshot": true, "snapshot_index": 3, "snapshot_complete": true, "added": []}}], 0)
		check(panel.rows == complete and panel._snapshot_index == 0, "late continuation cannot commit an abandoned snapshot")
		panel.search_box.text = "game::Speed"
		var sent: int = session.sent.size()
		panel._search(panel.search_box.text)
		check(session.sent.size() == sent + 1 and session.sent.back()[1][0].method == "godot.query", "interruption restores query-backed search")
		panel.search_box.text = ""
		panel._search("")
		sent = session.sent.size()
		panel.select_entity(Fixtures.entity_ref("999"), false)
		check(session.sent.size() == sent + 1 and session.sent.back()[1][0].method == "godot.query", "interruption restores missing-entity selection fetches")
		client._capture("bevy:rpc", [{"jsonrpc": "2.0", "id": session.sent.back()[1][0].id, "result": {"total": 0, "entities": []}}], 0)
		check(panel.status_label.text == "Entity not available in this session", "recovered selection fetch reports an absent entity")
		panel.hide()
		panel.show()
		check(panel._subscribed_session == 0 and session.sent.back()[1][0].method == "godot.subscribe", "reopening an interrupted pane requests a fresh subscription")
		client._capture("bevy:rpc", [Fixtures.snapshot()], 0)
		check(panel.rows == complete and panel.status_label.text == "7 entities", "fresh snapshot completes after interruption")
	client._capture("bevy:rpc", [partial], 0)
	panel.select_entity(Fixtures.entity_ref("999"), false)
	panel.hide()
	check(panel._pending_selection.is_empty() and panel._snapshot_rows.is_empty(), "hiding the pane discards staged selection and rows")
	panel.shutdown()
	client.shutdown()
	panel.free()
	adapter.free()
	completed.append("snapshot recovery")

func _snapshot_selection(host: Node) -> void:
	var panel = PanelScene.instantiate()
	host.add_child(panel)
	var client = FakeClient.new()
	var remote = FakeRemote.new()
	host.add_child(remote)
	panel.client = client
	panel.remote = remote
	var a = Fixtures.row("1", "A", ["game::GodotNodeHandle"], null, true)
	var b = Fixtures.row("2", "B", ["game::Speed"])
	var first := {"snapshot": true, "snapshot_index": 0, "snapshot_complete": false, "added": [a]}
	var last := {"snapshot": true, "snapshot_index": 1, "snapshot_complete": true, "added": [b]}
	for source in ["pane", "Remote", "local"]:
		panel.apply_summary({"snapshot": true, "added": []})
		panel.selected_entity = {}
		panel.apply_summary(first)
		var before_completion: int = client.frames.size()
		if source == "Remote":
			panel._remote_selected(["101"], 0)
			check(client.frames.back().method == "godot.entity_for_node", "Remote selection resolves its runtime entity while loading")
			client.reply(client.frames.back().id, {"result": a.entity})
			before_completion += 1
		else:
			panel.select_entity(a.entity, source == "pane")
		check(client.frames.size() == before_completion and panel.selected_entity.is_empty(), source + " selection waits for snapshot completion without querying")
		panel.apply_summary(last)
		check(panel.selected_entity == a.entity and panel._pending_selection.is_empty(), source + " selection replays after snapshot completion")
		check(client.frames.size() == before_completion + (1 if source == "pane" else 0), source + " deferred selection preserves inspection intent")
		if source == "pane":
			check(client.frames.back().method == "godot.get_components" and client.frames.back().params.entity == a.entity, "deferred pane inspection targets the selected entity")
	panel.apply_summary(first)
	var sent: int = client.frames.size()
	panel.select_entity(Fixtures.entity_ref("999"), false)
	check(client.frames.size() == sent, "absent entity fetch waits for final chunk")
	panel.apply_summary(last)
	check(client.frames.size() == sent + 1 and client.frames.back().method == "godot.query", "still-absent selection is fetched after completion")
	client.reply(client.frames.back().id, {"result": {"total": 0, "entities": []}})
	check(panel.status_label.text == "Entity not available in this session", "still-absent deferred selection reports its result")
	panel.apply_summary(first)
	panel.select_entity(Fixtures.entity_ref("999"), false)
	panel.select_entity(b.entity, false)
	sent = client.frames.size()
	panel.apply_summary(last)
	check(panel.selected_entity == b.entity and client.frames.size() == sent, "newer visible selection cancels an older deferred request")
	panel.apply_summary(first)
	panel.select_entity(Fixtures.entity_ref("999"), false)
	panel.cancel_inspection()
	panel.apply_summary(last)
	check(client.frames.size() == sent and panel._pending_selection.is_empty(), "local selection cancellation discards deferred work before resolution")
	panel.apply_summary({"snapshot": true, "added": []})
	panel.apply_summary(first)
	panel.select_entity(a.entity, false)
	panel.select_entity(b.entity, false)
	panel.apply_summary(last)
	check(panel.selected_entity == b.entity and client.frames.size() == sent, "latest of two deferred selections wins")
	panel.shutdown()
	panel.free()
	remote.free()
	completed.append("snapshot selection")

func _detached_inspector(host: Node) -> void:
	var section = load("res://addons/godot-bevy/bevy_inspector_section.gd").new()
	host.add_child(section)
	var client = FakeClient.new()
	section.configure(client, null, null, Fixtures.entity_ref("4294967298"), 0)
	client.reply(1, {"result": {}})
	client.active_session_id = -1
	client.active_session_changed.emit(-1)
	client.active_session_id = 0
	client.active_session_changed.emit(0)
	section.refresh()
	check(section._session_detached and client.frames.size() == 1, "reused session index cannot revive an old Inspector target")
	section.free()
	completed.append("detached inspector")

func _handshake() -> void:
	var client = RpcClient.new()
	var session = FakeSession.new()
	var errors: Array = []
	client.connection_error.connect(func(reason): errors.append(reason))
	client.setup_session(0, session)
	session.started.emit()
	client.advance(0.499)
	check(session.sent.is_empty(), "fallback waits 500 ms")
	client.advance(0.001)
	check(session.sent.size() == 1 and session.sent[0][1][0].method == "godot.debugger_config", "lost ready triggers config fallback")
	var frame: Dictionary = session.sent[0][1][0]
	client.advance(0.5)
	check(session.sent.size() == 2 and session.sent[1][1][0] == frame, "no config reply resends after 500 ms with same id")
	check(client._pending.size() == 1, "config retries keep one pending callback")
	client._capture("bevy:rpc", [{"jsonrpc": "2.0", "method": "godot.ready", "params": {}}], 0)
	check(session.sent.size() == 3, "ready immediately requests config during retry wait")
	client._capture("bevy:rpc", [{"jsonrpc": "2.0", "id": frame.id, "result": {"enabled": true, "update_interval": 0.25}}], 0)
	client.advance(25.0)
	check(session.sent.size() == 3 and client.config_ready and client.update_interval == 0.25 and client._pending.is_empty(), "config reply cancels retries and pending callback")
	client._capture("bevy:rpc", [{"jsonrpc": "2.0", "method": "godot.ready", "params": {}}], 0)
	check(session.sent.size() == 3, "duplicate ready after config does not restart handshake")
	session.stopped.emit()
	session.started.emit()
	var before: int = session.sent.size()
	client._capture("bevy:rpc", [{"jsonrpc": "2.0", "method": "godot.ready", "params": {}}], 0)
	check(session.sent.size() == before + 1 and not client.config_ready, "reused session resets config and ready requests immediately")
	session.stopped.emit()
	client.advance(1.0)
	client._capture("bevy:rpc", [{"jsonrpc": "2.0", "method": "godot.ready", "params": {}}], 0)
	check(session.sent.size() == before + 1 and client._pending.is_empty(), "session stop cancels retries and ignores ready")
	session.started.emit()
	for i in 40:
		client.advance(0.5)
	before = session.sent.size()
	client.advance(10.0)
	check(session.sent.size() == before and not client.config_ready and client._pending.is_empty() and errors.size() == 1 and errors[0].contains("20 s"), "unanswered config stops after 20 seconds with visible error")
	client.shutdown()
	completed.append("handshake")

func _selection_echo(host: Node) -> void:
	var panel = PanelScene.instantiate()
	host.add_child(panel)
	var client = FakeClient.new()
	var remote = FakeRemote.new()
	host.add_child(remote)
	panel.client = client
	panel.remote = remote
	remote.selection_changed.connect(panel._remote_selected)
	var a = Fixtures.row("1", "A", ["game::GodotNodeHandle"], null, true)
	var b = Fixtures.row("2", "B", ["game::GodotNodeHandle"], null, true)
	panel.apply_summary({"snapshot": true, "added": [a, b]})
	panel.select_entity(a.entity)
	client.reply(1, {"result": {"game::GodotNodeHandle": {"kind": "node", "valid": true, "instance_id": "101"}}})
	panel.select_entity(b.entity)
	var serial: int = panel._selection_serial
	await host.get_tree().process_frame
	check(panel._selection_serial == serial and client.frames.size() == 2 and panel.selected_entity == b.entity, "older pane Remote echo cannot cancel or reselect over newer selection")
	client.reply(2, {"result": {"game::GodotNodeHandle": {"kind": "node", "valid": true, "instance_id": "102"}}})
	check(remote.requested == [["101", 0], ["102", 0]], "newer pane selection reaches Remote after older echo")
	await host.get_tree().process_frame
	check(panel._selection_serial == serial and client.frames.size() == 2, "current pane echo does not start reverse lookup")
	check(panel._remote_selection_echoes.is_empty(), "deferred echo guards expire after notification")
	panel.shutdown()
	check(panel.rows.is_empty() and panel.items.is_empty() and panel.selected_entity.is_empty() and panel.status_label.text == "no running session", "pane shutdown clears the populated tree, selection and status")
	panel.free()
	remote.free()
	completed.append("selection echo")

func _local_cancellation(host: Node) -> void:
	var panel = PanelScene.instantiate()
	host.add_child(panel)
	var client = FakeClient.new()
	var remote = FakeRemote.new()
	host.add_child(remote)
	panel.client = client
	panel.remote = remote
	var a = Fixtures.row("1", "A", ["game::GodotNodeHandle"], null, true)
	var b = Fixtures.row("2", "B", ["game::Speed"])
	panel.apply_summary({"snapshot": true, "added": [a, b]})
	panel.select_entity(a.entity)
	panel.select_entity(b.entity, false)
	client.reply(1, {"result": {"game::GodotNodeHandle": {"kind": "node", "valid": true, "instance_id": "101"}}})
	check(remote.requested.is_empty() and panel.selected_entity == b.entity, "highlight-only selection cancels older pane inspection")
	panel.select_entity(a.entity)
	panel.cancel_inspection()
	client.reply(2, {"result": {"game::GodotNodeHandle": {"kind": "node", "valid": true, "instance_id": "101"}}})
	check(remote.requested.is_empty(), "local selection event cancels pane work before resolution returns")
	panel.shutdown()
	panel.free()
	remote.free()
	completed.append("local cancellation")

func _dock_tabs(host: Node) -> void:
	var tabs := TabContainer.new()
	host.add_child(tabs)
	var dock := VBoxContainer.new()
	var pane := Panel.new()
	tabs.add_child(dock)
	tabs.add_child(pane)
	tabs.current_tab = 1
	var remote = RemoteTree.new()
	remote.pane = pane
	check(not remote._show_dock(dock) and tabs.current_tab == 1 and tabs.get_current_tab_control() == pane, "Remote reveal never switches a TabContainer containing Entities")
	check(remote.reason.contains("shares"), "shared dock explains unavailable Remote sync")
	tabs.remove_child(pane)
	host.add_child(pane)
	check(remote._show_dock(dock) and tabs.current_tab == 0, "Remote reveal switches an independent Scene dock")
	remote.tree = Tree.new()
	var root = remote.tree.create_item()
	root.set_metadata(0, 101)
	check(remote.node_path("101") == "" and remote.reason.contains("path metadata"), "missing version-specific node path reports unavailability")
	root.set_meta("node_path", NodePath("/root/MainMenu"))
	check(remote.node_path("101") == "/root/MainMenu", "Remote node path metadata stays exact")
	remote.tree.free()
	remote.free()
	tabs.free()
	pane.free()
	completed.append("dock tabs")

func _property(proxy, component: String, path: Array, role: String = "value") -> String:
	for property in proxy.lookup:
		var entry: Dictionary = proxy.lookup[property]
		if entry.component == component and entry.path == path and entry.role == role:
			return property
	check(false, "mapped property exists: %s %s %s" % [component, path, role])
	return ""

func _proxy_mapping() -> void:
	var proxy = Proxy.new(Fixtures.entity_ref("1"), 0)
	proxy.accept_components(Fixtures.values())
	var categories := 0
	var groups := 0
	for property in proxy.get_property_list():
		if property.usage & PROPERTY_USAGE_CATEGORY and property.name == "Bevy":
			categories += 1
		if property.usage & PROPERTY_USAGE_GROUP and not property.name.is_empty():
			groups += 1
		if proxy.lookup.has(property.name):
			check(property.usage & PROPERTY_USAGE_EDITOR and not property.usage & PROPERTY_USAGE_STORAGE, "runtime properties are editor-only observations")
			check(not property.name.get_slice("/", property.name.get_slice_count("/") - 1).is_valid_int(), "mapped leaf names cannot acquire Godot's synthetic index default")
	check(categories == 1 and groups == 6, "one Bevy category; named and multi-value components keep groups")
	var types := {"bool": TYPE_BOOL, "integer": TYPE_INT, "float": TYPE_FLOAT,
		"string": TYPE_STRING, "char": TYPE_STRING, "enum": TYPE_STRING,
		"entity": TYPE_STRING, "node": TYPE_STRING, "asset": TYPE_STRING,
		"opaque": TYPE_STRING, "unsupported": TYPE_STRING, "depth_limit": TYPE_STRING}
	for kind in types:
		var entry: Dictionary = proxy.lookup[_property(proxy, kind, [])]
		check(entry.property.type == types[kind], kind + " maps to its specified Variant type")
		if kind in ["unsupported", "depth_limit"]:
			check(proxy.get(entry.property.name) == Fixtures.values()[kind].reason, kind + " displays its supplied reason")
	for kind in ["struct", "tuple_struct", "tuple", "list", "array"]:
		var path: Array = [{"field": "speed"}] if kind == "struct" else [{"index": 0}]
		var property := _property(proxy, kind, path)
		check(not proxy.lookup[property].property.usage & PROPERTY_USAGE_READ_ONLY, kind + " aggregate reason does not disable writable children")
	check(proxy.get(_property(proxy, "list", [], "truncated")) == "2 more (read limit)", "list truncation is visible")
	for kind in ["map", "set"]:
		for entry in proxy.lookup.values():
			if entry.component == kind:
				check(entry.property.usage & PROPERTY_USAGE_READ_ONLY, kind + " entries and descendants stay read-only")
	var values := Fixtures.presentation()
	proxy.accept_components(values)
	var component := "game::VeryLongComponentNameThatMustRemainReadableInANarrowInspector"
	var path: Array = [{"field": "one"}, {"field": "two"}, {"field": "three"}, {"field": "four"}, {"field": "five"}]
	var deep := _property(proxy, component, path)
	check(proxy.lookup[deep].sections.size() == 4 and proxy.lookup[deep].label == "four › five", "deep paths stop at three sections below component and use breadcrumb labels")
	check(proxy.lookup[deep].copy_path == JSON.stringify(path), "breadcrumb retains the complete exact path for copying")
	var names: Dictionary = {}
	for entry in proxy.lookup.values():
		check(not names.has(entry.property.name), "property names are globally unique")
		names[entry.property.name] = true
	var left := _property(proxy, "left::Same", [{"field": "a/b"}])
	var right := _property(proxy, "right::Same", [{"field": "a/b"}])
	check(left != right and proxy.lookup[left].sections[0].label != proxy.lookup[right].sections[0].label, "colliding component names get distinct properties and group labels")
	var wide := _property(proxy, "game::Wide", [])
	check(proxy.lookup[wide].property.type == TYPE_STRING and proxy.get(wide) == values["game::Wide"].value, "wide integer stays exact decimal text")
	check(proxy.lookup[wide].reason.contains("0") and proxy.lookup[wide].reason.contains("100"), "wide integer bounds remain visible")
	var ranged := _property(proxy, "game::Range", [])
	check(proxy.lookup[ranged].property.hint == PROPERTY_HINT_RANGE and proxy.lookup[ranged].property.hint_string == "0,10,0.01", "float range metadata uses adjustment increment without zero step")
	var variant := _property(proxy, "game::Payload", [])
	check(proxy.get(variant) == "Moving" and proxy.lookup[variant].model.unit_variants == ["Idle"], "payload enum retains current variant outside permitted switches")
	check(proxy.lookup[_property(proxy, "game::Payload", [{"variant": "Moving"}, {"field": "speed"}])].path == [{"variant": "Moving"}, {"field": "speed"}], "mapping preserves enum variant guard")
	var collision := Fixtures.aggregate("struct", {"fields": [
		{"name": "a › b", "value": Fixtures.scalar("bool", true)},
		{"name": "a", "value": Fixtures.aggregate("struct", {"fields": [{"name": "b", "value": Fixtures.scalar("bool", false)}]})}]})
	for field in ["three", "two", "one"]:
		collision = Fixtures.aggregate("struct", {"fields": [{"name": field, "value": collision}]})
	proxy.accept_components({"game::Collision": collision})
	var base: Array = [{"field": "one"}, {"field": "two"}, {"field": "three"}]
	var flat := _property(proxy, "game::Collision", base + [{"field": "a › b"}])
	var nested := _property(proxy, "game::Collision", base + [{"field": "a"}, {"field": "b"}])
	check(flat != nested and proxy.get(flat) and not proxy.get(nested), "breadcrumb text cannot collide with a literal field name")
	proxy.invalidate()
	completed.append("proxy mapping")

func _component_presentation() -> void:
	var fake = FakeClient.new()
	var proxy = Proxy.new(Fixtures.entity_ref("1"), 0)
	proxy.configure(fake)
	var generic := "bevy_state::state::resources::PreviousState<platformer_2d_example::GameState>"
	var values := Fixtures.presentation()
	values["game::Named"] = Fixtures.aggregate("struct", {"fields": [{"name": "speed", "value": Fixtures.scalar("float", 5.0)}]})
	values["game::Named"].writable.reason = "aggregate replacement denied"
	values["game::Pair"] = Fixtures.aggregate("tuple_struct", {"fields": [Fixtures.scalar("float", 1.0), Fixtures.scalar("float", 2.0)]})
	values["game::NestedTuple"] = Fixtures.aggregate("tuple_struct", {"fields": [values["game::Named"]]})
	values["game::Empty"] = Fixtures.aggregate("struct", {"fields": []})
	values["game::Restricted"] = Fixtures.aggregate("struct", {"fields": [{"name": "speed", "value": Fixtures.scalar("float", 5.0, false)}]})
	values["game::Restricted"].writable.reason = "component is immutable"
	values["game::DuplicateReason"] = values["game::Restricted"].duplicate(true)
	values["game::DuplicateReason"].writable.reason = "InspectorReadOnly"
	proxy.accept_components(values)
	var speed := _property(proxy, "game::Speed", [{"index": 0}])
	var entry: Dictionary = proxy.lookup[speed]
	check(entry.label == "Speed" and entry.sections.is_empty() and not speed.contains("/") and proxy.get(speed) == 275.0, "singleton unnamed leaf is one Speed row with no generated group")
	check(proxy.lookup[_property(proxy, "game::Text", [])].label == "Text" and proxy.lookup[_property(proxy, "game::Text", [])].sections.is_empty(), "scalar component uses its component name directly")
	var rows := 0
	for item in proxy.lookup.values():
		if item.component == "game::Speed":
			rows += 1
		check(not item.role in ["type", "reason"] and not item.label in ["Type", "Whole value"], "type and aggregate reason never consume property rows")
	check(rows == 1, "Speed has exactly one row")
	for component in ["game::Named", "game::Pair", "game::NestedTuple", "game::Empty"]:
		var found := false
		for item in proxy.lookup.values():
			if item.component == component:
				found = not item.sections.is_empty()
		check(found, component + " remains visible as a group")
	var previous := _property(proxy, generic, [{"field": "previous"}])
	check(proxy.lookup[previous].sections[0].label == "PreviousState<GameState>", "qualified generic type retains outer type and argument in label")
	check(proxy.component_label("game::Speed") == "Speed", "plain qualified type label is Speed")
	check(proxy.component_label("a::Outer<b::Inner<c::Value>, (d::Left, e::Right)>") == "Outer<Inner<Value>, (Left, Right)>", "nested generics and tuple arguments retain their structure")
	check(proxy.section_tooltips[proxy.lookup[previous].sections[0].key] == generic, "component tooltip retains the exact full generic type")
	var named := _property(proxy, "game::Named", [{"field": "speed"}])
	check(proxy.section_tooltips[proxy.lookup[named].sections[0].key] == "game::Named", "fully editable leaves suppress the aggregate reason")
	var restricted := _property(proxy, "game::Restricted", [{"field": "speed"}])
	check(proxy.section_tooltips[proxy.lookup[restricted].sections[0].key].contains("component is immutable"), "distinct aggregate restriction is available on its group tooltip")
	check(proxy.lookup[restricted].reason == "InspectorReadOnly", "leaf restriction remains on its property")
	var duplicate := _property(proxy, "game::DuplicateReason", [{"field": "speed"}])
	check(proxy.section_tooltips[proxy.lookup[duplicate].sections[0].key] == "game::DuplicateReason", "group does not repeat a restriction already visible on its leaf")
	check(proxy.has_method("_property_can_revert"), "proxy explicitly disables undefined runtime reverts")
	for property in proxy.lookup:
		check(not proxy.property_can_revert(property), "runtime property cannot revert: " + property)
	proxy.set(speed, 300.0)
	check(fake.frames[0].method == "godot.mutate_leaf" and fake.frames[0].params == {"entity": Fixtures.entity_ref("1"), "component": "game::Speed", "path": [{"index": 0}], "value": 300.0}, "flattened Speed retains the exact indexed mutation path")
	fake.request("godot.get_components", {"entity": Fixtures.entity_ref("1")})
	check(fake.last_frame("godot.mutate_leaf") == fake.frames[0] and fake.last_frame("missing").is_empty(), "probe frame lookup skips later reads and reports absent methods")
	proxy.invalidate()
	proxy = Proxy.new()
	proxy.accept_components({"a::State<a::Game>": values["game::Named"], "b::State<b::Game>": values["game::Named"]})
	var a := _property(proxy, "a::State<a::Game>", [{"field": "speed"}])
	var b := _property(proxy, "b::State<b::Game>", [{"field": "speed"}])
	check(proxy.lookup[a].sections[0].label == "a::State<a::Game>" and proxy.lookup[b].sections[0].label == "b::State<b::Game>", "colliding generic labels expand to full paths")
	proxy.invalidate()
	completed.append("component presentation")

func _proxy_edits() -> void:
	var fake = FakeClient.new()
	var proxy = Proxy.new(Fixtures.entity_ref("1"), 0)
	proxy.configure(fake)
	proxy.accept_components({"game::Speed": Fixtures.scalar("float", 0.125), "game::Text": Fixtures.scalar("string", "accepted")})
	var number := _property(proxy, "game::Speed", [])
	var text := _property(proxy, "game::Text", [])
	var shapes: Array = []
	proxy.property_list_changed.connect(func(): shapes.append(true))
	for i in 10:
		check(proxy.get(number) == 0.125, "getter reads accepted cache")
	check(fake.frames.is_empty(), "getter performs no RPC")
	var states: Array = []
	proxy.field_changed.connect(func(name): states.append([name, proxy.states[name].pending]))
	check(proxy._set(number, 0.001), "set returns handled locally")
	check(proxy.states[number].pending and states == [[number, true]] and proxy.get(number) == 0.125, "submission publishes pending in the same frame and preserves accepted cache")
	check(fake.frames[0].params == {"entity": Fixtures.entity_ref("1"), "component": "game::Speed", "path": [], "value": 0.001}, "proxy sends exact unquantized mutation frame")
	proxy.set(number, 9.0)
	check(fake.frames.size() == 1, "pending field cannot submit twice")
	fake.reply(1, {"result": Fixtures.scalar("float", 0.0625)})
	check(proxy.get(number) == 0.0625 and proxy.states[number].status == "Accepted" and not proxy.states[number].pending, "acknowledgement value replaces candidate exactly")
	check(fake.frames.back().method == "godot.get_components", "acknowledgement requests a fresh sample")
	var old_read: int = proxy._read_id
	proxy.set(number, 0.125)
	fake.reply(proxy.states[number].request, {"error": {"message": "rejected", "data": {"reason": "path shape changed"}}})
	check(proxy.get(number) == 0.0625 and proxy.states[number].status == "path shape changed" and proxy.states[number].rejected, "rejection retains accepted value and exact reason")
	check(not fake.callbacks.has(old_read), "acknowledgement discards a read begun before it")
	proxy.states[text].focused = true
	proxy.set_candidate(text, "half typed", 4, 1, 4)
	proxy.accept_components({"game::Speed": Fixtures.scalar("float", 0.25), "game::Text": Fixtures.scalar("string", "sample")})
	check(proxy.get(number) == 0.25 and proxy.get(text) == "accepted" and proxy.states[text].candidate == "half typed", "refresh updates other values while retaining focused accepted value and candidate")
	check(shapes.is_empty(), "value changes never notify the property list")
	proxy.restore_candidate(text)
	check(proxy.states[text].candidate == "accepted", "Escape restores accepted text without a mutation")
	proxy.states[text].focused = false
	proxy.accept_components({"game::Speed": Fixtures.scalar("float", 0.25, false), "game::Text": Fixtures.scalar("string", "sample")})
	check(shapes.size() == 1, "permission changes rebuild the property list")
	var sent: int = fake.frames.size()
	proxy.set(number, 1.0)
	check(fake.frames.size() == sent and proxy.states[number].status == "InspectorReadOnly", "read-only attempts keep reason and send nothing")
	proxy.invalidate()
	check(fake.callbacks.is_empty(), "invalidating proxy cancels owned requests")
	completed.append("proxy edits")

func _proxy_ownership(host: Node) -> void:
	var panel = PanelScene.instantiate()
	host.add_child(panel)
	panel.client = FakeClient.new()
	var reference := Fixtures.entity_ref("1")
	var proxy = panel.proxy_for(reference, 0)
	proxy.accept_components({"game::Text": Fixtures.scalar("string", "accepted")})
	var name := _property(proxy, "game::Text", [])
	proxy.set_candidate(name, "candidate", 3, 0, 3)
	proxy.selected_property = name
	panel._proxy = proxy
	panel._clear_proxy()
	check(panel.proxy_for(reference, 0) == proxy and proxy.states[name].candidate == "candidate" and proxy.selected_property == name, "pane retains cache, text and selection after disposable inspection is cleared")
	var control = ValueEditor.new()
	host.add_child(control)
	control.configure(proxy.components["game::Text"], reference, "game::Text", [], panel.client, 0, "", proxy)
	control.editor.text_submitted.emit("submitted")
	check(proxy.states[name].pending and not control.editor.editable, "fallback field disables immediately through the shared proxy")
	var request: int = proxy.states[name].request
	control.free()
	panel.client.reply(request, {"result": Fixtures.scalar("string", "accepted after teardown")})
	check(proxy.get(name) == "accepted after teardown" and not proxy.states[name].pending, "freeing a fallback row does not cancel or lose its mutation acknowledgement")
	panel.client.sessions[0].order = 2
	var replacement = panel.proxy_for(reference, 0)
	check(replacement != proxy and proxy.detached, "new session incarnation cannot reuse old proxy state")
	panel.shutdown()
	check(replacement.detached and panel.proxies.is_empty(), "shutdown invalidates all pane-owned proxies")
	panel.free()
	completed.append("proxy ownership")

func _proxy_paths() -> void:
	var fake = FakeClient.new()
	var proxy = Proxy.new(Fixtures.entity_ref("1"), 0)
	proxy.configure(fake)
	var values := Fixtures.presentation()
	values["game::Wide"].value = "340282366920938463463374607431768211455"
	values["game::Wide"].erase("range")
	proxy.accept_components(values)
	var wide := _property(proxy, "game::Wide", [])
	proxy.set(wide, values["game::Wide"].value)
	var mutation: Dictionary = fake.last_frame("godot.mutate_leaf")
	check(mutation.get("id", -1) == proxy.states[wide].request and mutation.get("params", {}).get("value") == values["game::Wide"].value and mutation.get("params", {}).get("value") is String, "proxy sends the full u128 decimal string without conversion")
	fake.reply(proxy.states[wide].request, {"result": values["game::Wide"]})
	check(proxy.get(wide) == values["game::Wide"].value, "proxy accepts the full u128 decimal string exactly")
	var path: Array = [{"variant": "Moving"}, {"field": "speed"}]
	var leaf := _property(proxy, "game::Payload", path)
	proxy.lookup[leaf].label = "Renamed display"
	proxy.set(leaf, 9.0)
	mutation = fake.last_frame("godot.mutate_leaf")
	check(mutation.get("id", -1) == proxy.states[leaf].request and mutation.get("params", {}) == {"entity": Fixtures.entity_ref("1"), "component": "game::Payload", "path": path, "value": 9.0}, "display names never reconstruct guarded mutation paths")
	fake.reply(proxy.states[leaf].request, {"result": Fixtures.scalar("float", 8.5)})
	check(proxy.get(leaf) == 8.5, "guarded leaf displays the response value instead of the candidate")
	proxy.invalidate()
	completed.append("proxy paths")
