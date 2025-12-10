@tool
extends Panel
## Bevy Entity Inspector Panel
##
## This panel shows Bevy entities and their components when the game is running.
## It appears as a tab next to the Scene tab in the editor.

# UI elements
var entity_tree: Tree
var component_panel: RichTextLabel
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

	var refresh_btn := Button.new()
	refresh_btn.text = "Refresh"
	refresh_btn.pressed.connect(_on_refresh_pressed)
	header.add_child(refresh_btn)
	main_vbox.add_child(header)

	# Status label
	status_label = Label.new()
	status_label.text = "Waiting for game..."
	status_label.add_theme_color_override("font_color", Color(0.7, 0.7, 0.7))
	main_vbox.add_child(status_label)

	# Split container for tree and components
	var split := HSplitContainer.new()
	split.size_flags_vertical = Control.SIZE_EXPAND_FILL
	split.split_offset = 150
	main_vbox.add_child(split)

	# Entity tree
	entity_tree = Tree.new()
	entity_tree.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	entity_tree.size_flags_vertical = Control.SIZE_EXPAND_FILL
	entity_tree.item_selected.connect(_on_entity_selected)
	split.add_child(entity_tree)

	# Component panel
	var right_vbox := VBoxContainer.new()
	right_vbox.size_flags_horizontal = Control.SIZE_EXPAND_FILL

	var comp_header := Label.new()
	comp_header.text = "Components"
	right_vbox.add_child(comp_header)

	component_panel = RichTextLabel.new()
	component_panel.size_flags_vertical = Control.SIZE_EXPAND_FILL
	component_panel.size_flags_horizontal = Control.SIZE_EXPAND_FILL
	component_panel.bbcode_enabled = true
	component_panel.scroll_following = false
	component_panel.selection_enabled = true
	right_vbox.add_child(component_panel)

	split.add_child(right_vbox)

func _on_refresh_pressed() -> void:
	# TODO: Request refresh from game
	pass

func _on_entity_selected() -> void:
	var selected := entity_tree.get_selected()
	if not selected:
		return
	# TODO: Request component data for selected entity
	pass

# Called by the debugger plugin when entity data is received
func update_entities(data: Array) -> void:
	if not entity_tree:
		return

	status_label.text = "Connected"
	status_label.add_theme_color_override("font_color", Color(0.5, 0.9, 0.5))

	entity_tree.clear()
	var root: TreeItem = entity_tree.create_item()
	root.set_text(0, "World (%d entities)" % data.size())

	for entity_data in data:
		if entity_data is Array and entity_data.size() >= 3:
			var entity_bits: int = entity_data[0]
			var entity_name: String = entity_data[1]
			var has_godot_node: bool = entity_data[2]

			var item: TreeItem = entity_tree.create_item(root)
			var display_name: String = entity_name if entity_name else "Entity %d" % (entity_bits & 0xFFFFFFFF)

			if has_godot_node:
				display_name += " [G]"

			item.set_text(0, display_name)
			item.set_metadata(0, entity_bits)

# Called by the debugger plugin when component data is received
func update_components(data: Array) -> void:
	if not component_panel:
		return

	component_panel.clear()

	for component_data in data:
		if component_data is Array and component_data.size() >= 2:
			var component_name: String = component_data[0]
			var component_value: String = component_data[1]

			component_panel.append_text("[b][color=cyan]%s[/color][/b]\n" % component_name)
			component_panel.append_text("%s\n\n" % component_value)
