//! Internal/Exploratory Benchmarks
//!
//! These benchmarks are for research and validation purposes, not for tracking
//! godot-bevy system performance over time. They help validate design decisions
//! like "should we use GDScript bulk operations or individual FFI calls?"
//!
//! To run these benchmarks, uncomment the module in lib.rs and rebuild.

#![allow(dead_code)]

use godot::builtin::StringName;
use godot::classes::{Node, Node2D, Node3D, PackedScene, ResourceLoader};
use godot::obj::NewAlloc;
use godot::prelude::*;
use godot_bevy_test::bench;

// =============================================================================
// Scene Tree Analysis Benchmarks
// =============================================================================
// These benchmarks compare different approaches for analyzing node metadata
// during scene tree initialization. The goal is to find the optimal strategy:
//
// 1. GDScript Full: GDScript analyzes everything (type, name, parent, collision)
// 2. GDScript Type-Only + FFI: GDScript only does type, Rust does rest via FFI
// 3. Pure FFI: Rust does everything (baseline for non-type metadata)
//
// Key insight: node_type detection is expensive in Rust (many try_cast calls)
// but cheap in GDScript (single `is` check). The question is whether the
// other metadata should also come from GDScript or be fetched via FFI.
//
// Results (500 nodes, release build):
// | Approach                  | Time     |
// |---------------------------|----------|
// | GDScript Full             | ~203µs   |
// | GDScript Type + FFI       | ~202µs   |
// | Pure FFI (metadata only)  | ~381µs   |
//
// Results (10,000 nodes, release build):
// | Approach                  | Time     |
// |---------------------------|----------|
// | GDScript Full             | ~4.6ms   |
// | GDScript Type + FFI       | ~4.6ms   |
// | Pure FFI (metadata only)  | ~8.7ms   |
//
// Conclusion: GDScript Full is optimal. Hybrid offers <1% improvement.
// Pure FFI is ~2x slower. The relationship scales linearly.

const ANALYSIS_NODE_COUNT: usize = 500;

/// Helper to create a mix of different node types for realistic benchmarking
fn create_mixed_nodes() -> Vec<Gd<Node>> {
    let mut nodes: Vec<Gd<Node>> = Vec::with_capacity(ANALYSIS_NODE_COUNT);

    for i in 0..ANALYSIS_NODE_COUNT {
        let node: Gd<Node> = match i % 5 {
            0 => Node3D::new_alloc().upcast(),
            1 => Node2D::new_alloc().upcast(),
            2 => {
                let mut n = Node3D::new_alloc();
                n.set_name(&format!("MeshLike_{i}"));
                n.upcast()
            }
            3 => {
                let mut n = Node2D::new_alloc();
                n.set_name(&format!("Sprite_{i}"));
                n.upcast()
            }
            _ => Node::new_alloc(),
        };
        nodes.push(node);
    }

    nodes
}

/// Helper to get or create the BenchmarkHelpers node
/// Dynamically loads the script and instantiates it if not already present
fn get_benchmark_helpers() -> Option<Gd<Node>> {
    use godot::classes::{Engine, ResourceLoader};

    let scene_tree = Engine::singleton()
        .get_main_loop()
        .and_then(|ml| ml.try_cast::<godot::classes::SceneTree>().ok())?;

    let root = scene_tree.get_root()?;

    // Try to find existing BenchmarkHelpers
    if let Some(helpers) = root
        .try_get_node_as::<Node>("BevyAppSingleton/BenchmarkHelpers")
        .or_else(|| root.try_get_node_as::<Node>("/root/BevyAppSingleton/BenchmarkHelpers"))
    {
        return Some(helpers);
    }

    // Not found - dynamically create it
    let bevy_app = root.try_get_node_as::<Node>("BevyAppSingleton")?;

    // Load the GDScript
    let script = ResourceLoader::singleton()
        .load("res://benchmark_helpers.gd")?
        .try_cast::<godot::classes::Script>()
        .ok()?;

    // Create a new Node and attach the script
    let mut helpers = Node::new_alloc();
    helpers.set_name("BenchmarkHelpers");
    helpers.set_script(&script);

    // Add to the scene tree
    bevy_app.clone().add_child(&helpers);

    Some(helpers)
}

/// Benchmark: GDScript Full - all metadata from GDScript (current approach)
///
/// Calls BenchmarkHelpers.benchmark_analyze_nodes_full() which returns:
/// instance_ids, node_types, node_names, parent_ids, collision_masks
#[bench(repeat = 3)]
fn internal_scene_analysis_gdscript_full() -> i32 {
    let nodes = create_mixed_nodes();

    let result = if let Some(mut helpers) = get_benchmark_helpers() {
        // Convert Vec<Gd<Node>> to Godot VarArray
        let mut arr = godot::builtin::VarArray::new();
        for node in nodes.iter() {
            arr.push(&node.to_variant());
        }

        // Call the GDScript benchmark method
        let result = helpers.call("benchmark_analyze_nodes_full", &[arr.to_variant()]);
        let dict = result.to::<godot::builtin::VarDictionary>();

        // Read from returned PackedArrays (this is what Rust does after GDScript call)
        let instance_ids = dict
            .get("instance_ids")
            .map(|v| v.to::<godot::builtin::PackedInt64Array>());
        let node_types = dict
            .get("node_types")
            .map(|v| v.to::<godot::builtin::PackedStringArray>());
        let node_names = dict
            .get("node_names")
            .map(|v| v.to::<godot::builtin::PackedStringArray>());
        let _parent_ids = dict
            .get("parent_ids")
            .map(|v| v.to::<godot::builtin::PackedInt64Array>());
        let _collision_masks = dict
            .get("collision_masks")
            .map(|v| v.to::<godot::builtin::PackedInt64Array>());

        // Process the returned data
        let mut total = 0i32;
        if let (Some(ids), Some(types), Some(names)) = (instance_ids, node_types, node_names) {
            for i in 0..ids.len() {
                let _id = ids.get(i);
                let _type = types.get(i);
                let _name = names.get(i);
                total += 1;
            }
        }
        total
    } else {
        // Watcher not available, return node count
        nodes.len() as i32
    };

    // Cleanup
    for node in nodes {
        node.free();
    }

    result
}

/// Benchmark: GDScript Type-Only + FFI for rest
///
/// Calls BenchmarkHelpers.benchmark_analyze_nodes_type_only() which
/// returns only instance_ids and node_types. Then Rust gets the rest via FFI.
#[bench(repeat = 3)]
fn internal_scene_analysis_gdscript_type_then_ffi() -> i32 {
    let nodes = create_mixed_nodes();

    let body_entered = StringName::from("body_entered");
    let body_exited = StringName::from("body_exited");
    let area_entered = StringName::from("area_entered");
    let area_exited = StringName::from("area_exited");

    let result = if let Some(mut helpers) = get_benchmark_helpers() {
        // Convert Vec<Gd<Node>> to Godot VarArray
        let mut arr = godot::builtin::VarArray::new();
        for node in nodes.iter() {
            arr.push(&node.to_variant());
        }

        // Call the GDScript benchmark method (type only)
        let result = helpers.call("benchmark_analyze_nodes_type_only", &[arr.to_variant()]);
        let dict = result.to::<godot::builtin::VarDictionary>();

        let instance_ids = dict
            .get("instance_ids")
            .map(|v| v.to::<godot::builtin::PackedInt64Array>());
        let node_types = dict
            .get("node_types")
            .map(|v| v.to::<godot::builtin::PackedStringArray>());

        // Now get the rest via FFI
        let mut total = 0i32;
        if let (Some(ids), Some(types)) = (instance_ids, node_types) {
            for i in 0..ids.len() {
                let _id = ids.get(i);
                let _type = types.get(i);

                // Get the rest via FFI (using original node reference)
                let node = &nodes[i];
                let _name = node.get_name();
                let _parent_id = node.get_parent().map(|p| p.instance_id());

                // Collision mask detection (4 has_signal calls)
                let mut mask = 0u8;
                if node.has_signal(&body_entered) {
                    mask |= 1;
                }
                if node.has_signal(&body_exited) {
                    mask |= 2;
                }
                if node.has_signal(&area_entered) {
                    mask |= 4;
                }
                if node.has_signal(&area_exited) {
                    mask |= 8;
                }

                total += mask as i32;
            }
        }
        total
    } else {
        nodes.len() as i32
    };

    // Cleanup
    for node in nodes {
        node.free();
    }

    result
}

/// Benchmark: GDScript Full with Groups - all metadata INCLUDING groups
///
/// Tests if adding groups to the bulk GDScript analysis is worthwhile.
#[bench(repeat = 3)]
fn internal_scene_analysis_gdscript_full_with_groups() -> i32 {
    let nodes = create_mixed_nodes();

    let result = if let Some(mut helpers) = get_benchmark_helpers() {
        let mut arr = godot::builtin::VarArray::new();
        for node in nodes.iter() {
            arr.push(&node.to_variant());
        }

        let result = helpers.call(
            "benchmark_analyze_nodes_full_with_groups",
            &[arr.to_variant()],
        );
        let dict = result.to::<godot::builtin::VarDictionary>();

        let instance_ids = dict
            .get("instance_ids")
            .map(|v| v.to::<godot::builtin::PackedInt64Array>());
        let node_types = dict
            .get("node_types")
            .map(|v| v.to::<godot::builtin::PackedStringArray>());
        let _groups = dict
            .get("groups")
            .map(|v| v.to::<godot::builtin::VarArray>());

        let mut total = 0i32;
        if let (Some(ids), Some(types)) = (instance_ids, node_types) {
            for i in 0..ids.len() {
                let _id = ids.get(i);
                let _type = types.get(i);
                total += 1;
            }
        }
        total
    } else {
        nodes.len() as i32
    };

    for node in nodes {
        node.free();
    }

    result
}

/// Benchmark: FFI get_groups only
///
/// Measures the cost of calling get_groups() via FFI for each node.
#[bench(repeat = 3)]
fn internal_scene_analysis_ffi_groups_only() -> i32 {
    let nodes = create_mixed_nodes();

    let mut total = 0i32;
    for node in nodes.iter() {
        let groups = node.get_groups();
        total += groups.len() as i32;
    }

    for node in nodes {
        node.free();
    }

    total
}

// =============================================================================
// Scene Spawning Benchmarks
// =============================================================================
// These benchmarks measure the raw performance of scene spawning operations.
// They helped validate the per-frame caching optimization in spawn_scene.
//
// Key insight: ResourceLoader.load() has significant overhead even when Godot
// caches the resource internally. Caching the loaded PackedScene in Rust
// avoids this FFI overhead when spawning multiple instances of the same scene.
//
// Results (100 scenes, release build):
// | Approach                  | Time     |
// |---------------------------|----------|
// | Unbatched (load per spawn)| ~4.3ms   |
// | Cached load               | ~200µs   |
//
// Conclusion: Per-frame caching provides ~22x improvement for batch spawns.

const SCENE_SPAWN_COUNT: usize = 100;

/// Get the scene tree root for benchmarks
fn get_scene_root() -> Gd<Node> {
    godot::classes::Engine::singleton()
        .get_main_loop()
        .and_then(|ml| ml.try_cast::<godot::classes::SceneTree>().ok())
        .and_then(|st| st.get_root())
        .expect("Scene tree root should exist")
        .upcast()
}

/// Benchmark: Unbatched - load, instantiate, add_child per scene
///
/// This simulates the OLD spawn_scene behavior (before caching):
/// 1. Load the resource each time (expensive FFI call)
/// 2. Instantiate the scene
/// 3. Add to scene tree
#[bench(repeat = 3)]
fn internal_scene_spawn_unbatched() -> i32 {
    let root = get_scene_root();
    let mut instances: Vec<Gd<Node>> = Vec::with_capacity(SCENE_SPAWN_COUNT);

    for _ in 0..SCENE_SPAWN_COUNT {
        // Load (Godot caches internally, but FFI overhead is significant)
        let packed_scene = ResourceLoader::singleton()
            .load("res://test_spawn_scene.tscn")
            .expect("Test scene should exist")
            .cast::<PackedScene>();

        let instance = packed_scene
            .instantiate()
            .expect("Scene should instantiate");

        root.clone().add_child(&instance);
        instances.push(instance);
    }

    let result = instances.len() as i32;

    for mut instance in instances {
        instance.queue_free();
    }

    result
}

// =============================================================================
// Collision Signal Connection Benchmarks
// =============================================================================
// These benchmarks compare different approaches for connecting collision signals
// to Area3D/RigidBody3D nodes. Each collision body needs up to 4 signal connections:
// - body_entered, body_exited, area_entered, area_exited
//
// Approaches tested:
// 1. Individual FFI: Rust calls node.connect() for each signal (current fallback)
// 2. Bulk GDScript: Single call to GDScript that connects all signals internally
//
// The question: Is the FFI overhead of 4 connect() calls per node significant
// enough that batching via GDScript is faster?

use godot::classes::{Area3D, Script};
use godot_bevy::watchers::collision_watcher::CollisionWatcher;

const COLLISION_BODY_COUNT: usize = 100;

// Signal name constants
const BODY_ENTERED: &str = "body_entered";
const BODY_EXITED: &str = "body_exited";
const AREA_ENTERED: &str = "area_entered";
const AREA_EXITED: &str = "area_exited";

/// Create Area3D nodes for collision benchmarking
fn create_area3d_nodes() -> Vec<Gd<Area3D>> {
    let mut nodes: Vec<Gd<Area3D>> = Vec::with_capacity(COLLISION_BODY_COUNT);

    for i in 0..COLLISION_BODY_COUNT {
        let mut area = Area3D::new_alloc();
        area.set_name(&format!("BenchArea3D_{i}"));
        nodes.push(area);
    }

    nodes
}

/// Get or create a real CollisionWatcher for benchmarks
fn get_or_create_collision_watcher() -> Gd<CollisionWatcher> {
    let root = get_scene_root();

    // Try to find existing CollisionWatcher
    if let Some(watcher) = root.try_get_node_as::<CollisionWatcher>("BenchCollisionWatcher") {
        return watcher;
    }

    // Create a real CollisionWatcher (has collision_event method)
    let mut watcher = CollisionWatcher::new_alloc();
    watcher.set_name("BenchCollisionWatcher");
    root.clone().add_child(&watcher);
    watcher
}

/// Get or create the OptimizedBulkOperations node
fn get_or_create_bulk_operations() -> Option<Gd<Node>> {
    let root = get_scene_root();

    // Try to find existing
    if let Some(ops) = root.try_get_node_as::<Node>("BenchBulkOperations") {
        return Some(ops);
    }

    // Try to load the GDScript and create the node
    let script = ResourceLoader::singleton()
        .load("res://addons/godot-bevy/optimized_bulk_operations.gd")?
        .try_cast::<Script>()
        .ok()?;

    let mut ops = Node::new_alloc();
    ops.set_name("BenchBulkOperations");
    ops.set_script(&script);
    root.clone().add_child(&ops);

    // Verify the method exists
    if ops.has_method("bulk_connect_collision_signals") {
        Some(ops)
    } else {
        ops.queue_free();
        None
    }
}

/// Benchmark: Individual FFI - connect each signal via separate FFI calls
///
/// This is the current fallback path when OptimizedBulkOperations is not available.
/// For each node: 4 connect() calls + callable creation + bind() calls
#[bench(repeat = 3)]
fn internal_collision_connect_individual_ffi() -> i32 {
    let root = get_scene_root();
    let nodes = create_area3d_nodes();
    let watcher = get_or_create_collision_watcher();

    // Add nodes to scene tree (required for signal connection)
    for node in nodes.iter() {
        root.clone().add_child(node);
    }

    let body_entered = StringName::from(BODY_ENTERED);
    let body_exited = StringName::from(BODY_EXITED);
    let area_entered = StringName::from(AREA_ENTERED);
    let area_exited = StringName::from(AREA_EXITED);

    // Connect signals individually (simulating current fallback path)
    for node in nodes.iter() {
        let mut node = node.clone();
        let node_variant = node.to_variant();

        // body_entered
        node.connect(
            &body_entered,
            &watcher
                .callable("collision_event")
                .bind(&[node_variant.clone(), "Started".to_variant()]),
        );

        // body_exited
        node.connect(
            &body_exited,
            &watcher
                .callable("collision_event")
                .bind(&[node_variant.clone(), "Ended".to_variant()]),
        );

        // area_entered
        node.connect(
            &area_entered,
            &watcher
                .callable("collision_event")
                .bind(&[node_variant.clone(), "Started".to_variant()]),
        );

        // area_exited
        node.connect(
            &area_exited,
            &watcher
                .callable("collision_event")
                .bind(&[node_variant, "Ended".to_variant()]),
        );
    }

    let result = nodes.len() as i32;

    // Cleanup
    for mut node in nodes {
        node.queue_free();
    }

    result
}

/// Benchmark: Bulk GDScript - single call to connect all signals
///
/// This uses OptimizedBulkOperations.bulk_connect_collision_signals() which
/// does all the signal connections within GDScript, avoiding per-signal FFI overhead.
#[bench(repeat = 3)]
fn internal_collision_connect_bulk_gdscript() -> i32 {
    let root = get_scene_root();
    let nodes = create_area3d_nodes();
    let watcher = get_or_create_collision_watcher();

    // Add nodes to scene tree
    for node in nodes.iter() {
        root.clone().add_child(node);
    }

    let result = if let Some(mut bulk_ops) = get_or_create_bulk_operations() {
        // Prepare packed arrays
        let instance_ids: Vec<i64> = nodes.iter().map(|n| n.instance_id().to_i64()).collect();
        // Full mask: all 4 signals (bits 0-3)
        let collision_masks: Vec<i64> = vec![0b1111i64; nodes.len()];

        let ids_packed = godot::builtin::PackedInt64Array::from(instance_ids.as_slice());
        let masks_packed = godot::builtin::PackedInt64Array::from(collision_masks.as_slice());

        // Single FFI call to connect all signals
        bulk_ops.call(
            "bulk_connect_collision_signals",
            &[
                ids_packed.to_variant(),
                masks_packed.to_variant(),
                watcher.to_variant(),
            ],
        );

        nodes.len() as i32
    } else {
        // OptimizedBulkOperations not available - skip benchmark
        godot::prelude::godot_warn!(
            "[BENCH] OptimizedBulkOperations not found - bulk benchmark skipped"
        );
        0
    };

    // Cleanup
    for mut node in nodes {
        node.queue_free();
    }

    result
}

/// Benchmark: Baseline - just create nodes and add to tree (no signal connection)
///
/// This measures the overhead of node creation and tree insertion,
/// to isolate the signal connection cost.
#[bench(repeat = 3)]
fn internal_collision_baseline_no_signals() -> i32 {
    let root = get_scene_root();
    let nodes = create_area3d_nodes();

    // Add nodes to scene tree
    for node in nodes.iter() {
        root.clone().add_child(node);
    }

    let result = nodes.len() as i32;

    // Cleanup
    for mut node in nodes {
        node.queue_free();
    }

    result
}

/// Benchmark: Cached load - load once, then instantiate per scene
///
/// This simulates the NEW spawn_scene behavior (with caching):
/// 1. Load the resource once and cache it
/// 2. Instantiate from cached PackedScene
/// 3. Add to scene tree
#[bench(repeat = 3)]
fn internal_scene_spawn_cached_load() -> i32 {
    let root = get_scene_root();

    // Load the packed scene ONCE (simulating per-frame caching)
    let packed_scene = ResourceLoader::singleton()
        .load("res://test_spawn_scene.tscn")
        .expect("Test scene should exist")
        .cast::<PackedScene>();

    let mut instances: Vec<Gd<Node>> = Vec::with_capacity(SCENE_SPAWN_COUNT);

    for _ in 0..SCENE_SPAWN_COUNT {
        let instance = packed_scene
            .instantiate()
            .expect("Scene should instantiate");
        root.clone().add_child(&instance);
        instances.push(instance);
    }

    let result = instances.len() as i32;

    for mut instance in instances {
        instance.queue_free();
    }

    result
}

/// Benchmark: Cached load with transform application
///
/// Measures the additional cost of setting transforms on spawned scenes.
#[bench(repeat = 3)]
fn internal_scene_spawn_with_transform() -> i32 {
    let root = get_scene_root();

    let packed_scene = ResourceLoader::singleton()
        .load("res://test_spawn_scene.tscn")
        .expect("Test scene should exist")
        .cast::<PackedScene>();

    let mut instances: Vec<Gd<Node>> = Vec::with_capacity(SCENE_SPAWN_COUNT);

    for i in 0..SCENE_SPAWN_COUNT {
        let instance = packed_scene
            .instantiate()
            .expect("Scene should instantiate");

        // Apply transform (the scene is Node3D)
        if let Ok(mut node3d) = instance.clone().try_cast::<Node3D>() {
            node3d.set_position(Vector3::new(i as f32 * 10.0, 0.0, 0.0));
        }

        root.clone().add_child(&instance);
        instances.push(instance);
    }

    let result = instances.len() as i32;

    for mut instance in instances {
        instance.queue_free();
    }

    result
}
