@tool
extends EditorDebuggerPlugin
## Bevy Entity Inspector - EditorDebuggerPlugin
##
## This plugin adds a "Bevy Entities" tab to the debugger panel that shows
## all Bevy entities and their components at runtime.

# UI elements per session - initialize inline
var _session_uis := {}
var selected_entity_bits: int = 0

func _has_capture(prefix: String) -> bool:
	# Handle all messages prefixed with "bevy:"
	return prefix == "bevy"

func _capture(message: String, data: Array, session_id: int) -> bool:
	# Ensure UI exists for this session
	_ensure_session_ui(session_id)

	match message:
		"bevy:entities":
			_handle_entities_update(session_id, data)
			return true
		"bevy:components":
			_handle_components_update(session_id, data)
			return true
		_:
			return false

func _setup_session(session_id: int) -> void:
	print("BevyDebugger: _setup_session called with session_id: ", session_id)
	_create_session_ui(session_id)

func _ensure_session_ui(session_id: int) -> void:
	if not _session_uis.has(session_id):
		print("BevyDebugger: Creating UI lazily for session: ", session_id)
		_create_session_ui(session_id)

func _create_session_ui(session_id: int) -> void:
	if _session_uis.has(session_id):
		return

	var session = get_session(session_id)
	if not session:
		print("BevyDebugger: Could not get session: ", session_id)
		return

	# Create the main UI container
	var main_container := HSplitContainer.new()
	main_container.name = "Bevy Entities"
	main_container.split_offset = 300

	# Left side: Entity hierarchy tree
	var left_panel := VBoxContainer.new()
	left_panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	var header := HBoxContainer.new()
	var title := Label.new()
	title.text = "Entities"
	header.add_child(title)

	var refresh_btn := Button.new()
	refresh_btn.text = "Refresh"
	refresh_btn.pressed.connect(_on_refresh_pressed.bind(session_id))
	header.add_child(refresh_btn)
	left_panel.add_child(header)

	var entity_tree := Tree.new()
	entity_tree.size_flags_vertical = Control.SIZE_EXPAND_FILL
	entity_tree.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	entity_tree.item_selected.connect(_on_entity_selected.bind(session_id))
	left_panel.add_child(entity_tree)

	main_container.add_child(left_panel)

	# Right side: Component inspector
	var right_panel := VBoxContainer.new()
	right_panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	var component_header := Label.new()
	component_header.text = "Components"
	right_panel.add_child(component_header)

	var component_panel := RichTextLabel.new()
	component_panel.size_flags_vertical = Control.SIZE_EXPAND_FILL
	component_panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	component_panel.bbcode_enabled = true
	component_panel.scroll_following = false
	component_panel.selection_enabled = true
	right_panel.add_child(component_panel)

	main_container.add_child(right_panel)

	# Store UI references
	_session_uis[session_id] = {
		"tree": entity_tree,
		"panel": component_panel,
		"container": main_container
	}

	# Add the tab to the debugger
	print("BevyDebugger: About to add session tab. Session active: ", session.is_active())
	session.add_session_tab(main_container)
	print("BevyDebugger: Added session tab for session: ", session_id)
	print("BevyDebugger: Container visible: ", main_container.visible, " in tree: ", main_container.is_inside_tree())

	# Connect to session signals
	# Use a lambda to capture session_id since stopped signal takes no args
	session.started.connect(func(): _on_session_started(session_id))
	session.stopped.connect(func(): _on_session_stopped(session_id))

func _on_session_started(session_id: int) -> void:
	print("BevyDebugger: Session started: ", session_id)
	# Make sure container is visible when session starts
	if _session_uis.has(session_id):
		var ui = _session_uis[session_id]
		if ui.container:
			ui.container.visible = true
			var parent = ui.container.get_parent()
			print("BevyDebugger: Made container visible for session: ", session_id)
			print("BevyDebugger: Container parent: ", parent, " parent class: ", parent.get_class() if parent else "none")
			if parent:
				print("BevyDebugger: Parent children count: ", parent.get_child_count())

func _on_session_stopped(session_id: int) -> void:
	print("BevyDebugger: Session stopped: ", session_id)
	# Clear UI when game stops
	if _session_uis.has(session_id):
		var ui = _session_uis[session_id]
		if ui.tree:
			ui.tree.clear()
		if ui.panel:
			ui.panel.clear()

func _on_refresh_pressed(session_id: int) -> void:
	var session = get_session(session_id)
	if session and session.is_active():
		session.send_message("bevy:request_entities", [])

func _on_entity_selected(session_id: int) -> void:
	if not _session_uis.has(session_id):
		return
	var ui = _session_uis[session_id]
	var tree: Tree = ui.tree
	var selected: TreeItem = tree.get_selected()
	if not selected:
		return

	# Get entity bits from metadata
	var entity_bits: int = selected.get_metadata(0)
	selected_entity_bits = entity_bits

	# Request components for this entity
	var session = get_session(session_id)
	if session and session.is_active():
		session.send_message("bevy:request_components", [entity_bits])

func _handle_entities_update(session_id: int, data: Array) -> void:
	if not _session_uis.has(session_id):
		print("BevyDebugger: No UI for session: ", session_id)
		return

	var ui = _session_uis[session_id]
	var entity_tree: Tree = ui.tree

	if not entity_tree:
		print("BevyDebugger: entity_tree is null for session: ", session_id)
		return

	entity_tree.clear()
	var root: TreeItem = entity_tree.create_item()
	root.set_text(0, "World (%d entities)" % data.size())

	# Data format: Array of [entity_bits: int, name: String, has_godot_node: bool]
	for entity_data in data:
		if entity_data is Array and entity_data.size() >= 3:
			var entity_bits: int = entity_data[0]
			var entity_name: String = entity_data[1]
			var has_godot_node: bool = entity_data[2]

			var item: TreeItem = entity_tree.create_item(root)
			var display_name: String = entity_name if entity_name else "Entity %d" % (entity_bits & 0xFFFFFFFF)

			# Add indicator if entity has a Godot node
			if has_godot_node:
				display_name += " [G]"

			item.set_text(0, display_name)
			item.set_metadata(0, entity_bits)

func _handle_components_update(session_id: int, data: Array) -> void:
	if not _session_uis.has(session_id):
		return

	var ui = _session_uis[session_id]
	var component_panel: RichTextLabel = ui.panel

	if not component_panel:
		return

	component_panel.clear()

	# Data format: Array of [component_name: String, component_data: String]
	for component_data in data:
		if component_data is Array and component_data.size() >= 2:
			var component_name: String = component_data[0]
			var component_value: String = component_data[1]

			# Format with BBCode
			component_panel.append_text("[b][color=cyan]%s[/color][/b]\n" % component_name)
			component_panel.append_text("%s\n\n" % component_value)
