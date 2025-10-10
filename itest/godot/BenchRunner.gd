# Benchmark runner for godot-bevy
# This script orchestrates the execution of Rust-based benchmarks
# and ensures benchmarks run in headless mode only

extends Node
class_name BenchRunner

func _ready():
	# Ensure benchmarks are run in headless mode only (not in editor)
	if Engine.is_editor_hint() || DisplayServer.get_name() != 'headless':
		push_error("Benchmarks must be run in headless mode (without editor).")
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

	# Create the benchmark runner
	var rust_runner = ClassDB.instantiate("IntegrationTests")

	# Run all benchmarks
	print("Running benchmarks...")
	rust_runner.run_all_benchmarks(self)

	# Benchmarks are synchronous, so we can quit immediately after
	get_tree().quit()
