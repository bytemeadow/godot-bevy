# Test runner for godot-bevy integration tests
# This script orchestrates the execution of Rust-based tests
# and ensures tests run in headless mode only
#
# Copy this file and TestRunner.tscn to your test project's godot/ directory.
# If you used a custom name with declare_test_runner!(CustomName),
# update test_class_name to match.

extends Node
class_name TestRunner

# Configure this to match your test class name if you used
# declare_test_runner!(CustomName) instead of the default
@export var test_class_name: String = "IntegrationTests"

func _ready():
	# Ensure tests are run in headless mode only (not in editor)
	if Engine.is_editor_hint() || DisplayServer.get_name() != 'headless':
		push_error("Integration tests must be run in headless mode (without editor).")
		get_tree().quit(2)
		return

	# Wait for physics to initialize to ensure extensions are loaded
	await get_tree().physics_frame

	print("Checking for %s class..." % test_class_name)

	# Check if the class exists
	if not ClassDB.class_exists(test_class_name):
		push_error("%s class not found - extension may not be loaded" % test_class_name)
		get_tree().quit(2)
		return

	print("Found %s class, creating instance..." % test_class_name)

	# Create the test runner
	var rust_runner = ClassDB.instantiate(test_class_name)

	# Run all tests (async - tests will complete and call quit())
	print("Running tests...")
	rust_runner.run_all_tests(self)
