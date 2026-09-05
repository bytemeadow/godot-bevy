# Copy this file and BenchRunner.tscn to your test project's godot/ directory.
# If you used a custom name with declare_test_runner!(CustomName),
# update test_class_name to match.

extends Node
class_name BenchRunner

# Configure this to match your test class name if you used
# declare_test_runner!(CustomName) instead of the default
@export var test_class_name: String = "IntegrationTests"

func _ready():
	if Engine.is_editor_hint() || DisplayServer.get_name() != 'headless':
		push_error("Benchmarks must be run in headless mode (without editor).")
		get_tree().quit(2)
		return

	# Wait for physics to initialize to ensure extensions are loaded
	await get_tree().physics_frame

	print("Checking for %s class..." % test_class_name)

	if not ClassDB.class_exists(test_class_name):
		push_error("%s class not found - extension may not be loaded" % test_class_name)
		get_tree().quit(2)
		return

	print("Found %s class, creating instance..." % test_class_name)

	var rust_runner = ClassDB.instantiate(test_class_name)

	print("Running benchmarks...")
	var exit_code = rust_runner.run_all_benchmarks(self)

	# Benchmarks are synchronous, so we can quit immediately after
	get_tree().quit(exit_code)
