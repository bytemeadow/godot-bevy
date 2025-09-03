@tool
extends ConfirmationDialog

signal project_created(project_info: Dictionary)

@onready var project_name_input: LineEdit = $VBox/ProjectName/LineEdit
@onready var version_input: LineEdit = $VBox/Version/LineEdit
@onready var release_build_check: CheckBox = $VBox/ReleaseBuild

const DEFAULT_VERSION = "0.9.0"

func _ready():
	title = "Setup godot-bevy Project"
	get_ok_button().text = "Create Project"
	get_cancel_button().text = "Cancel"

	# Set defaults
	project_name_input.text = "my_game"
	version_input.text = DEFAULT_VERSION

	# Connect signals
	get_ok_button().pressed.connect(_on_create_pressed)
	
	# Force proper sizing - known Godot issue workaround
	call_deferred("_fix_dialog_size")


func _fix_dialog_size():
	# Workaround for Godot's dialog sizing issues
	# Reset minimum size to force recalculation
	min_size = Vector2.ZERO
	size = Vector2.ZERO
	# Force layout update
	await get_tree().process_frame
	# Let dialog calculate proper size
	reset_size()
	# Set fixed width but keep auto height
	size.x = 800
	min_size.x = 800

func _on_create_pressed():
	var info = {
		"project_name": project_name_input.text,
		"godot_bevy_version": version_input.text,
		"release_build": release_build_check.button_pressed
	}

	project_created.emit(info)
	hide()
