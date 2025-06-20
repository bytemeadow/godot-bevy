extends Control

## Main controller for the boids performance benchmark
## Coordinates between Godot-only and godot-bevy implementations

@onready var implementation_option: OptionButton = $UI/VBoxContainer/ImplementationContainer/ImplementationOption
@onready var boid_count_slider: HSlider = $UI/VBoxContainer/BoidCountContainer/BoidCountSlider
@onready var boid_count_label: Label = $UI/VBoxContainer/BoidCountContainer/BoidCountLabel
@onready var start_button: Button = $UI/VBoxContainer/ControlsContainer/StartButton
@onready var stop_button: Button = $UI/VBoxContainer/ControlsContainer/StopButton
@onready var reset_button: Button = $UI/VBoxContainer/ControlsContainer/ResetButton

@onready var fps_label: Label = $UI/VBoxContainer/PerformanceContainer/FPSLabel
@onready var avg_fps_label: Label = $UI/VBoxContainer/PerformanceContainer/AvgFPSLabel
@onready var min_fps_label: Label = $UI/VBoxContainer/PerformanceContainer/MinFPSLabel
@onready var max_fps_label: Label = $UI/VBoxContainer/PerformanceContainer/MaxFPSLabel
@onready var boids_count_label: Label = $UI/VBoxContainer/PerformanceContainer/BoidsCountLabel
@onready var benchmark_status: Label = $UI/VBoxContainer/PerformanceContainer/BenchmarkStatus

@onready var godot_boids: Node2D = $GodotBoidsContainer
@onready var bevy_boids: Node2D = $BevyBoidsContainer

enum Implementation {
	GODOT = 0,
	BEVY = 1
}

var current_implementation: Implementation = Implementation.GODOT
var is_benchmark_running: bool = false
var target_boid_count: int = 20000

# Performance tracking
var frame_times: Array[float] = []
var max_samples: int = 300  # 5 seconds at 60 FPS
var performance_start_time: float = 0.0
var min_fps: float = INF
var max_fps: float = 0.0

func _ready():
	# Set initial UI state
	stop_button.disabled = true
	_update_boid_count_label()
	_update_status("Ready")

	# Set up performance tracking
	reset_performance_metrics()

	# Initialize Bevy benchmark interface (commented out for now)
	# _initialize_bevy_interface()

	print("🎮 Boids Performance Benchmark Ready!")
	print("   - Switch between Godot (GDScript) and godot-bevy (Rust + ECS)")
	print("   - Adjust boid count to test performance limits")
	print("   - Compare FPS metrics between implementations")

func _process(_delta):
	if is_benchmark_running:
		_update_performance_metrics()

func _update_performance_metrics():
	var current_fps = Engine.get_frames_per_second()
	var avg_fps = current_fps
	var min_fps_val = min_fps
	var max_fps_val = max_fps
	var current_boid_count = 0

	# Use Godot's metrics for Godot implementation
	# Track frame times for rolling average
	var frame_time = 1.0 / max(current_fps, 1.0)
	frame_times.append(frame_time)

	if frame_times.size() > max_samples:
		frame_times.pop_front()

	# Update min/max FPS
	if current_fps < min_fps and current_fps > 0:
		min_fps = current_fps
	if current_fps > max_fps:
		max_fps = current_fps

	# Calculate average FPS
	var avg_frame_time = 0.0
	for time in frame_times:
		avg_frame_time += time
	avg_frame_time /= frame_times.size()
	avg_fps = 1.0 / avg_frame_time
	min_fps_val = min_fps
	max_fps_val = max_fps

	match current_implementation:
		Implementation.GODOT:
			current_boid_count = godot_boids.get_boid_count()
		Implementation.BEVY:
			current_boid_count = bevy_boids.get_boid_count()

	# Update UI
	fps_label.text = "FPS: %.1f" % current_fps
	avg_fps_label.text = "Avg FPS: %.1f" % avg_fps
	min_fps_label.text = "Min FPS: %.1f" % min_fps_val
	max_fps_label.text = "Max FPS: %.1f" % max_fps_val
	boids_count_label.text = "Active Boids: %d" % current_boid_count

func reset_performance_metrics():
	frame_times.clear()
	min_fps = INF
	max_fps = 0.0
	performance_start_time = Time.get_ticks_msec() / 1000.0

func _update_boid_count_label():
	boid_count_label.text = str(int(boid_count_slider.value))

func _update_status(status: String):
	benchmark_status.text = "Status: " + status

## UI Event Handlers

func _on_implementation_changed(index: int):
	current_implementation = index as Implementation

	if is_benchmark_running:
		_stop_current_benchmark()

	match current_implementation:
		Implementation.GODOT:
			_update_status("Switched to Godot (GDScript)")
		Implementation.BEVY:
			_update_status("Switched to godot-bevy (Rust + ECS)")

func _on_boid_count_changed(value: float):
	target_boid_count = int(value)
	_update_boid_count_label()

	# Update active implementation if benchmark is running
	if is_benchmark_running:
		match current_implementation:
			Implementation.GODOT:
				godot_boids.set_target_boid_count(target_boid_count)
			Implementation.BEVY:
				bevy_boids.set_target_boid_count(target_boid_count)

func _on_start_pressed():
	_start_benchmark()

func _on_stop_pressed():
	_stop_benchmark()

func _on_reset_pressed():
	reset_performance_metrics()
	_update_status("Metrics reset")

## Benchmark Control

func _start_benchmark():
	if is_benchmark_running:
		return

	is_benchmark_running = true
	start_button.disabled = true
	stop_button.disabled = false
	implementation_option.disabled = true

	reset_performance_metrics()

	match current_implementation:
		Implementation.GODOT:
			_start_godot_benchmark()
		Implementation.BEVY:
			_start_bevy_benchmark()

func _stop_benchmark():
	if not is_benchmark_running:
		return

	_stop_current_benchmark()

	is_benchmark_running = false
	start_button.disabled = false
	stop_button.disabled = true
	implementation_option.disabled = false

	_update_status("Stopped")

func _start_godot_benchmark():
	_update_status("Running Godot benchmark...")
	godot_boids.start_benchmark(target_boid_count)

func _start_bevy_benchmark():
	_update_status("Running godot-bevy benchmark...")
	bevy_boids.start_benchmark(target_boid_count)

func _stop_current_benchmark():
	match current_implementation:
		Implementation.GODOT:
			godot_boids.stop_benchmark()
		Implementation.BEVY:
			bevy_boids.stop_benchmark()

## Performance Comparison Utilities

func get_performance_summary() -> Dictionary:
	var avg_frame_time = 0.0
	if frame_times.size() > 0:
		for time in frame_times:
			avg_frame_time += time
		avg_frame_time /= frame_times.size()

	return {
		"implementation": Implementation.keys()[current_implementation],
		"boid_count": target_boid_count,
		"avg_fps": 1.0 / avg_frame_time if avg_frame_time > 0 else 0.0,
		"min_fps": min_fps if min_fps != INF else 0.0,
		"max_fps": max_fps,
		"sample_count": frame_times.size(),
		"duration_seconds": frame_times.size() / 60.0
	}

func print_performance_summary():
	var summary = get_performance_summary()
	print("\n📊 Performance Summary:")
	print("   Implementation: %s" % summary.implementation)
	print("   Boid Count: %d" % summary.boid_count)
	print("   Average FPS: %.1f" % summary.avg_fps)
	print("   Min FPS: %.1f" % summary.min_fps)
	print("   Max FPS: %.1f" % summary.max_fps)
	print("   Duration: %.1f seconds" % summary.duration_seconds)
	print("   Samples: %d" % summary.sample_count)

func _initialize_bevy_interface():
	# Try to create the Bevy benchmark interface (not implemented yet)
	print("⚠️ BoidsBenchmark class not found - Bevy implementation unavailable")
	print("   (This will be available once the Rust extension is properly integrated)")
