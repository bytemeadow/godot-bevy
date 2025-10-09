# Test runner for godot-bevy integration tests
# This script orchestrates the execution of Rust-based tests
# and ensures tests run in headless mode only

extends Node
class_name TestRunner

func _ready():
	# Ensure tests are run in headless mode only (not in editor)
	if Engine.is_editor_hint() || DisplayServer.get_name() != 'headless':
		push_error("Integration tests must be run in headless mode (without editor).")
		get_tree().quit(2)
		return

	# Wait for physics to initialize to ensure extensions are loaded
	await get_tree().physics_frame

	print("Checking for IntegrationTests class...")

	# Check if the class exists
	if not ClassDB.class_exists("IntegrationTests"):
		push_error("IntegrationTests class not found - extension may not be loaded")
		get_tree().quit(2)
		return

	print("Found IntegrationTests class, creating instance...")

	# Create the test runner
	var rust_runner = ClassDB.instantiate("IntegrationTests")

	# Run all tests (async - tests will complete and call quit())
	print("Running tests...")
	rust_runner.run_all_tests(self)
