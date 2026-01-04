use godot::builtin::PackedInt64Array;
use godot::classes::Engine;
use godot::obj::NewAlloc;
use godot::prelude::*;
use godot_bevy_test::bench;

// =============================================================================
// Scene Tree Benchmarks
// =============================================================================
// These benchmarks measure the performance of our scene tree node analysis,
// comparing the cost of FFI type detection vs GDScript pre-analysis.

const SCENE_TREE_NODE_COUNT: usize = 1000;

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

/// Measures the cost of node type analysis WITHOUT OptimizedSceneTreeWatcher.
///
/// This simulates what `create_scene_tree_entity` would do if we didn't have
/// the GDScript watcher pre-analyzing node types. For each node, Rust would need to:
/// - get_name() - 1 FFI call
/// - has_signal() x4 for collision detection - 4 FFI calls
/// - get_groups() - 1 FFI call
/// - get_parent() - 1 FFI call
/// - try_from_instance_id for type detection - up to 199 FFI calls
///
/// The type detection uses `Gd::try_from_instance_id` which is the same pattern
/// as `GodotNode::try_get` in `add_comprehensive_node_type_markers`.
#[bench(repeat = 3)]
fn scene_tree_node_analysis_ffi() -> i32 {
    let mut nodes: Vec<Gd<Node>> = Vec::with_capacity(SCENE_TREE_NODE_COUNT);
    for i in 0..SCENE_TREE_NODE_COUNT {
        let mut node: Gd<Node> = match i % 4 {
            0 => Node3D::new_alloc().upcast(),
            1 => Node2D::new_alloc().upcast(),
            2 => godot::classes::Area3D::new_alloc().upcast(),
            _ => godot::classes::RigidBody3D::new_alloc().upcast(),
        };
        node.set_name(&format!("Node_{i}"));
        nodes.push(node);
    }

    let mut result = 0i32;

    for node in &nodes {
        let instance_id = node.instance_id();

        // 1. Get node name
        let name = node.get_name();
        result += name.len() as i32;

        // 2. Check collision signals (collision_mask_from_node - 4 FFI calls)
        if node.has_signal("body_entered") {
            result += 1;
        }
        if node.has_signal("body_exited") {
            result += 1;
        }
        if node.has_signal("area_entered") {
            result += 1;
        }
        if node.has_signal("area_exited") {
            result += 1;
        }

        // 3. Get groups
        let groups = node.get_groups();
        result += groups.len() as i32;

        // 4. Get parent for hierarchy
        if node.get_parent().is_some() {
            result += 1;
        }

        // 5. Type detection via try_from_instance_id
        // The real function does up to 199 try_get calls. We simulate a representative subset.
        if Gd::<Node3D>::try_from_instance_id(instance_id).is_ok() {
            result += 1;
            // Check specific 3D types (sample of 20 common types)
            if Gd::<godot::classes::MeshInstance3D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::Camera3D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::Area3D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::RigidBody3D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::StaticBody3D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::CharacterBody3D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::CollisionShape3D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::DirectionalLight3D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::OmniLight3D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::SpotLight3D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::AnimatedSprite3D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::Sprite3D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::GpuParticles3D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::CpuParticles3D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::Path3D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::PathFollow3D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::RayCast3D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::ShapeCast3D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::Skeleton3D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::BoneAttachment3D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
        } else if Gd::<Node2D>::try_from_instance_id(instance_id).is_ok() {
            result += 1;
            // Sample of 2D types
            if Gd::<godot::classes::Sprite2D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::AnimatedSprite2D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::Area2D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::RigidBody2D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::StaticBody2D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::CharacterBody2D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::CollisionShape2D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::Camera2D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::TileMap>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
            if Gd::<godot::classes::Path2D>::try_from_instance_id(instance_id).is_ok() {
                result += 1;
            }
        } else if Gd::<godot::classes::Control>::try_from_instance_id(instance_id).is_ok() {
            result += 1;
        }

        // Universal types check (always done)
        if Gd::<godot::classes::Timer>::try_from_instance_id(instance_id).is_ok() {
            result += 1;
        }
        if Gd::<godot::classes::AudioStreamPlayer>::try_from_instance_id(instance_id).is_ok() {
            result += 1;
        }
        if Gd::<godot::classes::CanvasLayer>::try_from_instance_id(instance_id).is_ok() {
            result += 1;
        }
    }

    for node in nodes {
        node.free();
    }

    result
}

/// Measures the cost of node type analysis WITH OptimizedSceneTreeWatcher.
///
/// This simulates what actually happens: GDScript receives the node in `_on_node_added`,
/// analyzes it using native `is` type checks (fast in GDScript), and sends pre-analyzed
/// data to Rust. Rust then just does string matching instead of FFI type detection.
#[bench(repeat = 3)]
fn scene_tree_node_analysis_gdscript() -> i32 {
    let mut nodes: Vec<Gd<Node>> = Vec::with_capacity(SCENE_TREE_NODE_COUNT);
    let mut instance_ids = Vec::with_capacity(SCENE_TREE_NODE_COUNT);

    for i in 0..SCENE_TREE_NODE_COUNT {
        let mut node: Gd<Node> = match i % 4 {
            0 => Node3D::new_alloc().upcast(),
            1 => Node2D::new_alloc().upcast(),
            2 => godot::classes::Area3D::new_alloc().upcast(),
            _ => godot::classes::RigidBody3D::new_alloc().upcast(),
        };
        node.set_name(&format!("Node_{i}"));
        instance_ids.push(node.instance_id().to_i64());
        nodes.push(node);
    }

    let mut bulk_ops = get_bulk_operations_node();
    let ids_packed = PackedInt64Array::from(instance_ids.as_slice());

    // Call bulk analysis (simulates what GDScript watcher does)
    let analysis = bulk_ops
        .call("bulk_analyze_nodes", &[ids_packed.to_variant()])
        .to::<godot::builtin::VarDictionary>();

    let result = if let Some(types) = analysis
        .get("node_types")
        .map(|v| v.to::<godot::builtin::PackedStringArray>())
    {
        types.len() as i32
    } else {
        -1
    };

    for node in nodes {
        node.free();
    }

    result
}
