@tool
extends Panel
## Bevy Entity Inspector Panel
##
## This panel shows Bevy entities and their components when the game is running.
## It appears as a tab next to the Scene tab in the editor.
##
## Entities are shown as expandable items with components as children,
## similar to bevy-inspector-egui.

# UI elements
var entity_tree: Tree
var status_label: Label

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

	# Entity tree with components as children
	entity_tree = Tree.new()
	entity_tree.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	entity_tree.size_flags_vertical = Control.SIZE_EXPAND_FILL
	entity_tree.hide_root = true
	main_vbox.add_child(entity_tree)

# Called by the debugger plugin when entity data is received
# Data format: Array of [entity_bits, name, has_godot_node, components]
# where components is Array of component name strings
func update_entities(data: Array) -> void:
	if not entity_tree:
		return

	status_label.text = "%d entities" % data.size()
	status_label.add_theme_color_override("font_color", Color(0.5, 0.9, 0.5))

	entity_tree.clear()
	var root: TreeItem = entity_tree.create_item()

	for entity_data in data:
		if entity_data is Array and entity_data.size() >= 3:
			var entity_bits: int = entity_data[0]
			var entity_name: String = entity_data[1]
			var has_godot_node: bool = entity_data[2]
			var components: Array = entity_data[3] if entity_data.size() >= 4 else []

			# Create entity item (expandable if has components)
			var entity_item: TreeItem = entity_tree.create_item(root)
			var display_name: String = entity_name if entity_name else "Entity %d" % (entity_bits & 0xFFFFFFFF)

			if has_godot_node:
				display_name += " [G]"

			entity_item.set_text(0, display_name)
			entity_item.set_metadata(0, entity_bits)

			# Add components as children
			for component_name in components:
				var comp_item: TreeItem = entity_tree.create_item(entity_item)

				# Extract just the short type name (after last ::)
				var short_name: String = component_name
				var last_sep: int = component_name.rfind("::")
				if last_sep >= 0:
					short_name = component_name.substr(last_sep + 2)

				comp_item.set_text(0, short_name)
				comp_item.set_tooltip_text(0, component_name)  # Full name on hover
				comp_item.set_custom_color(0, Color(0.6, 0.8, 1.0))  # Light blue for components

			# Collapse by default if there are components
			if components.size() > 0:
				entity_item.collapsed = true

# Called by the debugger plugin when component data is received
func update_components(data: Array) -> void:
	# Currently unused - component names are sent with entity data
	pass
