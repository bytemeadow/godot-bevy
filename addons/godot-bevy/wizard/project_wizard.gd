@tool
extends ConfirmationDialog

signal project_created(project_info: Dictionary)

@onready var project_name_input: LineEdit = $VBox/ProjectName/LineEdit
@onready var version_input: LineEdit = $VBox/Version/LineEdit
@onready var use_defaults_check: CheckBox = $VBox/UseDefaults
@onready var release_build_check: CheckBox = $VBox/ReleaseBuild
@onready var features_container: VBoxContainer = $VBox/FeaturesContainer

var feature_checkboxes: Dictionary = {}

func _ready():
	title = "Setup godot-bevy Project"
	get_ok_button().text = "Create Project"
	get_cancel_button().text = "Cancel"
	
	# Set defaults
	project_name_input.text = "my_game"
	version_input.text = "0.8.4"
	use_defaults_check.button_pressed = true
	
	# Connect signals
	get_ok_button().pressed.connect(_on_create_pressed)
	use_defaults_check.toggled.connect(_on_use_defaults_toggled)
	
	# Create feature checkboxes
	_create_feature_checkboxes()
	
	# Initially hide custom features if using defaults
	_on_use_defaults_toggled(true)

func _create_feature_checkboxes():
	var features = [
		{"name": "GodotAssetsPlugin", "description": "Asset loading through Bevy", "default": true},
		{"name": "GodotTransformSyncPlugin", "description": "Transform synchronization", "default": true},
		{"name": "GodotCollisionsPlugin", "description": "Collision detection", "default": true},
		{"name": "GodotSignalsPlugin", "description": "Signal event bridge", "default": true},
		{"name": "BevyInputBridgePlugin", "description": "Bevy input API", "default": true},
		{"name": "GodotAudioPlugin", "description": "Audio system", "default": true},
		{"name": "GodotPackedScenePlugin", "description": "Scene spawning", "default": true},
		{"name": "bevy_gamepad", "description": "Gamepad support (adds GilrsPlugin)", "default": true}
	]
	
	for feature in features:
		var checkbox = CheckBox.new()
		checkbox.text = feature.name + " - " + feature.description
		checkbox.button_pressed = feature.default
		feature_checkboxes[feature.name] = checkbox
		features_container.add_child(checkbox)

func _on_use_defaults_toggled(pressed: bool):
	features_container.visible = not pressed

func _on_create_pressed():
	var info = {
		"project_name": project_name_input.text,
		"godot_bevy_version": version_input.text,
		"use_defaults": use_defaults_check.button_pressed,
		"release_build": release_build_check.button_pressed,
		"features": {}
	}
	
	if not info.use_defaults:
		for feature_name in feature_checkboxes:
			info.features[feature_name] = feature_checkboxes[feature_name].button_pressed
	
	project_created.emit(info)
	hide()
