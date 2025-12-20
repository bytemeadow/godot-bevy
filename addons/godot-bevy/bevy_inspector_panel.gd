@tool
extends Panel
## Bevy Entity Inspector Panel
##
## Displays Bevy entities and their components in the editor when the game is running.

# UI elements
var entity_tree: Tree
var status_label: Label

# Track expanded state by entity_bits (persists across refreshes)
var _expanded_entities: Dictionary = {}

# Editor icons cache
var _icon_entity: Texture2D
var _icon_entity_godot: Texture2D
var _icon_component: Texture2D

# Dynamic icon cache (icon_name -> icon)
var _icon_cache: Dictionary = {}

func _ready() -> void:
	_load_icons()
	_setup_ui()

func _load_icons() -> void:
	# Load editor icons for visual presentation
	var theme := EditorInterface.get_editor_theme()
	if theme:
		_icon_entity = theme.get_icon(&"Node", &"EditorIcons")
		_icon_entity_godot = theme.get_icon(&"Godot", &"EditorIcons")
		_icon_component = theme.get_icon(&"Object", &"EditorIcons")

func _setup_ui() -> void:
	name = "Bevy"
	custom_minimum_size = Vector2(200, 200)

	var main_vbox := VBoxContainer.new()
	main_vbox.set_anchors_preset(Control.PRESET_FULL_RECT)
	main_vbox.add_theme_constant_override("separation", 4)
	add_child(main_vbox)

	# Header
	var header := HBoxContainer.new()
	var title := Label.new()
	title.text = "Bevy Entities"
	header.add_child(title)

	header.add_spacer(false)

	status_label = Label.new()
	status_label.text = "Waiting..."
	status_label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
	header.add_child(status_label)

	main_vbox.add_child(header)

	# Entity tree with hierarchy
	entity_tree = Tree.new()
	entity_tree.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	entity_tree.size_flags_vertical = Control.SIZE_EXPAND_FILL
	entity_tree.hide_root = true
	entity_tree.item_collapsed.connect(_on_item_collapsed)
	main_vbox.add_child(entity_tree)

func _on_item_collapsed(item: TreeItem) -> void:
	var entity_bits = item.get_metadata(0)
	if entity_bits != null:
		_expanded_entities[entity_bits] = not item.collapsed

func update_entities(data: Array) -> void:
	if not entity_tree:
		return

	status_label.text = "%d entities" % data.size()
	status_label.add_theme_color_override("font_color", Color(0.5, 0.9, 0.5))

	entity_tree.clear()
	var tree_root: TreeItem = entity_tree.create_item()

	# Data format: [entity_bits, name, has_godot_node, parent_bits, components]
	var entities_by_id: Dictionary = {}
	var children_by_parent: Dictionary = {}

	for entity_data in data:
		if not (entity_data is Array and entity_data.size() >= 5):
			continue

		var entity_bits: int = entity_data[0]
		var entity_name: String = entity_data[1]
		var has_godot_node: bool = entity_data[2]
		var parent_bits: int = entity_data[3]
		var components: Array = entity_data[4]

		entities_by_id[entity_bits] = {
			"name": entity_name,
			"has_godot_node": has_godot_node,
			"parent_bits": parent_bits,
			"components": components
		}

		if parent_bits == -1:
			if not children_by_parent.has(-1):
				children_by_parent[-1] = []
			children_by_parent[-1].append(entity_bits)
		else:
			if not children_by_parent.has(parent_bits):
				children_by_parent[parent_bits] = []
			children_by_parent[parent_bits].append(entity_bits)

	# Build tree recursively starting from root entities (parent_bits == -1)
	var tree_items: Dictionary = {}
	_build_entity_tree(tree_root, -1, entities_by_id, children_by_parent, tree_items)

func _build_entity_tree(parent_item: TreeItem, parent_bits: int, entities_by_id: Dictionary, children_by_parent: Dictionary, tree_items: Dictionary) -> void:
	if not children_by_parent.has(parent_bits):
		return

	for entity_bits in children_by_parent[parent_bits]:
		var info: Dictionary = entities_by_id[entity_bits]
		var entity_item: TreeItem = entity_tree.create_item(parent_item)

		var display_name: String = info["name"] if info["name"] else "Entity %d" % (entity_bits & 0xFFFFFFFF)

		entity_item.set_text(0, display_name)
		entity_item.set_metadata(0, entity_bits)
		tree_items[entity_bits] = entity_item

		# Find the node type from marker components and set appropriate icon
		var entity_icon: Texture2D = _get_entity_icon(info["components"], info["has_godot_node"])
		if entity_icon:
			entity_item.set_icon(0, entity_icon)

		# Add components as children of entity
		for component in info["components"]:
			# Skip hierarchy components - already shown visually in the tree
			if component is Dictionary:
				var comp_short_name: String = component.get("short_name", "")
				var comp_full_name: String = component.get("name", "")
				if comp_short_name in ["ChildOf", "Children"] or "::ChildOf" in comp_full_name or "::Children" in comp_full_name:
					continue
			_add_component_item(entity_item, component)

		# Recursively add child entities
		_build_entity_tree(entity_item, entity_bits, entities_by_id, children_by_parent, tree_items)

		# Restore expanded/collapsed state
		var has_children: bool = info["components"].size() > 0 or children_by_parent.has(entity_bits)
		if has_children:
			var is_expanded: bool = _expanded_entities.get(entity_bits, false)
			entity_item.collapsed = not is_expanded

func _add_component_item(parent_item: TreeItem, component) -> void:
	var comp_item: TreeItem = entity_tree.create_item(parent_item)

	# Handle both old format (string) and new format (dictionary)
	var full_name: String
	var short_name: String
	var component_value = null

	if component is Dictionary:
		full_name = component.get("name", "Unknown")
		short_name = component.get("short_name", "")
		component_value = component.get("value", null)
	else:
		full_name = str(component)
		short_name = ""

	# Fallback: extract short name from full path if not provided
	if short_name.is_empty():
		var last_sep: int = full_name.rfind("::")
		if last_sep >= 0:
			short_name = full_name.substr(last_sep + 2)
		else:
			short_name = full_name

	# Format display based on whether we have reflected value
	var display_text: String = short_name
	if component_value != null:
		var value_str: String = _format_value(component_value)
		if value_str:
			display_text = "%s: %s" % [short_name, value_str]

	comp_item.set_text(0, display_text)
	comp_item.set_tooltip_text(0, full_name)
	comp_item.set_custom_color(0, Color(0.6, 0.8, 1.0))

	# Set component icon based on type
	var icon: Texture2D = _get_component_icon(short_name)
	if icon:
		comp_item.set_icon(0, icon)

	# Add fields as children if we have structured data
	if component_value is Dictionary and component_value.has("fields"):
		_add_fields(comp_item, component_value)

func _get_icon(icon_name: String) -> Texture2D:
	# Check cache first
	if _icon_cache.has(icon_name):
		return _icon_cache[icon_name]

	# Look up icon from editor theme
	var theme := EditorInterface.get_editor_theme()
	if theme:
		var icon: Texture2D = theme.get_icon(icon_name, &"EditorIcons")
		_icon_cache[icon_name] = icon
		return icon

	_icon_cache[icon_name] = null
	return null

func _get_entity_icon(components: Array, has_godot_node: bool) -> Texture2D:
	# Look for the most specific marker component to determine the Godot node type
	# (e.g., prefer Sprite2D over Node2D over CanvasItem over Node)
	var best_node_type: StringName = &""

	for component in components:
		if not component is Dictionary:
			continue

		var short_name: String = component.get("short_name", "")

		# Fallback: extract short name from full path if not provided
		if short_name.is_empty():
			var full_name: String = component.get("name", "")
			var last_sep: int = full_name.rfind("::")
			if last_sep >= 0:
				short_name = full_name.substr(last_sep + 2)
			else:
				short_name = full_name

		# Check if this is a marker component (ends with "Marker")
		if short_name.ends_with("Marker"):
			# Extract the node type name (e.g., "Node2DMarker" -> "Node2D")
			var node_type: StringName = StringName(short_name.substr(0, short_name.length() - 6))

			if best_node_type.is_empty():
				best_node_type = node_type
			elif ClassDB.is_parent_class(node_type, best_node_type):
				# node_type is more specific (node_type inherits from best_node_type)
				best_node_type = node_type

	if not best_node_type.is_empty():
		var icon: Texture2D = _get_icon(best_node_type)
		if icon:
			return icon

	# Fallback icons
	if has_godot_node:
		return _icon_entity_godot

	return _icon_entity

func _get_component_icon(short_name: String) -> Texture2D:
	# Map component types to appropriate icons
	match short_name:
		"Transform", "GlobalTransform", "Transform2D", "Transform3D":
			return _get_icon("Transform3D")
		"GodotNodeHandle":
			return _icon_entity_godot
		"Visibility", "InheritedVisibility", "ViewVisibility":
			return _get_icon("GuiVisibilityVisible")
		"Mesh", "Mesh2d", "Mesh3d", "Handle<Mesh>":
			return _get_icon("MeshInstance3D")
		"Camera", "Camera2d", "Camera3d":
			return _get_icon("Camera3D")
		"AudioPlayer", "AudioSink", "SpatialAudioSink":
			return _get_icon("AudioStreamPlayer")
		"Name":
			return _get_icon("String")
		"TransformSyncMetadata":
			return _get_icon("VisualShaderNodeComment")
		"Groups":
			return _get_icon("Groups")
		"TransformTreeChanged":
			return _get_icon("StatusWarning")

	# Check if this is a marker component - use the corresponding Godot node icon
	if short_name.ends_with("Marker"):
		var node_type: String = short_name.substr(0, short_name.length() - 6)
		var icon: Texture2D = _get_icon(node_type)
		if icon:
			return icon

	return _icon_component

func _add_fields(parent_item: TreeItem, value_dict: Dictionary) -> void:
	var fields = value_dict.get("fields")
	if fields == null:
		return

	if fields is Dictionary:
		for field_name in fields:
			var field_value = fields[field_name]
			var field_item: TreeItem = entity_tree.create_item(parent_item)
			var display: String = "%s: %s" % [field_name, _format_value(field_value)]
			field_item.set_text(0, display)
			field_item.set_custom_color(0, Color(0.8, 0.8, 0.6))

			# Recurse for nested structs
			if field_value is Dictionary and field_value.has("fields"):
				_add_fields(field_item, field_value)
	elif fields is Array:
		for i in range(fields.size()):
			var field_value = fields[i]
			var field_item: TreeItem = entity_tree.create_item(parent_item)
			var display: String = "[%d]: %s" % [i, _format_value(field_value)]
			field_item.set_text(0, display)
			field_item.set_custom_color(0, Color(0.8, 0.8, 0.6))

func _format_value(value) -> String:
	if value == null:
		return ""

	if value is bool:
		return "true" if value else "false"

	if value is int:
		return str(value)

	if value is float:
		# Format floats nicely - avoid excessive precision
		if abs(value) < 0.0001 and value != 0.0:
			return String.num(value, 6)
		return "%.3f" % value if fmod(value, 1.0) != 0.0 else "%.1f" % value

	if value is String:
		return '"%s"' % value

	if value is Dictionary:
		var type_name: String = value.get("type", "")

		match type_name:
			"struct":
				var fields = value.get("fields", {})
				if fields.size() <= 3:
					var parts: Array = []
					for key in fields:
						parts.append("%s: %s" % [key, _format_value(fields[key])])
					return "{ %s }" % ", ".join(parts)
				return "{ %d fields }" % fields.size()
			"tuple_struct", "tuple":
				var fields = value.get("fields", [])
				if fields.size() <= 3:
					var parts: Array = []
					for f in fields:
						parts.append(_format_value(f))
					return "(%s)" % ", ".join(parts)
				return "(%d items)" % fields.size()
			"enum":
				var variant: String = value.get("variant", "?")
				var fields = value.get("fields", {})
				if fields.is_empty():
					return variant
				return "%s { ... }" % variant
			"list", "array":
				var items = value.get("items", [])
				return "[%d items]" % items.size()
			"map":
				return "{%d entries}" % value.get("len", 0)
			"set":
				return "{%d items}" % value.get("len", 0)
			"opaque":
				return value.get("debug", "?")

		return str(value)

	return str(value)
