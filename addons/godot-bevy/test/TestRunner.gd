# Launch this scene from the installed addon with --scene; copying is not needed.
# If you used a custom name with declare_test_runner!(CustomName),
# update test_class_name to match.

extends Node
class_name TestRunner

# Configure this to match your test class name if you used
# declare_test_runner!(CustomName) instead of the default
@export var test_class_name: String = "IntegrationTests"

func _ready():
	if Engine.is_editor_hint() || DisplayServer.get_name() != 'headless':
		push_error("Integration tests must be run in headless mode (without editor).")
		get_tree().quit(2)
		return

	# Wait for physics to initialize to ensure extensions are loaded
	await get_tree().physics_frame
	if "--bevy-inspector-tests" in OS.get_cmdline_user_args():
		var script = load("res://addons/godot-bevy/test/bevy_inspector_tests.gd")
		if script == null or not script.can_instantiate():
			push_error("bevy_inspector_tests.gd failed to load")
			get_tree().quit(2)
			return
		var suite = script.new()
		get_tree().create_timer(30.0).timeout.connect(func():
			push_error("BEVY_INSPECTOR_TEST runner timed out")
			get_tree().quit(2)
		)
		var result = await suite.run(self)
		if not suite.finished:
			push_error("BEVY_INSPECTOR_TEST suite aborted before completion")
			get_tree().quit(1)
			return
		get_tree().quit(result)
		return

	print("Checking for %s class..." % test_class_name)

	if not ClassDB.class_exists(test_class_name):
		push_error("%s class not found - extension may not be loaded" % test_class_name)
		get_tree().quit(2)
		return

	print("Found %s class, creating instance..." % test_class_name)

	var rust_runner = ClassDB.instantiate(test_class_name)

	print("Running tests...")
	rust_runner.run_all_tests(self)
