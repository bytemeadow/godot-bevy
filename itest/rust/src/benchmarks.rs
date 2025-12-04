use godot::builtin::{
    PackedFloat32Array, PackedInt64Array, PackedVector2Array, PackedVector3Array,
    PackedVector4Array,
};
use godot::classes::{Engine, InputEventKey, InputMap};
use godot::global::Key;
use godot::obj::{NewAlloc, NewGd};
use godot::prelude::*;
use godot_bevy_itest_macros::bench;

const BENCH_ENTITY_COUNT: usize = 5000;
const BENCH_ACTION_EVENT_COUNT: usize = 100;

fn get_bevy_app_singleton() -> Gd<Node> {
    let scene_tree = Engine::singleton()
        .get_main_loop()
        .and_then(|l| l.try_cast::<godot::classes::SceneTree>().ok())
        .expect("Failed to get SceneTree");
    let root = scene_tree.get_root().expect("Failed to get root node");
    root.get_node_or_null("BevyAppSingleton")
        .expect("BevyAppSingleton not found - ensure it is configured as an autoload")
}

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

    let mut bevy_app = get_bevy_app_singleton();
    bevy_app.call(
        "bulk_update_transforms_3d",
        &[
            ids_packed.to_variant(),
            pos_packed.to_variant(),
            rot_packed.to_variant(),
            scale_packed.to_variant(),
        ],
    );

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

    let mut bevy_app = get_bevy_app_singleton();
    bevy_app.call(
        "bulk_update_transforms_2d",
        &[
            ids_packed.to_variant(),
            pos_packed.to_variant(),
            rot_packed.to_variant(),
            scale_packed.to_variant(),
        ],
    );

    // Cleanup
    let count = nodes.len() as i32;
    for node in nodes {
        node.free();
    }

    count
}

/// Benchmark: Individual transform reads (3D)
/// Measures the cost of reading transforms one-by-one via individual FFI calls
#[bench(repeat = 3)]
fn transform_read_individual_3d() -> i32 {
    let mut nodes = Vec::new();

    // Create nodes with initial transforms
    for i in 0..BENCH_ENTITY_COUNT {
        let mut node = Node3D::new_alloc();
        node.set_position(Vector3::new(i as f32, i as f32 * 2.0, i as f32 * 3.0));
        nodes.push(node);
    }

    // Read each transform individually (N FFI calls)
    let mut sum = Vector3::ZERO;
    for node in &nodes {
        sum += node.get_position();
    }

    // Cleanup
    let count = nodes.len() as i32;
    for node in nodes {
        node.free();
    }

    // Use sum to prevent optimization
    (count as f32 + sum.x) as i32
}

/// Benchmark: Bulk transform reads (3D)
/// Measures the cost of reading transforms via bulk PackedArray FFI call
#[bench(repeat = 3)]
fn transform_read_bulk_3d() -> i32 {
    let mut nodes = Vec::new();
    let mut instance_ids = Vec::new();

    // Create nodes with initial transforms
    for i in 0..BENCH_ENTITY_COUNT {
        let mut node = Node3D::new_alloc();
        node.set_position(Vector3::new(i as f32, i as f32 * 2.0, i as f32 * 3.0));
        instance_ids.push(node.instance_id().to_i64());
        nodes.push(node);
    }

    let ids_packed = PackedInt64Array::from(instance_ids.as_slice());

    let mut bevy_app = get_bevy_app_singleton();
    let result = bevy_app
        .call("bulk_get_transforms_3d", &[ids_packed.to_variant()])
        .to::<godot::builtin::Dictionary>();

    let mut sum = Vector3::ZERO;
    if let Some(positions) = result
        .get("positions")
        .map(|v| v.to::<PackedVector3Array>())
    {
        for i in 0..positions.len() {
            if let Some(pos) = positions.get(i) {
                sum += pos;
            }
        }
    }

    // Cleanup
    let count = nodes.len() as i32;
    for node in nodes {
        node.free();
    }

    // Use sum to prevent optimization
    (count as f32 + sum.x) as i32
}

/// Benchmark: Individual transform reads (2D)
/// Measures the cost of reading 2D transforms one-by-one
#[bench(repeat = 3)]
fn transform_read_individual_2d() -> i32 {
    let mut nodes = Vec::new();

    // Create nodes with initial transforms
    for i in 0..BENCH_ENTITY_COUNT {
        let mut node = Node2D::new_alloc();
        node.set_position(Vector2::new(i as f32, i as f32 * 2.0));
        nodes.push(node);
    }

    // Read each transform individually (N FFI calls)
    let mut sum = Vector2::ZERO;
    for node in &nodes {
        sum += node.get_position();
    }

    // Cleanup
    let count = nodes.len() as i32;
    for node in nodes {
        node.free();
    }

    // Use sum to prevent optimization
    (count as f32 + sum.x) as i32
}

/// Benchmark: Bulk transform reads (2D)
/// Measures the cost of reading 2D transforms via bulk PackedArray FFI call
#[bench(repeat = 3)]
fn transform_read_bulk_2d() -> i32 {
    let mut nodes = Vec::new();
    let mut instance_ids = Vec::new();

    // Create nodes with initial transforms
    for i in 0..BENCH_ENTITY_COUNT {
        let mut node = Node2D::new_alloc();
        node.set_position(Vector2::new(i as f32, i as f32 * 2.0));
        instance_ids.push(node.instance_id().to_i64());
        nodes.push(node);
    }

    let ids_packed = PackedInt64Array::from(instance_ids.as_slice());

    let mut bevy_app = get_bevy_app_singleton();
    let result = bevy_app
        .call("bulk_get_transforms_2d", &[ids_packed.to_variant()])
        .to::<godot::builtin::Dictionary>();

    let mut sum = Vector2::ZERO;
    if let Some(positions) = result
        .get("positions")
        .map(|v| v.to::<PackedVector2Array>())
    {
        for i in 0..positions.len() {
            if let Some(pos) = positions.get(i) {
                sum += pos;
            }
        }
    }

    // Cleanup
    let count = nodes.len() as i32;
    for node in nodes {
        node.free();
    }

    // Use sum to prevent optimization
    (count as f32 + sum.x) as i32
}

/// Benchmark: Individual action checking
/// Measures the cost of checking input events against all actions one-by-one
#[bench(repeat = 3)]
fn action_check_individual() -> i32 {
    // Create a key event that we'll check against actions
    let mut key_event = InputEventKey::new_gd();
    key_event.set_keycode(Key::SPACE);
    key_event.set_pressed(true);

    let mut input_map = InputMap::singleton();
    let actions = input_map.get_actions();
    let action_count = actions.len();

    let mut match_count = 0;

    // Simulate checking multiple input events
    for _ in 0..BENCH_ACTION_EVENT_COUNT {
        // Check each action individually (N FFI calls per event)
        for action_name in actions.iter_shared() {
            if key_event.is_action(&action_name) {
                let _pressed = key_event.is_action_pressed(&action_name);
                let _strength = key_event.get_action_strength(&action_name);
                match_count += 1;
            }
        }
    }

    // Return action count to prevent optimization
    (action_count + match_count) as i32
}

/// Benchmark: Bulk action checking
/// Measures the cost of checking input events against all actions via single FFI call
#[bench(repeat = 3)]
fn action_check_bulk() -> i32 {
    // Create a key event that we'll check against actions
    let mut key_event = InputEventKey::new_gd();
    key_event.set_keycode(Key::SPACE);
    key_event.set_pressed(true);

    let mut bevy_app = get_bevy_app_singleton();
    let mut match_count = 0;

    // Simulate checking multiple input events
    for _ in 0..BENCH_ACTION_EVENT_COUNT {
        let result = bevy_app
            .call("bulk_check_actions", &[key_event.to_variant()])
            .to::<godot::builtin::Dictionary>();

        if let Some(actions) = result
            .get("actions")
            .map(|v| v.to::<godot::builtin::PackedStringArray>())
        {
            match_count += actions.len();
        }
    }

    match_count as i32
}
