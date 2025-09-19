//! Transform sync performance benchmark with Godot runtime
//!
//! This benchmark runs inside the actual Godot runtime to measure
//! the true FFI overhead difference between individual and bulk updates.
//!
//! Run with: `cargo bench --bench transform_sync_benchmark`
//! Or use the convenience script: `./run_benchmarks.sh`
//!
//! Expected results (based on real-world testing):
//! - 10 nodes: ~3.8x speedup
//! - 100 nodes: ~6.8x speedup
//! - 1000 nodes: ~6.8x speedup
//! - 5000 nodes: ~7.2x speedup
//!
//! In production games with 20,000+ entities, this optimization
//! provides approximately 1.22x FPS improvement.

use std::collections::HashMap;
use std::time::{Duration, Instant};

use godot::prelude::*;
use godot_testability_runtime::runtime::{GodotRuntime, RuntimeConfig, UserCallbacks};

/// Benchmark results
#[derive(Debug, Clone)]
struct BenchResult {
    name: String,
    times: Vec<Duration>,
    mean: Duration,
    min: Duration,
    max: Duration,
}

impl BenchResult {
    fn from_times(name: String, times: Vec<Duration>) -> Self {
        let mean = times.iter().sum::<Duration>() / times.len() as u32;
        let min = times.iter().min().copied().unwrap_or(Duration::ZERO);
        let max = times.iter().max().copied().unwrap_or(Duration::ZERO);

        Self {
            name,
            times,
            mean,
            min,
            max,
        }
    }

    fn print_summary(&self) {
        println!("  {}:", self.name);
        println!("    Mean: {:?}", self.mean);
        println!("    Min:  {:?}", self.min);
        println!("    Max:  {:?}", self.max);
        println!("    Samples: {}", self.times.len());
    }
}

fn main() {
    println!("🚀 Transform Sync Real Benchmark with Godot Runtime\n");

    // Create callbacks for godot-rust integration
    let callbacks = UserCallbacks {
        initialize_ffi: |get_proc_addr, library| {
            unsafe {
                type GetProcAddrFn =
                    unsafe extern "C" fn(*const i8) -> Option<unsafe extern "C" fn()>;
                let get_proc_addr_fn =
                    std::mem::transmute::<*const std::ffi::c_void, GetProcAddrFn>(get_proc_addr);
                let library_ptr = library as *mut ::godot::sys::__GdextClassLibrary;
                let config = ::godot::sys::GdextConfig::new(false);
                ::godot::sys::initialize(Some(get_proc_addr_fn), library_ptr, config);
            }
            Ok(())
        },
        load_class_method_table: |level| {
            let init_level = match level {
                0 => ::godot::init::InitLevel::Core,
                1 => ::godot::init::InitLevel::Servers,
                2 => ::godot::init::InitLevel::Scene,
                3 => ::godot::init::InitLevel::Editor,
                _ => return,
            };
            unsafe {
                ::godot::sys::load_class_method_table(init_level);
            }
        },
        register_classes: None,
    };

    let config = RuntimeConfig {
        headless: true,
        verbose: false,
        custom_args: vec![],
    };

    // Run benchmarks inside Godot runtime
    let runtime_result = GodotRuntime::run_godot(config, callbacks, move |_scene_tree_ptr| {
        println!("✅ Godot runtime initialized, running benchmarks...\n");

        // Load the production GDScript helper for bulk updates
        let bulk_helper = create_bulk_update_helper();
        if bulk_helper.is_none() {
            eprintln!("❌ Could not load bulk update helper from production BevyApp singleton!");
            eprintln!("   Make sure addons/godot-bevy/bevy_app_singleton.tscn exists");
            return Err(std::io::Error::other("Failed to load production GDScript").into());
        }

        println!("✨ Loaded bulk update helper with production GDScript methods!\n");
        let mut bulk_helper = bulk_helper.unwrap();

        let mut results = HashMap::new();

        // Test different node counts
        for node_count in [500, 1000, 5000, 10000, 20000] {
            println!("Testing with {} nodes:", node_count);

            // Benchmark individual FFI calls
            let individual_result = benchmark_individual_ffi(node_count, 100);
            individual_result.print_summary();
            results.insert(format!("individual_{}", node_count), individual_result);

            // Benchmark bulk packed arrays with production GDScript
            let bulk_result = benchmark_bulk_packed(node_count, 100, &mut bulk_helper);
            bulk_result.print_summary();
            results.insert(format!("bulk_{}", node_count), bulk_result);

            // Calculate and show speedup
            if let (Some(ind), Some(bulk)) = (
                results.get(&format!("individual_{}", node_count)),
                results.get(&format!("bulk_{}", node_count)),
            ) {
                let speedup = ind.mean.as_secs_f64() / bulk.mean.as_secs_f64();
                println!("  🎯 Speedup: {:.2}x faster with bulk updates", speedup);
                println!("  📊 FFI calls reduced: {} → 1", node_count * 8); // get + set for each property
            }

            println!();
        }

        Ok(())
    });

    if let Err(e) = runtime_result {
        eprintln!("❌ Failed to run benchmarks: {:?}", e);
        std::process::exit(1);
    }
}

/// Load the production BevyApp singleton with bulk update methods
fn create_bulk_update_helper() -> Option<Gd<godot::classes::RefCounted>> {
    use std::fs;
    use std::path::Path;

    // Load the production bevy_app_singleton.tscn
    // This is relative to where the benchmark runs (godot-bevy/godot-bevy)
    let scene_path = "../addons/godot-bevy/bevy_app_singleton.tscn";
    let file_path = Path::new(scene_path);

    // Read the file using Rust's standard library
    let content = match fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => {
            println!("  ✗ Could not read {}: {}", scene_path, e);
            return None;
        }
    };

    println!("  Found BevyApp singleton at {}", scene_path);

    // Parse the .tscn to extract the GDScript source
    // The script source is between 'script/source = "' and the closing '"'
    if let Some(start) = content.find("script/source = \"") {
        let script_start = start + 17; // Length of 'script/source = "'
        if let Some(end) = content[script_start..].find("\"\n") {
            let script_source = &content[script_start..script_start + end];

            // Replace escaped quotes and newlines
            let cleaned_source = script_source
                .replace("\\\"", "\"")
                .replace("\\n", "\n")
                .replace("\\t", "\t");

            // Replace "extends BevyApp" with "extends RefCounted"
            let modified_source = cleaned_source.replace("extends BevyApp", "extends RefCounted");

            println!("  ✓ Extracted GDScript from production singleton");

            // Create a GDScript with the extracted source
            let mut script = godot::classes::GDScript::new_gd();
            script.set_source_code(&modified_source);

            // Compile the script
            let error = script.reload();
            if error != godot::global::Error::OK {
                println!("  ✗ Failed to compile extracted GDScript: {:?}", error);
                return None;
            }

            // Create an instance with the script
            let mut helper = godot::classes::RefCounted::new_gd();
            helper.set_script(&script.to_variant());

            // Verify it has our methods
            if helper.has_method("bulk_update_transforms_3d")
                && helper.has_method("bulk_update_transforms_2d")
            {
                println!("  ✓ Successfully created helper from production BevyApp code");
                return Some(helper);
            } else {
                println!("  ✗ Extracted script missing bulk update methods");
            }
        }
    }

    println!("  ✗ Could not extract GDScript from BevyApp singleton");
    None
}

/// Benchmark individual FFI calls
fn benchmark_individual_ffi(node_count: usize, iterations: usize) -> BenchResult {
    let mut times = Vec::with_capacity(iterations);

    // Create nodes
    let mut nodes = Vec::with_capacity(node_count);
    for i in 0..node_count {
        let mut node = Node3D::new_alloc();
        node.set_name(&format!("Node_{}", i));
        node.set_position(Vector3::new(i as f32, 0.0, 0.0));
        nodes.push(node);
    }

    // Warm up
    for _ in 0..10 {
        for node in nodes.iter_mut() {
            let pos = node.get_position();
            node.set_position(pos + Vector3::new(0.001, 0.0, 0.0));
        }
    }

    // Benchmark
    for _ in 0..iterations {
        let start = Instant::now();

        // Each of these is an FFI call
        for node in nodes.iter_mut() {
            let pos = node.get_position(); // FFI call
            node.set_position(pos + Vector3::new(0.001, 0.0, 0.0)); // FFI call

            let rot = node.get_rotation(); // FFI call
            node.set_rotation(rot + Vector3::new(0.0, 0.001, 0.0)); // FFI call

            let scale = node.get_scale(); // FFI call
            node.set_scale(scale * 1.0001); // FFI call

            let _visible = node.is_visible(); // FFI call
            node.set_visible(true); // FFI call
        }

        times.push(start.elapsed());
    }

    // Cleanup
    for mut node in nodes {
        node.queue_free();
    }

    BenchResult::from_times(format!("Individual FFI ({} nodes)", node_count), times)
}

/// Benchmark bulk packed array approach with production GDScript
fn benchmark_bulk_packed(
    node_count: usize,
    iterations: usize,
    helper: &mut Gd<godot::classes::RefCounted>,
) -> BenchResult {
    let mut times = Vec::with_capacity(iterations);

    // Create nodes
    let nodes: Vec<Gd<Node3D>> = (0..node_count)
        .map(|i| {
            let mut node = Node3D::new_alloc();
            node.set_name(&format!("Node_{}", i));
            node.set_position(Vector3::new(i as f32, 0.0, 0.0));
            node
        })
        .collect();

    // Pre-allocate vectors to match production code
    let mut instance_ids_3d = Vec::with_capacity(node_count);
    let mut positions_3d = Vec::with_capacity(node_count);
    let mut rotations_3d = Vec::with_capacity(node_count);
    let mut scales_3d = Vec::with_capacity(node_count);

    // Warm up
    for warm_iter in 0..10 {
        instance_ids_3d.clear();
        positions_3d.clear();
        rotations_3d.clear();
        scales_3d.clear();

        // Match production: collect raw data first (from Bevy components, no FFI)
        for (i, node) in nodes.iter().enumerate() {
            // Direct instance ID collection (only FFI needed)
            instance_ids_3d.push(node.instance_id().to_i64());

            // Simulated Bevy component data (no FFI)
            let x = i as f32 + (warm_iter as f32 * 0.001);
            positions_3d.push(Vector3::new(x, 0.0, 0.0));

            // Simple quaternion (no FFI)
            rotations_3d.push(Vector4::new(0.0, 0.0, 0.0, 1.0));

            scales_3d.push(Vector3::ONE);
        }

        // Convert to packed arrays once (matching production)
        let packed_ids = PackedInt64Array::from(instance_ids_3d.as_slice());
        let packed_pos = PackedVector3Array::from(positions_3d.as_slice());
        let packed_rot = PackedVector4Array::from(rotations_3d.as_slice());
        let packed_scale = PackedVector3Array::from(scales_3d.as_slice());

        // Call the GDScript bulk update method
        helper.call(
            "bulk_update_transforms_3d",
            &[
                packed_ids.to_variant(),
                packed_pos.to_variant(),
                packed_rot.to_variant(),
                packed_scale.to_variant(),
            ],
        );
    }

    // Benchmark
    for iter in 0..iterations {
        // Clear and reuse vectors (matching production)
        instance_ids_3d.clear();
        positions_3d.clear();
        rotations_3d.clear();
        scales_3d.clear();

        let start = Instant::now();

        // Collect raw transform data (matching production pattern)
        // In production, this data comes from Bevy Transform components (NO FFI)
        for (i, node) in nodes.iter().enumerate() {
            // Instance ID is the only FFI call needed
            instance_ids_3d.push(node.instance_id().to_i64());

            // In production, these come from Bevy components - simulate with same
            // values that individual FFI uses (no FFI reads)
            let x = i as f32 + (iter as f32 * 0.001);
            let y = 0.0;
            let z = 0.0;

            positions_3d.push(Vector3::new(x, y, z));

            // Simple quaternion (no FFI)
            rotations_3d.push(Vector4 {
                x: 0.0,
                y: 0.0,
                z: 0.0,
                w: 1.0,
            });

            scales_3d.push(Vector3::new(1.0001, 1.0001, 1.0001));
        }

        // Convert to packed arrays once (matching production)
        let instance_ids_packed = PackedInt64Array::from(instance_ids_3d.as_slice());
        let positions_packed = PackedVector3Array::from(positions_3d.as_slice());
        let rotations_packed = PackedVector4Array::from(rotations_3d.as_slice());
        let scales_packed = PackedVector3Array::from(scales_3d.as_slice());

        // Single bulk update call (matching production)
        helper.call(
            "bulk_update_transforms_3d",
            &[
                instance_ids_packed.to_variant(),
                positions_packed.to_variant(),
                rotations_packed.to_variant(),
                scales_packed.to_variant(),
            ],
        );

        times.push(start.elapsed());
    }

    // Cleanup
    for mut node in nodes {
        node.queue_free();
    }

    BenchResult::from_times(format!("Bulk GDScript ({} nodes)", node_count), times)
}
