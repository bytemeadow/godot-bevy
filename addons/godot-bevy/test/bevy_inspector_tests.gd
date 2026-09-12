extends RefCounted

const Fixtures = preload("res://addons/godot-bevy/test/bevy_inspector_fixtures.gd")
const PanelScene = preload("res://addons/godot-bevy/bevy_inspector_panel.tscn")
const ValueEditor = preload("res://addons/godot-bevy/bevy_value_editor.gd")
const RemoteTree = preload("res://addons/godot-bevy/bevy_remote_tree.gd")
const RpcClient = preload("res://addons/godot-bevy/bevy_rpc_client.gd")

class FakeClient extends RefCounted:
	signal active_session_changed(session)
	var frames: Array = []
	var callbacks: Dictionary = {}
	var active_session_id := 0
	var update_interval := 0.5
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
		await _run_test("client", _client)
		await _run_test("handshake", _handshake)
		await _run_test("subscriptions", _subscriptions.bind(host))
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
	check(panel.hidden_count == 3 and panel.hidden_label.text == "3 hidden", "default internal filter counts observer, resource and empty entity")
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
	check(panel.items["4294967299"].visible and panel.hidden_label.text == "0 hidden", "toggle reveals internal set")
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
	panel.free()
	completed.append("tree")

func _values(host: Node) -> void:
	var expected = {"integer": SpinBox, "float": SpinBox, "bool": CheckBox, "string": LineEdit,
		"char": LineEdit, "enum": OptionButton, "entity": LinkButton, "node": LinkButton,
		"asset": Label, "opaque": Label, "unsupported": Label, "depth_limit": Label}
	for kind in Fixtures.values():
		var fake = FakeClient.new()
		var control = ValueEditor.new()
		host.add_child(control)
		var value: Dictionary = Fixtures.values()[kind]
		control.configure(value, Fixtures.entity_ref("4294967298"), "game::Speed", [], fake, 0)
		if expected.has(kind):
			check(is_instance_of(control.editor, expected[kind]), kind + " renders its control")
		else:
			check(control.fold != null and control.children_editors.size() > 0, kind + " renders collapsible children")
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
			check(control.children_editors[0].editor is Label and fake.frames.is_empty(), kind + " is read-only")
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
	check(control.editor.min_value == 1.0 and control.editor.max_value == 10.0 and control.editor.step == 1.0, "SpinBox honors InspectorRange and integer step")
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
