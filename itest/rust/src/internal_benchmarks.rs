//! Internal/Exploratory Benchmarks
//!
//! These benchmarks are for research and validation purposes, not for tracking
//! godot-bevy system performance over time. They help validate design decisions
//! like "should we use GDScript bulk operations or individual FFI calls?"
//!
//! To run these benchmarks, uncomment the module in lib.rs and rebuild.

#![allow(dead_code)]

use godot::builtin::StringName;
use godot::classes::{Node, Node2D, Node3D};
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

/// Benchmark: Pure FFI metadata (no type detection)
///
/// This measures the cost of getting non-type metadata via FFI.
/// Per node: get_name() + get_parent() + instance_id() + 4x has_signal()
/// This is what the hybrid approach would need to do in addition to GDScript type.
#[bench(repeat = 3)]
fn internal_scene_analysis_ffi_metadata_only() -> i32 {
    let nodes = create_mixed_nodes();

    let body_entered = StringName::from("body_entered");
    let body_exited = StringName::from("body_exited");
    let area_entered = StringName::from("area_entered");
    let area_exited = StringName::from("area_exited");

    let mut total = 0i32;

    for node in nodes.iter() {
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

    // Cleanup
    for node in nodes {
        node.free();
    }

    total
}
