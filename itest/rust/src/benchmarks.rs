use godot::builtin::{
    Basis, PackedFloat32Array, PackedInt64Array, PackedVector2Array, PackedVector3Array,
    PackedVector4Array, Quaternion, Transform2D, Transform3D,
};
use godot::classes::{Engine, InputEventKey, InputMap};
use godot::global::Key;
use godot::obj::{NewAlloc, NewGd};
use godot::prelude::*;
use godot_bevy_itest_macros::bench;

/// Helper to create a Transform3D from position, rotation (quaternion xyzw), and scale
fn make_transform_3d(pos: Vector3, rot: Vector4, scale: Vector3) -> Transform3D {
    let quat = Quaternion::new(rot.x, rot.y, rot.z, rot.w);
    let rotation_basis = Basis::from_quaternion(quat);
    let basis = Basis::from_cols(
        rotation_basis.col_a() * scale.x,
        rotation_basis.col_b() * scale.y,
        rotation_basis.col_c() * scale.z,
    );
    Transform3D { basis, origin: pos }
}

/// Helper to create a Transform2D from position, rotation (radians), and scale
fn make_transform_2d(pos: Vector2, rotation: f32, scale: Vector2) -> Transform2D {
    let cos_rot = rotation.cos();
    let sin_rot = rotation.sin();
    let a = Vector2::new(cos_rot * scale.x, sin_rot * scale.x);
    let b = Vector2::new(-sin_rot * scale.y, cos_rot * scale.y);
    Transform2D { a, b, origin: pos }
}

const BENCH_ENTITY_COUNT: usize = 20000;
const BENCH_ACTION_EVENT_COUNT: usize = 100;

fn get_bulk_operations_node() -> Gd<Node> {
    let scene_tree = Engine::singleton()
        .get_main_loop()
        .and_then(|l| l.try_cast::<godot::classes::SceneTree>().ok())
        .expect("Failed to get SceneTree");
    let root = scene_tree.get_root().expect("Failed to get root node");

    // Try to find existing node
    if let Some(node) = root
        .get_node_or_null("BevyAppSingleton/OptimizedBulkOperations")
        .or_else(|| root.get_node_or_null("/root/BevyAppSingleton/OptimizedBulkOperations"))
    {
        return node;
    }

    // Node doesn't exist (e.g., release build where it's not auto-registered)
    // Create it dynamically for benchmarks
    let mut bevy_app = root
        .get_node_or_null("BevyAppSingleton")
        .or_else(|| root.get_node_or_null("/root/BevyAppSingleton"))
        .expect("BevyAppSingleton not found - ensure it is configured as an autoload");

    let path = "res://addons/godot-bevy/optimized_bulk_operations.gd";
    let mut resource_loader = godot::classes::ResourceLoader::singleton();

    if let Some(resource) = resource_loader.load(path)
        && let Ok(mut script) = resource.try_cast::<godot::classes::GDScript>()
        && let Ok(instance) = script.try_instantiate(&[])
        && let Ok(mut node) = instance.try_to::<Gd<Node>>()
    {
        node.set_name("OptimizedBulkOperations");
        bevy_app.add_child(&node);
        node
    } else {
        panic!("Failed to create OptimizedBulkOperations node from GDScript");
    }
}

/// Measures the cost of updating 3D transforms one-by-one via individual FFI calls
#[bench(repeat = 3)]
fn transform_update_individual_3d() -> i32 {
    let mut nodes = Vec::with_capacity(BENCH_ENTITY_COUNT);
    for _ in 0..BENCH_ENTITY_COUNT {
        let mut node = Node3D::new_alloc();
        node.set_transform(make_transform_3d(
            Vector3::new(1.0, 2.0, 3.0),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
            Vector3::new(1.0, 1.0, 1.0),
        ));
        nodes.push(node);
    }

    for node in &mut nodes {
        node.set_transform(make_transform_3d(
            Vector3::new(5.0, 6.0, 7.0),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
            Vector3::new(1.0, 1.0, 1.0),
        ));
    }

    let count = nodes.len() as i32;
    for node in nodes {
        node.free();
    }

    count
}

/// Measures the cost of updating 3D transforms via bulk PackedArray FFI call
#[bench(repeat = 3)]
fn transform_update_bulk_3d() -> i32 {
    let mut nodes = Vec::with_capacity(BENCH_ENTITY_COUNT);
    let mut instance_ids = Vec::with_capacity(BENCH_ENTITY_COUNT);
    for _ in 0..BENCH_ENTITY_COUNT {
        let mut node = Node3D::new_alloc();
        node.set_position(Vector3::new(1.0, 2.0, 3.0));
        instance_ids.push(node.instance_id().to_i64());
        nodes.push(node);
    }

    let ids_packed = PackedInt64Array::from(instance_ids.as_slice());
    let mut bulk_ops = get_bulk_operations_node();

    let positions = vec![Vector3::new(5.0, 6.0, 7.0); BENCH_ENTITY_COUNT];
    let rotations = vec![Vector4::new(0.0, 0.0, 0.0, 1.0); BENCH_ENTITY_COUNT];
    let scales = vec![Vector3::new(1.0, 1.0, 1.0); BENCH_ENTITY_COUNT];

    let pos_packed = PackedVector3Array::from(positions.as_slice());
    let rot_packed = PackedVector4Array::from(rotations.as_slice());
    let scale_packed = PackedVector3Array::from(scales.as_slice());

    bulk_ops.call(
        "bulk_update_transforms_3d",
        &[
            ids_packed.to_variant(),
            pos_packed.to_variant(),
            rot_packed.to_variant(),
            scale_packed.to_variant(),
        ],
    );

    let count = nodes.len() as i32;
    for node in nodes {
        node.free();
    }

    count
}

/// Measures the cost of updating 2D transforms one-by-one via individual FFI calls
#[bench(repeat = 3)]
fn transform_update_individual_2d() -> i32 {
    let mut nodes = Vec::with_capacity(BENCH_ENTITY_COUNT);
    for _ in 0..BENCH_ENTITY_COUNT {
        let mut node = Node2D::new_alloc();
        node.set_transform(make_transform_2d(
            Vector2::new(1.0, 2.0),
            0.0,
            Vector2::new(1.0, 1.0),
        ));
        nodes.push(node);
    }

    for node in &mut nodes {
        node.set_transform(make_transform_2d(
            Vector2::new(5.0, 6.0),
            0.0,
            Vector2::new(1.0, 1.0),
        ));
    }

    let count = nodes.len() as i32;
    for node in nodes {
        node.free();
    }

    count
}

/// Measures the cost of updating 2D transforms via bulk PackedArray FFI call
#[bench(repeat = 3)]
fn transform_update_bulk_2d() -> i32 {
    let mut nodes = Vec::with_capacity(BENCH_ENTITY_COUNT);
    let mut instance_ids = Vec::with_capacity(BENCH_ENTITY_COUNT);
    for _ in 0..BENCH_ENTITY_COUNT {
        let mut node = Node2D::new_alloc();
        node.set_position(Vector2::new(1.0, 2.0));
        instance_ids.push(node.instance_id().to_i64());
        nodes.push(node);
    }

    let ids_packed = PackedInt64Array::from(instance_ids.as_slice());
    let mut bulk_ops = get_bulk_operations_node();

    let positions = vec![Vector2::new(5.0, 6.0); BENCH_ENTITY_COUNT];
    let rotations = vec![0.0f32; BENCH_ENTITY_COUNT];
    let scales = vec![Vector2::new(1.0, 1.0); BENCH_ENTITY_COUNT];

    let pos_packed = PackedVector2Array::from(positions.as_slice());
    let rot_packed = PackedFloat32Array::from(rotations.as_slice());
    let scale_packed = PackedVector2Array::from(scales.as_slice());

    bulk_ops.call(
        "bulk_update_transforms_2d",
        &[
            ids_packed.to_variant(),
            pos_packed.to_variant(),
            rot_packed.to_variant(),
            scale_packed.to_variant(),
        ],
    );

    let count = nodes.len() as i32;
    for node in nodes {
        node.free();
    }

    count
}

/// Measures the cost of reading 3D transforms one-by-one via individual FFI calls
#[bench(repeat = 3)]
fn transform_read_individual_3d() -> i32 {
    let mut nodes = Vec::with_capacity(BENCH_ENTITY_COUNT);
    for i in 0..BENCH_ENTITY_COUNT {
        let mut node = Node3D::new_alloc();
        node.set_transform(make_transform_3d(
            Vector3::new(i as f32, i as f32 * 2.0, i as f32 * 3.0),
            Vector4::new(0.0, 0.0, 0.0, 1.0),
            Vector3::new(1.0, 1.0, 1.0),
        ));
        nodes.push(node);
    }

    let mut sum = Vector3::ZERO;
    for node in &nodes {
        let transform = node.get_transform();
        // Use the transform data to prevent optimization
        sum += transform.origin;
    }

    let count = nodes.len() as i32;
    for node in nodes {
        node.free();
    }

    (count as f32 + sum.x) as i32
}

/// Measures the cost of reading 3D transforms via bulk PackedArray FFI call
#[bench(repeat = 3)]
fn transform_read_bulk_3d() -> i32 {
    let mut nodes = Vec::with_capacity(BENCH_ENTITY_COUNT);
    let mut instance_ids = Vec::with_capacity(BENCH_ENTITY_COUNT);
    for i in 0..BENCH_ENTITY_COUNT {
        let mut node = Node3D::new_alloc();
        node.set_position(Vector3::new(i as f32, i as f32 * 2.0, i as f32 * 3.0));
        instance_ids.push(node.instance_id().to_i64());
        nodes.push(node);
    }

    let ids_packed = PackedInt64Array::from(instance_ids.as_slice());
    let mut bulk_ops = get_bulk_operations_node();

    let mut sum = Vector3::ZERO;
    let result = bulk_ops
        .call("bulk_get_transforms_3d", &[ids_packed.to_variant()])
        .to::<godot::builtin::Dictionary>();

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

    let count = nodes.len() as i32;
    for node in nodes {
        node.free();
    }

    (count as f32 + sum.x) as i32
}

/// Measures the cost of reading 2D transforms one-by-one via individual FFI calls
#[bench(repeat = 3)]
fn transform_read_individual_2d() -> i32 {
    let mut nodes = Vec::with_capacity(BENCH_ENTITY_COUNT);
    for i in 0..BENCH_ENTITY_COUNT {
        let mut node = Node2D::new_alloc();
        node.set_transform(make_transform_2d(
            Vector2::new(i as f32, i as f32 * 2.0),
            0.0,
            Vector2::new(1.0, 1.0),
        ));
        nodes.push(node);
    }

    let mut sum = Vector2::ZERO;
    for node in &nodes {
        let transform = node.get_transform();
        // Use the transform data to prevent optimization
        sum += transform.origin;
    }

    let count = nodes.len() as i32;
    for node in nodes {
        node.free();
    }

    (count as f32 + sum.x) as i32
}

/// Measures the cost of reading 2D transforms via bulk PackedArray FFI call
#[bench(repeat = 3)]
fn transform_read_bulk_2d() -> i32 {
    let mut nodes = Vec::with_capacity(BENCH_ENTITY_COUNT);
    let mut instance_ids = Vec::with_capacity(BENCH_ENTITY_COUNT);
    for i in 0..BENCH_ENTITY_COUNT {
        let mut node = Node2D::new_alloc();
        node.set_position(Vector2::new(i as f32, i as f32 * 2.0));
        instance_ids.push(node.instance_id().to_i64());
        nodes.push(node);
    }

    let ids_packed = PackedInt64Array::from(instance_ids.as_slice());
    let mut bulk_ops = get_bulk_operations_node();

    let mut sum = Vector2::ZERO;
    let result = bulk_ops
        .call("bulk_get_transforms_2d", &[ids_packed.to_variant()])
        .to::<godot::builtin::Dictionary>();

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

    let count = nodes.len() as i32;
    for node in nodes {
        node.free();
    }

    (count as f32 + sum.x) as i32
}

/// Measures the cost of checking input events against actions one-by-one
#[bench(repeat = 3)]
fn action_check_individual() -> i32 {
    let mut key_event = InputEventKey::new_gd();
    key_event.set_keycode(Key::SPACE);
    key_event.set_pressed(true);

    let mut input_map = InputMap::singleton();
    let actions = input_map.get_actions();
    let action_count = actions.len();

    let mut match_count = 0;
    for _ in 0..BENCH_ACTION_EVENT_COUNT {
        for action_name in actions.iter_shared() {
            if key_event.is_action(&action_name) {
                let _pressed = key_event.is_action_pressed(&action_name);
                let _strength = key_event.get_action_strength(&action_name);
                match_count += 1;
            }
        }
    }

    (action_count + match_count) as i32
}

/// Measures the cost of checking input events against actions via single FFI call
#[bench(repeat = 3)]
fn action_check_bulk() -> i32 {
    let mut key_event = InputEventKey::new_gd();
    key_event.set_keycode(Key::SPACE);
    key_event.set_pressed(true);

    let mut bulk_ops = get_bulk_operations_node();
    let mut match_count = 0;

    for _ in 0..BENCH_ACTION_EVENT_COUNT {
        let result = bulk_ops
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
