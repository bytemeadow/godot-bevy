@tool
extends EditorPlugin

const WIZARD_SCENE_PATH = "res://addons/godot-bevy/wizard/project_wizard.tscn"
const AUTOLOAD_NAME = "BevyAppSingleton"
const AUTOLOAD_PATH = "res://addons/godot-bevy/bevy_app_singleton.tscn"
const BEVY_DEBUGGER_SCRIPT = "res://addons/godot-bevy/bevy_debugger_plugin.gd"
const BEVY_INSPECTOR_SCENE = "res://addons/godot-bevy/bevy_inspector_panel.tscn"

var wizard_dialog: Window
var _should_restart_after_build: bool = false
var _bevy_debugger: EditorDebuggerPlugin = null
var _bevy_inspector: Control = null
var _component_inspector: EditorInspectorPlugin
var _remote_tree: Node
var _local_selection_serial := 0

func _enable_plugin():
	add_autoload_singleton(AUTOLOAD_NAME, AUTOLOAD_PATH)
	print("godot-bevy: BevyAppSingleton autoload registered")

func _disable_plugin():
	remove_autoload_singleton(AUTOLOAD_NAME)
	print("godot-bevy: BevyAppSingleton autoload removed")

func _enter_tree():
	print("godot-bevy: _enter_tree() called")

	add_tool_menu_item("Setup godot-bevy Project", _on_setup_project)
	add_tool_menu_item("Build Rust Project", _on_build_rust)

	var inspector_scene = load(BEVY_INSPECTOR_SCENE) as PackedScene
	if inspector_scene:
		_bevy_inspector = inspector_scene.instantiate()
		add_control_to_dock(EditorPlugin.DOCK_SLOT_LEFT_BL, _bevy_inspector)
		print("godot-bevy: Bevy Inspector panel added to dock")
	else:
		push_error("godot-bevy: Failed to load Bevy Inspector scene")
		return

	_bevy_debugger = load(BEVY_DEBUGGER_SCRIPT).new()
	_remote_tree = preload("res://addons/godot-bevy/bevy_remote_tree.gd").new()
	add_child(_remote_tree)
	_remote_tree.pane = _bevy_inspector
	_bevy_inspector.setup(_bevy_debugger.client, _remote_tree)
	_component_inspector = preload("res://addons/godot-bevy/bevy_inspector_plugin.gd").new()
	_component_inspector.client = _bevy_debugger.client
	_component_inspector.pane = _bevy_inspector
	_component_inspector.remote = _remote_tree
	add_inspector_plugin(_component_inspector)
	add_debugger_plugin(_bevy_debugger)
	EditorInterface.get_selection().selection_changed.connect(_local_selection_changed)

	print("godot-bevy plugin activated!")

func _exit_tree():
	remove_tool_menu_item("Setup godot-bevy Project")
	remove_tool_menu_item("Build Rust Project")

	var selection = EditorInterface.get_selection()
	if selection.selection_changed.is_connected(_local_selection_changed):
		selection.selection_changed.disconnect(_local_selection_changed)
	if _component_inspector:
		_component_inspector.shutdown()
		remove_inspector_plugin(_component_inspector)
		_component_inspector = null
	if is_instance_valid(_bevy_inspector):
		_bevy_inspector.shutdown()
	if _bevy_debugger:
		_bevy_debugger.shutdown()
		remove_debugger_plugin(_bevy_debugger)
		_bevy_debugger = null
	if is_instance_valid(_remote_tree):
		_remote_tree.free()
	if is_instance_valid(_bevy_inspector):
		remove_control_from_docks(_bevy_inspector)
		_bevy_inspector.free()
		_bevy_inspector = null

	if wizard_dialog:
		wizard_dialog.queue_free()

func _process(delta: float) -> void:
	if _bevy_debugger != null:
		_bevy_debugger.client.advance(delta)

func _local_selection_changed() -> void:
	_bevy_inspector.cancel_inspection()
	_local_selection_serial += 1
	var serial := _local_selection_serial
	if _bevy_debugger.client.active_session_id == -1:
		return
	var selected = EditorInterface.get_selection().get_selected_nodes()
	var root = EditorInterface.get_edited_scene_root()
	if selected.size() != 1 or root == null or root.scene_file_path.is_empty():
		return
	var session: int = _bevy_debugger.client.active_session_id
	_bevy_debugger.client.request("godot.resolve_node", {
		"scene_path": String(root.scene_file_path), "node_path": String(root.get_path_to(selected[0]))
	}, func(frame):
		if serial != _local_selection_serial or session != _bevy_debugger.client.active_session_id:
			return
		if frame.has("error"):
			_bevy_inspector.status_label.text = frame.error.message
		elif frame.result.candidates.size() == 1:
			_bevy_inspector.select_entity(frame.result.candidates[0], false)
		else:
			_bevy_inspector.show_candidates(frame.result.candidates)
	, session)

func _on_setup_project():
	if not wizard_dialog:
		var wizard_scene = load(WIZARD_SCENE_PATH)
		if wizard_scene:
			wizard_dialog = wizard_scene.instantiate()
			wizard_dialog.project_created.connect(_on_project_created)
			EditorInterface.get_base_control().add_child(wizard_dialog)
		else:
			push_error("Failed to load wizard scene")

	wizard_dialog.popup_centered()


func _on_project_created(project_info: Dictionary):
	_scaffold_rust_project(project_info)

	var is_release = project_info.get("release_build", false)
	_should_restart_after_build = true
	_build_rust_project(is_release)

func _scaffold_rust_project(info: Dictionary):
	var base_path = ProjectSettings.globalize_path("res://")
	var rust_path = base_path.path_join("rust")
	var cargo_toml_path = rust_path.path_join("Cargo.toml")

	if FileAccess.file_exists(cargo_toml_path):
		push_warning("Rust project already exists at 'rust/' directory. Skipping Rust scaffolding.")
		print("Found existing Cargo.toml at: ", cargo_toml_path)
		return

	print("Project info received: ", info)
	print("Project name value: '", info.get("project_name", "KEY_NOT_FOUND"), "'")

	var project_name = info.project_name.strip_edges()
	if project_name.is_empty():
		project_name = "my_game"
		push_warning("Empty project name, using default: my_game")

	DirAccess.make_dir_recursive_absolute(rust_path.path_join("src"))

	var cargo_content = """[package]
name = "%s"
version = "0.1.0"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
bevy = { version = "0.19", default-features = false, features = ["bevy_state"] }
godot = "0.5"
godot-bevy = { version = "%s", features = ["default"] }

[workspace]
# Empty workspace table to make this a standalone project

# Optimize dependencies in debug builds for faster iteration.
[profile.dev.package."*"]
opt-level = 3
""" % [project_name.to_snake_case(), info.godot_bevy_version]

	_save_file(rust_path.path_join("Cargo.toml"), cargo_content)

	# Always use GodotDefaultPlugins for bootstrapping
	# Users can customize plugin selection in their generated code
	var plugin_config = "app.add_plugins(GodotDefaultPlugins);"

	var lib_content = """use godot::prelude::*;
use bevy::prelude::*;
use godot_bevy::prelude::*;

#[bevy_app]
fn build_app(app: &mut App) {
	// GodotDefaultPlugins provides all standard godot-bevy functionality
	// For minimal setup, use individual plugins instead:
	// app.add_plugins(GodotTransformSyncPlugin::default())
	//     .add_plugins(GodotAudioPlugin)
	//     .add_plugins(BevyInputBridgePlugin);
	%s

	app.add_systems(Update, hello_world_system);
}

fn hello_world_system(mut timer: Local<f32>, time: Res<Time>) {
	*timer += time.delta_secs();
	if *timer > 1.0 {
		*timer = 0.0;
		godot_print!("Hello from Bevy ECS!");
	}
}
""" % [plugin_config]

	_save_file(rust_path.path_join("src/lib.rs"), lib_content)

	var gdextension_content = """[configuration]
entry_symbol = "gdext_rust_init"
compatibility_minimum = 4.1
reloadable = true

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
		project_name.to_snake_case(),
		project_name.to_snake_case(),
		project_name.to_snake_case(),
		project_name.to_snake_case(),
		project_name.to_snake_case(),
		project_name.to_snake_case(),
		project_name.to_snake_case(),
		project_name.to_snake_case(),
	]

	_save_file(base_path.path_join("rust.gdextension"), gdextension_content)

	push_warning("Rust project scaffolded successfully! Building now...")

func _on_build_rust():
	_should_restart_after_build = false
	_build_rust_project(false)

func _build_rust_project(release_build: bool):
	var base_path = ProjectSettings.globalize_path("res://")
	var rust_path = base_path.path_join("rust")

	if not DirAccess.dir_exists_absolute(rust_path):
		push_error("No Rust project found! Run 'Setup godot-bevy Project' first.")
		return

	var args = ["build", "--manifest-path", rust_path.path_join("Cargo.toml")]
	if release_build:
		args.append("--release")

	print("Building Rust project...")
	print("Running: cargo ", " ".join(args))

	var output = []
	var exit_code = OS.execute("cargo", args, output, true, true)

	if exit_code == 0:
		var build_type = "debug" if not release_build else "release"
		push_warning("Rust build completed successfully! (%s)" % build_type)
		print("Build output:")
		for line in output:
			print("  ", line)

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
