use godot::builtin::{
    PackedFloat32Array, PackedInt64Array, PackedVector2Array, PackedVector3Array,
    PackedVector4Array,
};
use godot::classes::Engine;
use godot::obj::NewAlloc;
use godot::prelude::*;
use godot_bevy_itest_macros::bench;

const BENCH_ENTITY_COUNT: usize = 5000;

/// Benchmark: Individual transform updates (3D)
/// Measures the cost of updating transforms one-by-one via individual FFI calls
#[bench(repeat = 3)]
fn transform_update_individual_3d() -> i32 {
    let mut nodes = Vec::new();

    // Create nodes
    for _ in 0..BENCH_ENTITY_COUNT {
        let mut node = Node3D::new_alloc();
        node.set_position(Vector3::new(1.0, 2.0, 3.0));
        nodes.push(node);
    }

    // Update each transform individually (N FFI calls)
    for node in &mut nodes {
        node.set_position(Vector3::new(5.0, 6.0, 7.0));
    }

    // Cleanup
    let count = nodes.len() as i32;
    for node in nodes {
        node.free();
    }

    count
}

/// Benchmark: Bulk transform updates (3D)
/// Measures the cost of updating transforms via bulk PackedArray FFI call
#[bench(repeat = 3)]
fn transform_update_bulk_3d() -> i32 {
    let mut nodes = Vec::new();
    let mut instance_ids = Vec::new();

    // Create nodes
    for _ in 0..BENCH_ENTITY_COUNT {
        let mut node = Node3D::new_alloc();
        node.set_position(Vector3::new(1.0, 2.0, 3.0));
        instance_ids.push(node.instance_id().to_i64());
        nodes.push(node);
    }

    // Prepare bulk data
    let positions = vec![Vector3::new(5.0, 6.0, 7.0); BENCH_ENTITY_COUNT];
    let rotations = vec![Vector4::new(0.0, 0.0, 0.0, 1.0); BENCH_ENTITY_COUNT];
    let scales = vec![Vector3::new(1.0, 1.0, 1.0); BENCH_ENTITY_COUNT];

    // Convert to PackedArrays (1 FFI call instead of N)
    let ids_packed = PackedInt64Array::from(instance_ids.as_slice());
    let pos_packed = PackedVector3Array::from(positions.as_slice());
    let rot_packed = PackedVector4Array::from(rotations.as_slice());
    let scale_packed = PackedVector3Array::from(scales.as_slice());

    // Call bulk update if BevyAppSingleton exists
    if let Some(scene_tree) = Engine::singleton()
        .get_main_loop()
        .and_then(|l| l.try_cast::<godot::classes::SceneTree>().ok())
    {
        if let Some(root) = scene_tree.get_root() {
            if let Some(mut bevy_app) = root.get_node_or_null("BevyAppSingleton") {
                bevy_app.call(
                    "bulk_update_transforms_3d",
                    &[
                        ids_packed.to_variant(),
                        pos_packed.to_variant(),
                        rot_packed.to_variant(),
                        scale_packed.to_variant(),
                    ],
                );
            }
        }
    }

    // Cleanup
    let count = nodes.len() as i32;
    for node in nodes {
        node.free();
    }

    count
}

/// Benchmark: Individual transform updates (2D)
/// Measures the cost of updating 2D transforms one-by-one
#[bench(repeat = 3)]
fn transform_update_individual_2d() -> i32 {
    let mut nodes = Vec::new();

    // Create nodes
    for _ in 0..BENCH_ENTITY_COUNT {
        let mut node = Node2D::new_alloc();
        node.set_position(Vector2::new(1.0, 2.0));
        nodes.push(node);
    }

    // Update each transform individually (N FFI calls)
    for node in &mut nodes {
        node.set_position(Vector2::new(5.0, 6.0));
    }

    // Cleanup
    let count = nodes.len() as i32;
    for node in nodes {
        node.free();
    }

    count
}

/// Benchmark: Bulk transform updates (2D)
/// Measures the cost of updating 2D transforms via bulk PackedArray FFI call
#[bench(repeat = 3)]
fn transform_update_bulk_2d() -> i32 {
    let mut nodes = Vec::new();
    let mut instance_ids = Vec::new();

    // Create nodes
    for _ in 0..BENCH_ENTITY_COUNT {
        let mut node = Node2D::new_alloc();
        node.set_position(Vector2::new(1.0, 2.0));
        instance_ids.push(node.instance_id().to_i64());
        nodes.push(node);
    }

    // Prepare bulk data
    let positions = vec![Vector2::new(5.0, 6.0); BENCH_ENTITY_COUNT];
    let rotations = vec![0.0f32; BENCH_ENTITY_COUNT];
    let scales = vec![Vector2::new(1.0, 1.0); BENCH_ENTITY_COUNT];

    // Convert to PackedArrays (1 FFI call instead of N)
    let ids_packed = PackedInt64Array::from(instance_ids.as_slice());
    let pos_packed = PackedVector2Array::from(positions.as_slice());
    let rot_packed = PackedFloat32Array::from(rotations.as_slice());
    let scale_packed = PackedVector2Array::from(scales.as_slice());

    // Call bulk update if BevyAppSingleton exists
    if let Some(scene_tree) = Engine::singleton()
        .get_main_loop()
        .and_then(|l| l.try_cast::<godot::classes::SceneTree>().ok())
    {
        if let Some(root) = scene_tree.get_root() {
            if let Some(mut bevy_app) = root.get_node_or_null("BevyAppSingleton") {
                bevy_app.call(
                    "bulk_update_transforms_2d",
                    &[
                        ids_packed.to_variant(),
                        pos_packed.to_variant(),
                        rot_packed.to_variant(),
                        scale_packed.to_variant(),
                    ],
                );
            }
        }
    }

    // Cleanup
    let count = nodes.len() as i32;
    for node in nodes {
        node.free();
    }

    count
}
