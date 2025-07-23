@tool
extends EditorPlugin

const WIZARD_SCENE_PATH = "res://addons/godot-bevy/wizard/project_wizard.tscn"

var wizard_dialog: Window
var _should_restart_after_build: bool = false

func _enter_tree():
	# Add menu items
	add_tool_menu_item("Setup godot-bevy Project", _on_setup_project)
	add_tool_menu_item("Add BevyApp Singleton", _on_add_singleton)
	add_tool_menu_item("Build Rust Project", _on_build_rust)

	print("godot-bevy plugin activated!")

func _exit_tree():
	# Remove menu items
	remove_tool_menu_item("Setup godot-bevy Project")
	remove_tool_menu_item("Add BevyApp Singleton")
	remove_tool_menu_item("Build Rust Project")

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

	push_warning("BevyAppSingleton added to project autoload settings! Restart editor to apply changes.")

func _create_bevy_app_singleton(path: String):
	# Create the scene file content directly
	var scene_content = """[gd_scene format=3 uid="uid://bjsfwt816j4tp"]

[node name="BevyApp" type="BevyApp"]
"""
	
	# Save the scene file directly
	_save_file(path, scene_content)

	print("Created BevyApp singleton scene at: ", path)

func _on_project_created(project_info: Dictionary):
	# Handle the project creation based on wizard input
	_scaffold_rust_project(project_info)
	_create_bevy_app_singleton("res://bevy_app_singleton.tscn")
	_on_add_singleton()  # This will add it to autoload

	# Automatically build the Rust project and restart after
	var is_release = project_info.get("release_build", false)
	_should_restart_after_build = true
	_build_rust_project(is_release)

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

[workspace]
# Empty workspace table to make this a standalone project
""" % [_to_snake_case(project_name), info.godot_bevy_version]

	# Note: godot-bevy 0.8.4 doesn't have a "default" feature, so we just use the base dependency
	# The plugin selection will be handled in the Rust code via GodotDefaultPlugins

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
use bevy::prelude::*;
use godot_bevy::prelude::*;

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
""" % [plugin_config]

	_save_file(rust_path.path_join("src/lib.rs"), lib_content)

	# Create .gdextension file
	var gdextension_content = """[configuration]
entry_symbol = "gdext_rust_init"
compatibility_minimum = 4.1

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

	push_warning("Rust project scaffolded successfully! Building now...")

func _on_build_rust():
	# Build the Rust project (called from menu)
	_should_restart_after_build = false  # Don't restart for manual builds
	_build_rust_project(false)  # Default to debug build

func _build_rust_project(release_build: bool):
	var base_path = ProjectSettings.globalize_path("res://")
	var rust_path = base_path.path_join("rust")

	# Check if rust directory exists
	if not DirAccess.dir_exists_absolute(rust_path):
		push_error("No Rust project found! Run 'Setup godot-bevy Project' first.")
		return

	# Prepare cargo command with working directory
	var args = ["build", "--manifest-path", rust_path.path_join("Cargo.toml")]
	if release_build:
		args.append("--release")

	print("Building Rust project...")
	print("Running: cargo ", " ".join(args))

	# Execute cargo build
	var output = []
	var exit_code = OS.execute("cargo", args, output, true, true)

	# Process results
	if exit_code == 0:
		var build_type = "debug" if not release_build else "release"
		push_warning("Rust build completed successfully! (%s)" % build_type)
		print("Build output:")
		for line in output:
			print("  ", line)

		# Restart editor if this was called from project setup
		if _should_restart_after_build:
			push_warning("Restarting editor to apply autoload changes...")
			EditorInterface.restart_editor()
	else:
		push_error("Rust build failed with exit code: %d" % exit_code)
		print("Build errors:")
		for line in output:
			print("  ", line)

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
