@tool
extends EditorInspector

var proxy
var _column_reference: WeakRef

func _ready() -> void:
	horizontal_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	vertical_scroll_mode = ScrollContainer.SCROLL_MODE_DISABLED
	proxy.sections_repaired.connect(_bind_columns)
	edit(proxy)
	proxy.attach_inspector(self)

func _bind_columns() -> void:
	var ancestor := get_parent()
	while ancestor != null and not ancestor is EditorInspector:
		ancestor = ancestor.get_parent()
	var outer := ancestor as EditorInspector
	if outer == null:
		return
	if not outer.sort_children.is_connected(_bind_columns):
		outer.sort_children.connect(_bind_columns)
	var reference: EditorProperty
	for candidate in outer.find_children("*", "EditorProperty", true, false):
		if not is_ancestor_of(candidate) and candidate.get_edited_object() == outer.get_edited_object() and candidate.is_visible_in_tree() and candidate.draw_label:
			reference = candidate
			break
	if reference == null:
		return
	var previous: EditorProperty = _column_reference.get_ref() if _column_reference != null else null
	_column_reference = weakref(reference)
	for row in find_children("*", "EditorProperty", true, false):
		if row.get_edited_object() != proxy:
			continue
		var align := _align_column.bind(row)
		if not row.pre_sort_children.is_connected(align):
			row.pre_sort_children.connect(align)
		if previous != null and previous != reference and previous.sort_children.is_connected(row.queue_sort):
			previous.sort_children.disconnect(row.queue_sort)
		if not reference.sort_children.is_connected(row.queue_sort):
			reference.sort_children.connect(row.queue_sort)
		row.queue_sort()

func _align_column(row: EditorProperty) -> void:
	var reference: EditorProperty = _column_reference.get_ref()
	if reference == null or not reference.is_visible_in_tree() or reference.size.x <= 0 or row.size.x <= 0:
		return
	var value: Control
	# Skip the label at x=0 and any bottom editor; right accessories follow the value.
	for child in reference.get_children():
		if child is Control and child.visible and child.size.x > 0 and child.position.x > 0 and is_zero_approx(child.position.y):
			if value == null or child.position.x < value.position.x:
				value = child
	if value == null:
		return
	var value_width: float = row.global_position.x + row.size.x - value.global_position.x
	# EditorProperty truncates child_room to an integer; aim at the middle of that pixel.
	row.name_split_ratio = clampf(1.0 - (value_width + 0.5) / row.size.x, 0.0, 1.0)
