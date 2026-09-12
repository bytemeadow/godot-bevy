extends Node

const SCENARIO := "browser-orbit"
const SCENE := "res://browser/orbit.tscn"

var bridge: JavaScriptObject
var advance_callback: JavaScriptObject
var stable_frames := 0
var frame := 0
var started := false
var holding := false
var processed := false


func _ready() -> void:
	if not OS.has_feature("web"):
		queue_free()
		return
	bridge = JavaScriptBridge.get_interface("browserCapture")
	if bridge == null:
		queue_free()
		return
	advance_callback = JavaScriptBridge.create_callback(_advance)
	bridge.advance = advance_callback
	bridge.godot = Engine.get_version_info().string
	bridge.threaded = OS.has_feature("threads")
	process_priority = 2147483647
	get_tree().set_physics_interpolation_enabled(false)
	# BevyApp uses ALWAYS, so tree pause alone would leave the orbit running.
	get_node("/root/BevyAppSingleton").process_mode = Node.PROCESS_MODE_PAUSABLE
	RenderingServer.frame_post_draw.connect(_after_draw)


func _process(_delta: float) -> void:
	processed = true
	if started:
		frame += 1


func _after_draw() -> void:
	if holding or not processed:
		return
	processed = false
	if not started:
		var scene := get_tree().current_scene
		if scene == null or scene.scene_file_path != SCENE or not ClassDB.class_exists("BevyApp"):
			stable_frames = 0
			return
		var sprites := scene.find_children("*", "Sprite2D", true, false)
		if sprites.size() != 6:
			stable_frames = 0
			return
		for sprite: Sprite2D in sprites:
			if not sprite.is_node_ready() or not sprite.is_visible_in_tree() or sprite.texture == null or sprite.texture.get_size() == Vector2.ZERO:
				stable_frames = 0
				return
		stable_frames += 1
		if stable_frames < 3:
			return
		started = true
		_hold()
		bridge.ready = true
		JavaScriptBridge.get_interface("console").log("CAPTURE_READY scenario=%s scene=%s frame=0" % [SCENARIO, SCENE])
	elif frame == 60:
		_hold()


func _hold() -> void:
	holding = true
	get_tree().paused = true
	bridge.frame = frame
	bridge.scene = get_tree().current_scene.scene_file_path
	bridge.paused = true


func _advance(_arguments: Array) -> void:
	if not started or not holding or frame != 0:
		return
	holding = false
	bridge.paused = false
	get_tree().paused = false
