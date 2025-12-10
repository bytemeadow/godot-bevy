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

func _ready() -> void:
	_setup_ui()

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

# Track when user expands/collapses an entity
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
		if info["has_godot_node"]:
			display_name += " [G]"

		entity_item.set_text(0, display_name)
		entity_item.set_metadata(0, entity_bits)
		tree_items[entity_bits] = entity_item

		# Add components as children of entity
		for component_name in info["components"]:
			var comp_item: TreeItem = entity_tree.create_item(entity_item)
			var short_name: String = component_name
			var last_sep: int = component_name.rfind("::")
			if last_sep >= 0:
				short_name = component_name.substr(last_sep + 2)
			comp_item.set_text(0, short_name)
			comp_item.set_tooltip_text(0, component_name)
			comp_item.set_custom_color(0, Color(0.6, 0.8, 1.0))

		# Recursively add child entities
		_build_entity_tree(entity_item, entity_bits, entities_by_id, children_by_parent, tree_items)

		# Restore expanded/collapsed state
		var has_children: bool = info["components"].size() > 0 or children_by_parent.has(entity_bits)
		if has_children:
			var is_expanded: bool = _expanded_entities.get(entity_bits, false)
			entity_item.collapsed = not is_expanded
