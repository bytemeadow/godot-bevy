@tool
extends EditorPlugin

const CONFIG := "res://addons/editor_probe/probe.json"
const SCENE := "res://scenes/editor_probe.tscn"

var cfg := {}

func _enter_tree() -> void:
	cfg = JSON.parse_string(FileAccess.get_file_as_string(CONFIG))
	_run.call_deferred()

func _settle(frames: int = 30) -> void:
	for i in frames:
		await get_tree().process_frame

func _shot(name: String) -> void:
	RenderingServer.force_draw()
	var image := EditorInterface.get_base_control().get_viewport().get_texture().get_image()
	DirAccess.make_dir_recursive_absolute(cfg["shots"])
	var path: String = cfg["shots"].path_join("%s.png" % name)
	image.save_png(path)
	print("EDITOR_PROBE shot=", path)

func _probe_node() -> Node:
	return EditorInterface.get_edited_scene_root().get_node(cfg.get("node_name", "Probe"))

func _run() -> void:
	await _settle()
	var cls: String = cfg["class"]
	print("EDITOR_PROBE class_exists=", ClassDB.class_exists(cls))
	if not ClassDB.class_exists(cls):
		get_tree().quit(3)
		return

	var root: Node = ClassDB.instantiate(cfg.get("root", "Node2D"))
	root.name = "EditorProbe"
	var node: Node = ClassDB.instantiate(cls)
	node.name = cfg.get("node_name", "Probe")
	root.add_child(node)
	node.owner = root
	var packed := PackedScene.new()
	packed.pack(root)
	ResourceSaver.save(packed, SCENE)
	root.free()

	EditorInterface.open_scene_from_path(SCENE)
	await _settle()
	var property: String = cfg["property"]
	var edited := _probe_node()
	print("EDITOR_PROBE default ", property, "=", edited.get(property))
	EditorInterface.edit_node(edited)
	await _settle(10)
	await _shot("1-default")

	edited.set(property, cfg["value"])
	EditorInterface.edit_node(edited)
	await _settle(10)
	await _shot("2-edited")

	EditorInterface.save_scene()
	await _settle(10)
	EditorInterface.reload_scene_from_path(SCENE)
	await _settle()
	var reloaded := _probe_node()
	var value = reloaded.get(property)
	print("EDITOR_PROBE reloaded ", property, "=", value)
	EditorInterface.edit_node(reloaded)
	await _settle(10)
	await _shot("3-reloaded")
	var verdict := 0 if str(value) == str(cfg["value"]) else 4
	print("EDITOR_PROBE verdict=", verdict)
	get_tree().quit(verdict)
