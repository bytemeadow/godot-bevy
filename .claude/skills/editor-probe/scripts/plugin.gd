@tool
extends EditorPlugin

const CONFIG := "res://addons/editor_probe/probe.json"
const SCENE := "res://scenes/editor_probe.tscn"

const TYPE_NAMES := {
	TYPE_NIL: "NIL",
	TYPE_BOOL: "BOOL",
	TYPE_INT: "INT",
	TYPE_FLOAT: "FLOAT",
	TYPE_STRING: "STRING",
	TYPE_VECTOR2: "VECTOR2",
	TYPE_VECTOR2I: "VECTOR2I",
	TYPE_RECT2: "RECT2",
	TYPE_RECT2I: "RECT2I",
	TYPE_VECTOR3: "VECTOR3",
	TYPE_VECTOR3I: "VECTOR3I",
	TYPE_TRANSFORM2D: "TRANSFORM2D",
	TYPE_VECTOR4: "VECTOR4",
	TYPE_VECTOR4I: "VECTOR4I",
	TYPE_PLANE: "PLANE",
	TYPE_QUATERNION: "QUATERNION",
	TYPE_AABB: "AABB",
	TYPE_BASIS: "BASIS",
	TYPE_TRANSFORM3D: "TRANSFORM3D",
	TYPE_PROJECTION: "PROJECTION",
	TYPE_COLOR: "COLOR",
	TYPE_STRING_NAME: "STRING_NAME",
	TYPE_NODE_PATH: "NODE_PATH",
	TYPE_RID: "RID",
	TYPE_OBJECT: "OBJECT",
	TYPE_CALLABLE: "CALLABLE",
	TYPE_SIGNAL: "SIGNAL",
	TYPE_DICTIONARY: "DICTIONARY",
	TYPE_ARRAY: "ARRAY",
	TYPE_PACKED_BYTE_ARRAY: "PACKED_BYTE_ARRAY",
	TYPE_PACKED_INT32_ARRAY: "PACKED_INT32_ARRAY",
	TYPE_PACKED_INT64_ARRAY: "PACKED_INT64_ARRAY",
	TYPE_PACKED_FLOAT32_ARRAY: "PACKED_FLOAT32_ARRAY",
	TYPE_PACKED_FLOAT64_ARRAY: "PACKED_FLOAT64_ARRAY",
	TYPE_PACKED_STRING_ARRAY: "PACKED_STRING_ARRAY",
	TYPE_PACKED_VECTOR2_ARRAY: "PACKED_VECTOR2_ARRAY",
	TYPE_PACKED_VECTOR3_ARRAY: "PACKED_VECTOR3_ARRAY",
	TYPE_PACKED_COLOR_ARRAY: "PACKED_COLOR_ARRAY",
	TYPE_PACKED_VECTOR4_ARRAY: "PACKED_VECTOR4_ARRAY",
	TYPE_MAX: "MAX",
}

const HINT_NAMES := {
	PROPERTY_HINT_NONE: "NONE",
	PROPERTY_HINT_RANGE: "RANGE",
	PROPERTY_HINT_ENUM: "ENUM",
	PROPERTY_HINT_ENUM_SUGGESTION: "ENUM_SUGGESTION",
	PROPERTY_HINT_EXP_EASING: "EXP_EASING",
	PROPERTY_HINT_LINK: "LINK",
	PROPERTY_HINT_FLAGS: "FLAGS",
	PROPERTY_HINT_LAYERS_2D_RENDER: "LAYERS_2D_RENDER",
	PROPERTY_HINT_LAYERS_2D_PHYSICS: "LAYERS_2D_PHYSICS",
	PROPERTY_HINT_LAYERS_2D_NAVIGATION: "LAYERS_2D_NAVIGATION",
	PROPERTY_HINT_LAYERS_3D_RENDER: "LAYERS_3D_RENDER",
	PROPERTY_HINT_LAYERS_3D_PHYSICS: "LAYERS_3D_PHYSICS",
	PROPERTY_HINT_LAYERS_3D_NAVIGATION: "LAYERS_3D_NAVIGATION",
	PROPERTY_HINT_LAYERS_AVOIDANCE: "LAYERS_AVOIDANCE",
	PROPERTY_HINT_FILE: "FILE",
	PROPERTY_HINT_DIR: "DIR",
	PROPERTY_HINT_GLOBAL_FILE: "GLOBAL_FILE",
	PROPERTY_HINT_GLOBAL_DIR: "GLOBAL_DIR",
	PROPERTY_HINT_RESOURCE_TYPE: "RESOURCE_TYPE",
	PROPERTY_HINT_MULTILINE_TEXT: "MULTILINE_TEXT",
	PROPERTY_HINT_EXPRESSION: "EXPRESSION",
	PROPERTY_HINT_PLACEHOLDER_TEXT: "PLACEHOLDER_TEXT",
	PROPERTY_HINT_COLOR_NO_ALPHA: "COLOR_NO_ALPHA",
	PROPERTY_HINT_OBJECT_ID: "OBJECT_ID",
	PROPERTY_HINT_TYPE_STRING: "TYPE_STRING",
	PROPERTY_HINT_NODE_PATH_TO_EDITED_NODE: "NODE_PATH_TO_EDITED_NODE",
	PROPERTY_HINT_OBJECT_TOO_BIG: "OBJECT_TOO_BIG",
	PROPERTY_HINT_NODE_PATH_VALID_TYPES: "NODE_PATH_VALID_TYPES",
	PROPERTY_HINT_SAVE_FILE: "SAVE_FILE",
	PROPERTY_HINT_GLOBAL_SAVE_FILE: "GLOBAL_SAVE_FILE",
	PROPERTY_HINT_INT_IS_OBJECTID: "INT_IS_OBJECTID",
	PROPERTY_HINT_INT_IS_POINTER: "INT_IS_POINTER",
	PROPERTY_HINT_ARRAY_TYPE: "ARRAY_TYPE",
	PROPERTY_HINT_DICTIONARY_TYPE: "DICTIONARY_TYPE",
	PROPERTY_HINT_LOCALE_ID: "LOCALE_ID",
	PROPERTY_HINT_LOCALIZABLE_STRING: "LOCALIZABLE_STRING",
	PROPERTY_HINT_NODE_TYPE: "NODE_TYPE",
	PROPERTY_HINT_HIDE_QUATERNION_EDIT: "HIDE_QUATERNION_EDIT",
	PROPERTY_HINT_PASSWORD: "PASSWORD",
	PROPERTY_HINT_TOOL_BUTTON: "TOOL_BUTTON",
	PROPERTY_HINT_ONESHOT: "ONESHOT",
	PROPERTY_HINT_GROUP_ENABLE: "GROUP_ENABLE",
	PROPERTY_HINT_INPUT_NAME: "INPUT_NAME",
	PROPERTY_HINT_FILE_PATH: "FILE_PATH",
	PROPERTY_HINT_MAX: "MAX",
}

var probes: Array = []
var cfg: Dictionary = {}
var mismatches: Array[String] = []

func _enter_tree() -> void:
	probes = JSON.parse_string(FileAccess.get_file_as_string(CONFIG))
	_run.call_deferred()

func _settle(frames: int = 30) -> void:
	for i in frames:
		await get_tree().process_frame

func _shot(name: String) -> void:
	# The idle editor does not redraw without input.
	RenderingServer.force_draw()
	var image := EditorInterface.get_base_control().get_viewport().get_texture().get_image()
	DirAccess.make_dir_recursive_absolute(cfg["shots"])
	var path: String = cfg["shots"].path_join("%s.png" % name)
	var error := image.save_png(path)
	if error != OK:
		mismatches.append("screenshot %s: %s" % [name, error_string(error)])
	print("EDITOR_PROBE shot=", path)

func _probe_node() -> Node:
	return EditorInterface.get_edited_scene_root().get_node(cfg.get("node_name", "Probe"))

func _check(key: String, actual: Variant, expected: Variant) -> void:
	var numeric := (actual is int or actual is float) and (expected is int or expected is float)
	if (typeof(actual) != typeof(expected) and not numeric) or actual != expected:
		mismatches.append("%s expected=%s actual=%s" % [key, JSON.stringify(expected), JSON.stringify(actual)])

func _check_properties(node: Node) -> bool:
	var property: String = cfg["property"]
	var entry: Dictionary = {}
	for item in node.get_property_list():
		if item["name"] == property:
			entry = item
			break
	if entry.is_empty():
		mismatches.append("property %s missing from get_property_list()" % property)
		return false
	var expect: Dictionary = cfg.get("expect", {})
	if expect.has("type"):
		_check("expect.type", TYPE_NAMES.get(entry["type"], str(entry["type"])), expect["type"])
	if expect.has("hint"):
		_check("expect.hint", HINT_NAMES.get(entry["hint"], str(entry["hint"])), expect["hint"])
	if expect.has("hint_string"):
		_check("expect.hint_string", entry["hint_string"], expect["hint_string"])
	if expect.has("default"):
		_check("expect.default", node.get(property), expect["default"])
	return true

func _help_label() -> RichTextLabel:
	var script_editor := EditorInterface.get_script_editor()
	for help in script_editor.find_children("*", "EditorHelp", true, false):
		if not help.is_visible_in_tree():
			continue
		for child in help.get_children():
			if child is RichTextLabel and child.is_visible_in_tree():
				return child
	return null

func _check_help() -> void:
	var version := Engine.get_version_info()
	if version["major"] != 4 or version["minor"] != 6:
		mismatches.append("expect.description requires Godot 4.6 help controls; got %s" % version["string"])
		return
	EditorInterface.set_main_screen_editor("Script")
	EditorInterface.get_script_editor().goto_help("class_property:%s:%s" % [cfg["class"], cfg["property"]])
	await _settle()
	var label := _help_label()
	if label == null:
		mismatches.append("expect.description: Godot 4.6 visible EditorHelp/RichTextLabel not found under ScriptEditor; build with godot-bevy/register-docs")
	else:
		var description: String = cfg["expect"]["description"]
		if not label.get_parsed_text().contains(description):
			mismatches.append("expect.description missing %s in editor help; build with godot-bevy/register-docs" % JSON.stringify(description))
	_shot("4-help")

func _probe() -> int:
	var cls: String = cfg["class"]
	if not ClassDB.class_exists(cls):
		mismatches.append("class does not exist in the editor")
		return 3
	var root: Node = ClassDB.instantiate(cfg.get("root", "Node2D"))
	root.name = "EditorProbe"
	var node: Node = ClassDB.instantiate(cls)
	node.name = cfg.get("node_name", "Probe")
	root.add_child(node)
	node.owner = root
	var found := _check_properties(node)
	if not found:
		root.free()
		return 4
	var packed := PackedScene.new()
	var error := packed.pack(root)
	if error == OK:
		error = ResourceSaver.save(packed, SCENE)
	root.free()
	if error != OK:
		mismatches.append("save initial scene: %s" % error_string(error))
		return 4

	EditorInterface.open_scene_from_path(SCENE)
	EditorInterface.reload_scene_from_path(SCENE)
	await _settle()
	var property: String = cfg["property"]
	var edited := _probe_node()
	EditorInterface.edit_node(edited)
	await _settle(10)
	_shot("1-default")

	edited.set(property, cfg["value"])
	EditorInterface.edit_node(edited)
	await _settle(10)
	_shot("2-edited")

	error = EditorInterface.save_scene()
	if error != OK:
		mismatches.append("save edited scene: %s" % error_string(error))
	await _settle(10)
	EditorInterface.reload_scene_from_path(SCENE)
	await _settle()
	var reloaded := _probe_node()
	_check("reload", reloaded.get(property), cfg["value"])
	EditorInterface.edit_node(reloaded)
	await _settle(10)
	_shot("3-reloaded")
	if cfg.get("expect", {}).has("description"):
		await _check_help()
	return 0 if mismatches.is_empty() else 4

func _run() -> void:
	await _settle()
	var status := 0
	for probe in probes:
		cfg = probe
		mismatches = []
		var verdict := await _probe()
		print("EDITOR_PROBE verdict=%s class=%s property=%s mismatches=%s" % [
			verdict, cfg["class"], cfg["property"], JSON.stringify(mismatches)])
		status = maxi(status, verdict)
	print("EDITOR_PROBE complete")
	get_tree().quit(status)
