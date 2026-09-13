@tool
extends RefCounted

static func create(proxy, property: Dictionary) -> EditorProperty:
	return EditorInspector.instantiate_property_editor(proxy, property.type, property.name,
		property.hint, property.hint_string, property.usage)
