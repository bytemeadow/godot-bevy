@tool
extends EditorPlugin

const WIZARD_SCENE_PATH = "res://addons/godot-bevy/wizard/project_wizard.tscn"

var wizard_dialog: Window

func _enter_tree():
	# Add menu items
	add_tool_menu_item("Setup godot-bevy Project", _on_setup_project)
	add_tool_menu_item("Add BevyApp Singleton", _on_add_singleton)

	print("godot-bevy plugin activated!")

func _exit_tree():
	# Remove menu items
	remove_tool_menu_item("Setup godot-bevy Project")
	remove_tool_menu_item("Add BevyApp Singleton")

	if wizard_dialog:
		wizard_dialog.queue_free()

func _on_setup_project():
	# Show project wizard dialog
	if not wizard_dialog:
		var wizard_scene = load(WIZARD_SCENE_PATH)
		if wizard_scene:
			wizard_dialog = wizard_scene.instantiate()
			wizard_dialog.project_created.connect(_on_project_created)
			EditorInterface.get_base_control().add_child(wizard_dialog)
		else:
			push_error("Failed to load wizard scene")

	wizard_dialog.popup_centered(Vector2(600, 400))

func _on_add_singleton():
	# Check if singleton already exists
	if ProjectSettings.has_setting("autoload/BevyAppSingleton"):
		push_warning("BevyAppSingleton already exists in project settings")
		return

	# Create the singleton scene if it doesn't exist
	var singleton_path = "res://bevy_app_singleton.tscn"
	if not FileAccess.file_exists(singleton_path):
		_create_bevy_app_singleton(singleton_path)

	# Add to autoload
	ProjectSettings.set_setting("autoload/BevyAppSingleton", singleton_path)
	ProjectSettings.save()

	push_warning("BevyAppSingleton added to project autoload settings!")

	# Refresh the project to apply changes
	EditorInterface.restart_editor()

func _create_bevy_app_singleton(path: String):
	# Create a new scene with BevyApp node
	var scene = PackedScene.new()
	var root = Node.new()
	root.name = "BevyAppSingleton"

	# Add script to handle the BevyApp setup
	var script_content = """
extends Node

# This singleton manages the BevyApp instance for your game.
# It's automatically loaded when the game starts.

var bevy_app: BevyApp

func _ready():
	# Create and configure the BevyApp
	bevy_app = BevyApp.new()
	bevy_app.name = "BevyApp"
	add_child(bevy_app)

	print("BevyApp singleton initialized!")

func _exit_tree():
	# Cleanup is handled automatically by BevyApp
	pass
"""

	var script = GDScript.new()
	script.source_code = script_content
	root.set_script(script)

	# Pack and save the scene
	scene.pack(root)
	ResourceSaver.save(scene, path)

	print("Created BevyApp singleton scene at: ", path)

func _on_project_created(project_info: Dictionary):
	# Handle the project creation based on wizard input
	_scaffold_rust_project(project_info)
	_create_bevy_app_singleton("res://bevy_app_singleton.tscn")
	_on_add_singleton()  # This will add it to autoload

func _scaffold_rust_project(info: Dictionary):
	var base_path = ProjectSettings.globalize_path("res://")
	var rust_path = base_path.path_join("rust")

	# Debug: Print the info dictionary
	print("Project info received: ", info)
	print("Project name value: '", info.get("project_name", "KEY_NOT_FOUND"), "'")
	
	# Validate project name
	var project_name = info.project_name.strip_edges()
	if project_name.is_empty():
		project_name = "my_game"
		push_warning("Empty project name, using default: my_game")

	# Create directory structure
	DirAccess.make_dir_recursive_absolute(rust_path.path_join("src"))

	# Create Cargo.toml
	var cargo_content = """[package]
name = "%s"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
bevy = { version = "0.16", default-features = false, features = ["bevy_state"] }
godot = "0.3"
godot-bevy = "%s"

[features]
default = []
""" % [_to_snake_case(project_name), info.godot_bevy_version]

	if info.use_defaults:
		cargo_content = cargo_content.replace('godot-bevy = "%s"' % info.godot_bevy_version,
			'godot-bevy = { version = "%s", features = ["default"] }' % info.godot_bevy_version)

	_save_file(rust_path.path_join("Cargo.toml"), cargo_content)

	# Build plugin configuration
	var plugin_config = ""
	if info.use_defaults:
		plugin_config = "app.add_plugins(GodotDefaultPlugins);"
	else:
		var plugins_to_add = []
		if info.features.get("GodotAssetsPlugin", false):
			plugins_to_add.append("app.add_plugins(GodotAssetsPlugin);")
		if info.features.get("GodotTransformSyncPlugin", false):
			plugins_to_add.append("app.add_plugins(GodotTransformSyncPlugin::default());")
		if info.features.get("GodotCollisionsPlugin", false):
			plugins_to_add.append("app.add_plugins(GodotCollisionsPlugin);")
		if info.features.get("GodotSignalsPlugin", false):
			plugins_to_add.append("app.add_plugins(GodotSignalsPlugin);")
		if info.features.get("BevyInputBridgePlugin", false):
			plugins_to_add.append("app.add_plugins(BevyInputBridgePlugin);")
		if info.features.get("GodotAudioPlugin", false):
			plugins_to_add.append("app.add_plugins(GodotAudioPlugin);")
		if info.features.get("GodotPackedScenePlugin", false):
			plugins_to_add.append("app.add_plugins(GodotPackedScenePlugin);")

		if plugins_to_add.size() > 0:
			plugin_config = "    " + "\n    ".join(plugins_to_add)
		else:
			plugin_config = "    // Add plugins as needed"

	# Update Cargo.toml for gamepad feature
	if info.features.get("bevy_gamepad", false) and not info.use_defaults:
		cargo_content = cargo_content.replace('godot-bevy = "%s"' % info.godot_bevy_version,
			'godot-bevy = { version = "%s", features = ["bevy_gamepad"] }' % info.godot_bevy_version)

	# Create lib.rs
	var lib_content = """use godot::prelude::*;
use godot_bevy::prelude::*;

#[derive(GodotClass)]
#[class(base=Node)]
struct %s;

#[godot_api]
impl INode for %s {
	fn init(base: Base<Node>) -> Self {
		Self
	}
}

#[bevy_app]
fn build_app(app: &mut App) {
%s

	// Add your systems here
	app.add_systems(Update, hello_world_system);
}

fn hello_world_system() {
	// This runs every frame in Bevy's Update schedule
	static mut COUNTER: f32 = 0.0;
	unsafe {
		COUNTER += 0.016; // Approximate frame time
		if COUNTER > 1.0 {
			COUNTER = 0.0;
			godot_print!("Hello from Bevy ECS!");
		}
	}
}

// Required for GDExtension
struct %sExtension;

#[gdextension]
unsafe impl ExtensionLibrary for %sExtension {}
""" % [
		_to_pascal_case(project_name),
		_to_pascal_case(project_name),
		plugin_config,
		_to_pascal_case(project_name),
		_to_pascal_case(project_name)
	]

	_save_file(rust_path.path_join("src/lib.rs"), lib_content)

	# Create .gdextension file
	var gdextension_content = """[configuration]
entry_symbol = "gdext_rust_init"

[libraries]
linux.debug.x86_64 = "res://rust/target/debug/lib%s.so"
linux.release.x86_64 = "res://rust/target/release/lib%s.so"
windows.debug.x86_64 = "res://rust/target/debug/%s.dll"
windows.release.x86_64 = "res://rust/target/release/%s.dll"
macos.debug = "res://rust/target/debug/lib%s.dylib"
macos.release = "res://rust/target/release/lib%s.dylib"
macos.debug.arm64 = "res://rust/target/debug/lib%s.dylib"
macos.release.arm64 = "res://rust/target/release/lib%s.dylib"
""" % [
		_to_snake_case(project_name),
		_to_snake_case(project_name),
		_to_snake_case(project_name),
		_to_snake_case(project_name),
		_to_snake_case(project_name),
		_to_snake_case(project_name),
		_to_snake_case(project_name),
		_to_snake_case(project_name)
	]

	_save_file(base_path.path_join("rust.gdextension"), gdextension_content)

	# Create build scripts for different platforms
	if OS.get_name() == "Windows":
		var build_script = """@echo off
cd rust
cargo build %s
""" % ["--release" if info.release_build else ""]
		_save_file(base_path.path_join("build.bat"), build_script)
	else:
		var build_script = """#!/bin/bash
cd rust
cargo build %s
""" % ["--release" if info.release_build else ""]
		_save_file(base_path.path_join("build.sh"), build_script)
		# Make build script executable on Unix
		OS.execute("chmod", ["+x", base_path.path_join("build.sh")])

	push_warning("Rust project scaffolded successfully! Run build.sh to compile.")

func _save_file(path: String, content: String):
	var file = FileAccess.open(path, FileAccess.WRITE)
	if file:
		file.store_string(content)
		file.close()
		print("Created: ", path)
	else:
		push_error("Failed to create file: " + path)

# Helper functions for case conversion
func _to_snake_case(text: String) -> String:
	var result = ""
	for i in range(text.length()):
		var c = text[i]
		# Check if character is uppercase by comparing with lowercase version
		if c != c.to_lower() and i > 0:
			result += "_"
		result += c.to_lower()
	return result

func _to_pascal_case(text: String) -> String:
	var words = text.split("_")
	var result = ""
	for word in words:
		if word.length() > 0:
			result += word[0].to_upper() + word.substr(1).to_lower()
	return result
