extends Node

## Command-line interface for running automated benchmarks
## Usage: godot --headless -- --implementation=godot --boid-count=1000 --duration=10 --output=results.json

signal benchmark_completed(results: Dictionary)

# Command line arguments
var args: Dictionary = {}

# Benchmark parameters
var implementation: String = "godot"
var boid_count: int = 1000
var duration: float = 10.0
var output_file: String = ""
var headless: bool = false

# Benchmark state
var start_time: float = 0.0
var frame_times: Array[float] = []
var is_running: bool = false
var warmup_time: float = 0.0
var warmup_complete: bool = true  # Default to true, set to false when starting benchmark
var last_process_call: float = 0.0

# References
var main_controller: Control = null
var godot_boids: Node2D = null
var bevy_boids: Node2D = null

func _ready():
	# Parse command line arguments
	_parse_command_line()

	# Check if we're in headless mode
	headless = OS.has_feature("headless") or args.has("headless") or DisplayServer.get_name() == "headless"

	print("� Headless mode: %s" % headless)
	print("� Args found: %s" % args)

	if headless or args.size() > 0:
		print("� Running in benchmark mode")
		_setup_headless_benchmark()

func _parse_command_line():
	var cmd_args = OS.get_cmdline_args()
	print("� Command line args: %s" % cmd_args)

	# Godot includes the script arguments in the cmdline args
	# Look for our custom arguments that start with "--"
	for arg in cmd_args:
		if arg.begins_with("--") and arg.contains("="):
			var parts = arg.split("=", true, 1)
			if parts.size() == 2:
				var key = parts[0].substr(2)  # Remove "--"
				var value = parts[1]
				# Only accept our known arguments
				if key in ["implementation", "boid-count", "duration", "output", "headless"]:
					args[key] = value
					print("   Found arg: %s = %s" % [key, value])

	# Apply parsed arguments
	if args.has("implementation"):
		implementation = args["implementation"].to_lower()

	if args.has("boid-count"):
		boid_count = args["boid-count"].to_int()

	if args.has("duration"):
		duration = args["duration"].to_float()

	if args.has("output"):
		output_file = args["output"]

	print("� Benchmark Configuration:")
	print("   Implementation: %s" % implementation)
	print("   Boid Count: %d" % boid_count)
	print("   Duration: %.1f seconds" % duration)
	if output_file:
		print("   Output File: %s" % output_file)

func _setup_headless_benchmark():
	print("� Setting up headless benchmark...")

	# In headless mode, we need to load the main scene manually
	if headless:
		var main_scene = load("res://scenes/main.tscn")
		if main_scene:
			print("� Loading main scene...")
			var main_instance = main_scene.instantiate()
			get_tree().root.add_child(main_instance)
			# Wait for scene to be ready
			await get_tree().process_frame
			await get_tree().process_frame
		else:
			push_error("Could not load main scene!")
			get_tree().quit(1)
			return

	# Wait another frame for everything to initialize
	await get_tree().process_frame

	# Find the boids implementations
	print("� Looking for boids containers...")
	godot_boids = get_node_or_null("/root/Main/GodotBoidsContainer")
	bevy_boids = get_node_or_null("/root/Main/BevyBoidsContainer")

	if not godot_boids:
		print("❌ Could not find GodotBoidsContainer at /root/Main/GodotBoidsContainer")
		# Try alternative paths
		for node in get_tree().get_nodes_in_group("_boids_containers"):
			print("   Found node in group: %s" % node.get_path())

	if not bevy_boids:
		print("❌ Could not find BevyBoidsContainer at /root/Main/BevyBoidsContainer")

	if not godot_boids or not bevy_boids:
		push_error("Could not find boids containers!")
		get_tree().quit(1)
		return

	print("✅ Found both containers")

	# Start the benchmark
	_start_headless_benchmark()

func _start_headless_benchmark():
	print("\n� Starting benchmark...")
	print("   Implementation: %s" % implementation)
	print("   Boid count: %d" % boid_count)
	print("   Duration: %.1f seconds" % duration)

	# Start the appropriate implementation
	match implementation:
		"godot":
			if godot_boids.has_method("start_benchmark"):
				print("✅ Starting Godot benchmark...")
				godot_boids.start_benchmark(boid_count)
			else:
				push_error("GodotBoids does not have start_benchmark method!")
				get_tree().quit(1)
		"bevy", "rust":
			if bevy_boids.has_method("start_benchmark"):
				print("✅ Starting Bevy benchmark...")
				bevy_boids.start_benchmark(boid_count)
			else:
				push_error("BevyBoids does not have start_benchmark method!")
				get_tree().quit(1)
		_:
			push_error("Unknown implementation: %s" % implementation)
			get_tree().quit(1)

	# Wait for boids to spawn before starting measurement
	print("⏳ Waiting for boids to spawn...")
	_wait_for_boid_spawn()

func _wait_for_boid_spawn():
	warmup_time = 0.0
	warmup_complete = false

func _process(_delta: float):
	# You can't always trust the delta passed into _process to calculate FPS and frame times since it
	# becomes innacurate at very low fps due to https://github.com/godotengine/godot/issues/24624,
	# a good way to demonstrate this is to set godot's max fps to 1 and observe the values
	var delta = (Time.get_ticks_msec() - last_process_call) / 1000.0;
	last_process_call = Time.get_ticks_msec();
	# print("official: %.3f  ours %.3f" % [_delta, delta]);

	if not warmup_complete:
		_handle_warmup(delta)
		return

	if not is_running:
		return

	# Track frame time
	frame_times.append(delta)

	# Check if benchmark is complete
	var elapsed = (Time.get_ticks_msec() / 1000.0) - start_time

	# Print progress every second
	if int(elapsed) != int(elapsed - delta):
		var current_boid_count = 0
		match implementation:
			"godot":
				if godot_boids and godot_boids.has_method("get_boid_count"):
					current_boid_count = godot_boids.get_boid_count()
			"bevy", "rust":
				if bevy_boids and bevy_boids.has_method("get_boid_count"):
					current_boid_count = bevy_boids.get_boid_count()

		var fps = Engine.get_frames_per_second()
		print("⏱️  Progress: %.1f/%d seconds | Boids: %d | FPS: %.1f" % [elapsed, duration, current_boid_count, fps])

	if elapsed >= duration:
		_complete_benchmark()

func _handle_warmup(delta: float):
	warmup_time += delta

	# Check current boid count
	var current_boid_count = 0
	match implementation:
		"godot":
			if godot_boids and godot_boids.has_method("get_boid_count"):
				current_boid_count = godot_boids.get_boid_count()
		"bevy", "rust":
			if bevy_boids and bevy_boids.has_method("get_boid_count"):
				current_boid_count = bevy_boids.get_boid_count()

	# Print progress every second during warmup
	if int(warmup_time) != int(warmup_time - delta):
		print("⏳ Warmup: %d/%d boids spawned (%.1fs)" % [current_boid_count, boid_count, warmup_time])

	# Check if we've reached target count or timeout
	if current_boid_count >= boid_count:
		print("✅ Target boid count reached! Starting measurement...")
		warmup_complete = true
		is_running = true
		start_time = Time.get_ticks_msec() / 1000.0
		frame_times.clear()
	elif warmup_time > _get_warmup_timeout():
		print("⚠️  Warmup timeout! Only spawned %d/%d boids. Starting measurement anyway..." % [current_boid_count, boid_count])
		warmup_complete = true
		is_running = true
		start_time = Time.get_ticks_msec() / 1000.0
		frame_times.clear()

func _get_warmup_timeout() -> float:
	# Scale timeout based on boid count - larger counts need more time
	return min(120.0, max(30.0, boid_count / 200.0))  # 30-120s based on boid count

func _complete_benchmark():
	print("\n� Benchmark complete!")
	is_running = false

	# Stop the benchmark
	match implementation:
		"godot":
			if godot_boids and godot_boids.has_method("stop_benchmark"):
				godot_boids.stop_benchmark()
		"bevy", "rust":
			if bevy_boids and bevy_boids.has_method("stop_benchmark"):
				bevy_boids.stop_benchmark()

	# Calculate results
	var results = _calculate_results()

	# Output results
	_output_results(results)

	# Emit completion signal
	benchmark_completed.emit(results)

	# Quit if in headless mode
	if headless:
		print("� Exiting...")
		get_tree().quit(0)

func _calculate_results() -> Dictionary:
	# Calculate statistics
	var total_time = 0.0
	var min_frame_time = INF
	var max_frame_time = 0.0

	for frame_time in frame_times:
		total_time += frame_time
		min_frame_time = min(min_frame_time, frame_time)
		max_frame_time = max(max_frame_time, frame_time)

	var avg_frame_time = total_time / frame_times.size() if frame_times.size() > 0 else 0.0

	# Calculate FPS values
	var avg_fps = 1.0 / avg_frame_time if avg_frame_time > 0 else 0.0
	var min_fps = 1.0 / max_frame_time if max_frame_time > 0 else 0.0
	var max_fps = 1.0 / min_frame_time if min_frame_time > 0 else 0.0

	# Calculate percentiles
	var sorted_times = frame_times.duplicate()
	sorted_times.sort()

	var p50_index = int(sorted_times.size() * 0.5)
	var p95_index = int(sorted_times.size() * 0.95)
	var p99_index = int(sorted_times.size() * 0.99)

	var p50_frame_time = sorted_times[p50_index] if p50_index < sorted_times.size() else 0.0
	var p95_frame_time = sorted_times[p95_index] if p95_index < sorted_times.size() else 0.0
	var p99_frame_time = sorted_times[p99_index] if p99_index < sorted_times.size() else 0.0

	return {
		"implementation": implementation,
		"boid_count": boid_count,
		"duration": duration,
		"frame_count": frame_times.size(),
		"avg_fps": avg_fps,
		"min_fps": min_fps,
		"max_fps": max_fps,
		"p50_fps": 1.0 / p50_frame_time if p50_frame_time > 0 else 0.0,
		"p95_fps": 1.0 / p95_frame_time if p95_frame_time > 0 else 0.0,
		"p99_fps": 1.0 / p99_frame_time if p99_frame_time > 0 else 0.0,
		"avg_frame_time_ms": avg_frame_time * 1000.0,
		"min_frame_time_ms": min_frame_time * 1000.0,
		"max_frame_time_ms": max_frame_time * 1000.0,
		"timestamp": Time.get_datetime_string_from_system()
	}

func _output_results(results: Dictionary):
	print("\n� Benchmark Results:")
	print("   Implementation: %s" % results.implementation)
	print("   Boid Count: %d" % results.boid_count)
	print("   Duration: %.1f seconds" % results.duration)
	print("   Frame Count: %d" % results.frame_count)
	print("\n   Performance Metrics:")
	print("   Average FPS: %.1f" % results.avg_fps)
	print("   Min FPS: %.1f" % results.min_fps)
	print("   Max FPS: %.1f" % results.max_fps)
	print("   Median (p50) FPS: %.1f" % results.p50_fps)
	print("   95th Percentile FPS: %.1f" % results.p95_fps)
	print("   99th Percentile FPS: %.1f" % results.p99_fps)
	print("\n   Frame Times:")
	print("   Average: %.2f ms" % results.avg_frame_time_ms)
	print("   Min: %.2f ms" % results.min_frame_time_ms)
	print("   Max: %.2f ms" % results.max_frame_time_ms)

	# Save to file if requested
	if output_file:
		_save_results_to_file(results)

func _save_results_to_file(results: Dictionary):
	# Ensure the directory exists
	var dir = DirAccess.open(".")
	if dir:
		var output_dir = output_file.get_base_dir()
		if output_dir != "" and not dir.dir_exists(output_dir):
			print("� Creating directory: %s" % output_dir)
			dir.make_dir_recursive(output_dir)

	# Save the file
	var file = FileAccess.open(output_file, FileAccess.WRITE)
	if file:
		# Save as JSON
		JSON.stringify(results, "\t")
		file.store_string(JSON.stringify(results, "\t"))
		file.close()
		print("\n� Results saved to: %s" % output_file)

		# Double-check file exists
		if FileAccess.file_exists(output_file):
			print("✅ File verified at: %s" % output_file)
		else:
			push_error("File was not created properly!")
	else:
		var error = FileAccess.get_open_error()
		push_error("Failed to open output file: %s (error: %s)" % [output_file, error])
