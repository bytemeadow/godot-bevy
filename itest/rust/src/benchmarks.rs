//! Benchmarks for godot-bevy systems
//!
//! These benchmarks test the actual godot-bevy systems rather than raw FFI overhead.
//! They measure real-world performance of syncing transforms between Bevy and Godot.

use bevy::prelude::{Event, On, ResMut, Resource};
use crossbeam_channel as mpsc;
use godot::builtin::StringName;
use godot::classes::{Area3D, Engine, InputEventKey, InputMap, Node, Node2D, Node3D, SceneTree};
use godot::global::Key;
use godot::obj::{NewAlloc, Singleton};
use godot::prelude::*;
use godot_bevy::bevy_app::{App, First, Last, PostUpdate, PreUpdate, Update};
use godot_bevy::bevy_math::Vec3;
use godot_bevy::bevy_transform::components::Transform as BevyTransform;
use godot_bevy::interop::{GodotMainThread, GodotNodeHandle, Node2DMarker, Node3DMarker};
use godot_bevy::plugins::collisions::{
    CollisionMessageReader, CollisionMessageType, CollisionState, GodotCollisionsPlugin,
    RawCollisionMessage,
};
use godot_bevy::plugins::core::{PrePhysicsUpdate, SceneTreeComponentRegistry};
use godot_bevy::plugins::input::{GodotInputEventPlugin, InputEventReader, InputEventType};
use godot_bevy::plugins::packed_scene::{GodotPackedScenePlugin, GodotScene};
use godot_bevy::plugins::scene_tree::{
    GodotSceneTreePlugin, NodeEntityIndex, SceneTreeMessage, SceneTreeMessageReader,
    SceneTreeMessageType,
};
use godot_bevy::plugins::signals::{GodotSignals, GodotSignalsPlugin};
use godot_bevy::plugins::transforms::{
    GodotTransformSyncPlugin, GodotTransformSyncPluginExt, TransformSyncMetadata, TransformSyncMode,
};
use godot_bevy::watchers::collision_watcher::CollisionWatcher;
use godot_bevy_test::bench;

// =============================================================================
// Transform Sync Benchmarks
// =============================================================================
// These benchmarks measure the performance of our transform synchronization
// systems - the actual code that syncs transforms between Bevy ECS and Godot.

const NODE_COUNT: usize = 1000;

/// Creates a Bevy App configured with transform sync plugin and test entities.
/// Returns the app and the Godot nodes (to keep them alive).
fn setup_3d_benchmark_app() -> (App, Vec<Gd<Node3D>>) {
    let mut app = App::new();

    // Initialize schedules manually (avoid plugin duplication issues)
    app.init_schedule(PreUpdate);
    app.init_schedule(Last);

    // Add transform sync plugin
    app.add_plugins(GodotTransformSyncPlugin::default().with_sync_mode(TransformSyncMode::TwoWay));

    // Insert the GodotMainThread resource (required for GodotAccess)
    app.insert_non_send_resource(GodotMainThread);

    let mut nodes: Vec<Gd<Node3D>> = Vec::with_capacity(NODE_COUNT);

    for i in 0..NODE_COUNT {
        let mut node = Node3D::new_alloc();
        node.set_name(&format!("BenchNode3D_{i}"));
        node.set_position(Vector3::new(i as f32, 0.0, 0.0));

        let handle = GodotNodeHandle::new(node.clone());
        let transform = BevyTransform::from_xyz(i as f32, 0.0, 0.0);

        app.world_mut().spawn((
            handle,
            transform,
            TransformSyncMetadata::default(),
            Node3DMarker,
        ));

        nodes.push(node);
    }

    (app, nodes)
}

/// Creates a Bevy App configured for 2D transform sync benchmarking.
fn setup_2d_benchmark_app() -> (App, Vec<Gd<Node2D>>) {
    let mut app = App::new();

    // Initialize schedules manually (avoid plugin duplication issues)
    app.init_schedule(PreUpdate);
    app.init_schedule(Last);

    // Add transform sync plugin
    app.add_plugins(GodotTransformSyncPlugin::default().with_sync_mode(TransformSyncMode::TwoWay));
    app.insert_non_send_resource(GodotMainThread);

    let mut nodes: Vec<Gd<Node2D>> = Vec::with_capacity(NODE_COUNT);

    for i in 0..NODE_COUNT {
        let mut node = Node2D::new_alloc();
        node.set_name(&format!("BenchNode2D_{i}"));
        node.set_position(Vector2::new(i as f32, 0.0));

        let handle = GodotNodeHandle::new(node.clone());
        let transform = BevyTransform::from_xyz(i as f32, 0.0, 0.0);

        app.world_mut().spawn((
            handle,
            transform,
            TransformSyncMetadata::default(),
            Node2DMarker,
        ));

        nodes.push(node);
    }

    (app, nodes)
}

// =============================================================================
// 3D Transform Sync Benchmarks
// =============================================================================

/// Benchmark: Write transforms from Bevy to Godot (3D) using actual systems
///
/// This runs the real post_update_godot_transforms system that syncs
/// Bevy transform changes to Godot nodes.
#[bench(repeat = 3)]
fn transform_sync_bevy_to_godot_3d() -> i32 {
    let (mut app, nodes) = setup_3d_benchmark_app();

    // Modify all Bevy transforms to trigger change detection
    let mut query = app.world_mut().query::<&mut BevyTransform>();
    for (i, mut transform) in query.iter_mut(app.world_mut()).enumerate() {
        transform.translation = Vec3::new(i as f32 * 2.0, i as f32, 0.0);
    }

    // Run the Last schedule which contains the sync system
    app.world_mut().run_schedule(Last);

    let result = nodes.len() as i32;

    // Cleanup
    for node in nodes {
        node.free();
    }

    result
}

/// Benchmark: Read transforms from Godot into Bevy (3D) using actual systems
///
/// This runs the real pre_update_godot_transforms system that syncs
/// Godot node transforms into Bevy.
#[bench(repeat = 3)]
fn transform_sync_godot_to_bevy_3d() -> i32 {
    let (mut app, nodes) = setup_3d_benchmark_app();

    // Modify Godot transforms to simulate physics/animation changes
    for (i, node) in nodes.iter().enumerate() {
        let mut node = node.clone();
        node.set_position(Vector3::new(i as f32 * 2.0, i as f32, 0.0));
    }

    // Run the PreUpdate schedule which contains the sync system
    app.world_mut().run_schedule(PreUpdate);

    let result = nodes.len() as i32;

    for node in nodes {
        node.free();
    }

    result
}

// =============================================================================
// 2D Transform Sync Benchmarks
// =============================================================================

/// Benchmark: Write transforms from Bevy to Godot (2D) using actual systems
#[bench(repeat = 3)]
fn transform_sync_bevy_to_godot_2d() -> i32 {
    let (mut app, nodes) = setup_2d_benchmark_app();

    let mut query = app.world_mut().query::<&mut BevyTransform>();
    for (i, mut transform) in query.iter_mut(app.world_mut()).enumerate() {
        transform.translation = Vec3::new(i as f32 * 2.0, i as f32, 0.0);
    }

    app.world_mut().run_schedule(Last);

    let result = nodes.len() as i32;

    for node in nodes {
        node.free();
    }

    result
}

/// Benchmark: Read transforms from Godot into Bevy (2D) using actual systems
#[bench(repeat = 3)]
fn transform_sync_godot_to_bevy_2d() -> i32 {
    let (mut app, nodes) = setup_2d_benchmark_app();

    for (i, node) in nodes.iter().enumerate() {
        let mut node = node.clone();
        node.set_position(Vector2::new(i as f32 * 2.0, i as f32));
    }

    app.world_mut().run_schedule(PreUpdate);

    let result = nodes.len() as i32;

    for node in nodes {
        node.free();
    }

    result
}

// =============================================================================
// Full Round-Trip Benchmark
// =============================================================================

/// Benchmark: Complete transform sync cycle (both directions) for 3D
///
/// This represents a complete frame's worth of transform synchronization:
/// 1. PreUpdate: Read Godot transforms into Bevy
/// 2. Game logic modifies some transforms
/// 3. PostUpdate: Write Bevy transforms back to Godot
#[bench(repeat = 3)]
fn transform_sync_roundtrip_3d() -> i32 {
    let (mut app, nodes) = setup_3d_benchmark_app();

    // Simulate Godot physics moving nodes
    for (i, node) in nodes.iter().enumerate() {
        let mut node = node.clone();
        node.set_position(Vector3::new(i as f32, (i as f32).sin(), 0.0));
    }

    // Phase 1: Sync Godot -> Bevy (PreUpdate)
    app.world_mut().run_schedule(PreUpdate);

    // Phase 2: Simulate game logic modifying transforms
    let mut query = app.world_mut().query::<&mut BevyTransform>();
    for (i, mut transform) in query.iter_mut(app.world_mut()).enumerate() {
        if i % 2 == 0 {
            transform.translation.y += 10.0;
        }
    }

    // Phase 3: Sync Bevy -> Godot (Last)
    app.world_mut().run_schedule(Last);

    let result = nodes.len() as i32;

    for node in nodes {
        node.free();
    }

    result
}

/// Benchmark: Complete transform sync cycle (both directions) for 2D
#[bench(repeat = 3)]
fn transform_sync_roundtrip_2d() -> i32 {
    let (mut app, nodes) = setup_2d_benchmark_app();

    // Simulate Godot physics moving nodes
    for (i, node) in nodes.iter().enumerate() {
        let mut node = node.clone();
        node.set_position(Vector2::new(i as f32, (i as f32).sin()));
    }

    // Phase 1: Sync Godot -> Bevy (PreUpdate)
    app.world_mut().run_schedule(PreUpdate);

    // Phase 2: Simulate game logic modifying transforms
    let mut query = app.world_mut().query::<&mut BevyTransform>();
    for (i, mut transform) in query.iter_mut(app.world_mut()).enumerate() {
        if i % 2 == 0 {
            transform.translation.y += 10.0;
        }
    }

    // Phase 3: Sync Bevy -> Godot (Last)
    app.world_mut().run_schedule(Last);

    let result = nodes.len() as i32;

    for node in nodes {
        node.free();
    }

    result
}

// =============================================================================
// Scene Tree Message Processing Benchmarks
// =============================================================================
// These benchmarks measure the performance of processing scene tree messages
// (NodeAdded events) which is critical for entity creation and component setup.

const SCENE_TREE_NODE_COUNT: usize = 500;
const SCENE_TREE_SPARSE_RENAME_FRAMES: usize = 80;

/// Get the Godot scene tree
fn get_scene_tree() -> Gd<SceneTree> {
    Engine::singleton()
        .get_main_loop()
        .expect("Main loop should exist")
        .cast::<SceneTree>()
}

/// Creates a mix of Godot nodes for scene tree benchmarking.
/// Returns nodes attached to the scene tree (required for scene tree plugin).
fn create_scene_tree_nodes() -> Vec<Gd<Node>> {
    let scene_tree = get_scene_tree();
    let root = scene_tree.get_root().expect("Root should exist");

    let mut nodes: Vec<Gd<Node>> = Vec::with_capacity(SCENE_TREE_NODE_COUNT);

    for i in 0..SCENE_TREE_NODE_COUNT {
        let node: Gd<Node> = match i % 3 {
            0 => {
                let mut n = Node3D::new_alloc();
                n.set_name(&format!("BenchNode3D_{i}"));
                n.upcast()
            }
            1 => {
                let mut n = Node2D::new_alloc();
                n.set_name(&format!("BenchNode2D_{i}"));
                n.upcast()
            }
            _ => {
                let mut n = Node::new_alloc();
                n.set_name(&format!("BenchNode_{i}"));
                n
            }
        };

        // Add to scene tree (required for the plugin to work)
        root.clone().add_child(&node);
        nodes.push(node);
    }

    nodes
}

/// Creates SceneTreeMessage events for a batch of nodes.
/// Simulates the optimized path with pre-analyzed type information.
fn create_node_added_messages(nodes: &[Gd<Node>]) -> Vec<SceneTreeMessage> {
    nodes
        .iter()
        .enumerate()
        .map(|(i, node)| {
            let node_type = match i % 3 {
                0 => "Node3D",
                1 => "Node2D",
                _ => "Node",
            };

            SceneTreeMessage {
                node_id: GodotNodeHandle::from(node.instance_id()),
                message_type: SceneTreeMessageType::NodeAdded,
                node_type: Some(node_type.to_string()),
                node_name: Some(node.get_name().to_string()),
                parent_id: node.get_parent().map(|p| p.instance_id()),
                collision_mask: Some(0), // No collision signals
                groups: Some(vec![]),    // No groups
            }
        })
        .collect()
}

/// Setup a Bevy App with the scene tree plugin for benchmarking.
/// Returns the app and an mpsc sender for injecting messages.
fn setup_scene_tree_benchmark_app() -> (App, mpsc::Sender<SceneTreeMessage>) {
    let mut app = App::new();

    // Initialize required schedules
    app.init_schedule(First);
    app.init_schedule(PreUpdate);

    // Insert required resources (normally added by GodotBaseCorePlugin)
    app.insert_non_send_resource(GodotMainThread);
    app.init_resource::<SceneTreeComponentRegistry>();

    // Create a channel for injecting messages BEFORE adding the plugin
    // (plugin will try to init its own receiver, but we'll override it)
    let (sender, receiver) = mpsc::unbounded::<SceneTreeMessage>();

    // Add the scene tree plugin
    app.add_plugins(GodotSceneTreePlugin::default());

    // Replace the message reader with our test channel
    app.insert_resource(SceneTreeMessageReader::new(receiver));

    (app, sender)
}

/// Benchmark: Scene tree message systems when no messages are pending (idle path)
///
/// This captures per-frame overhead when the scene tree is stable and no
/// node-added/removed messages are flowing from Godot.
#[bench(repeat = 3)]
fn scene_tree_idle_no_messages() -> i32 {
    let (mut app, _sender) = setup_scene_tree_benchmark_app();

    const IDLE_FRAMES: usize = 200;
    for _ in 0..IDLE_FRAMES {
        app.world_mut().run_schedule(First);
    }

    IDLE_FRAMES as i32
}

/// Benchmark: Process NodeAdded messages with pre-analyzed types (optimized path)
///
/// This measures the performance of the `read_scene_tree_messages` system
/// processing a batch of NodeAdded events. This is the hot path when nodes
/// are added to the Godot scene tree at runtime.
///
/// The benchmark uses pre-analyzed type information (simulating the optimized
/// GDScript watcher path) which avoids expensive FFI type detection.
#[bench(repeat = 3)]
fn scene_tree_process_node_added_optimized() -> i32 {
    let (mut app, sender) = setup_scene_tree_benchmark_app();
    let nodes = create_scene_tree_nodes();

    // Create messages with pre-analyzed types (optimized path)
    let messages = create_node_added_messages(&nodes);

    // Send all messages through the channel
    for msg in messages {
        sender.send(msg).expect("Send should succeed");
    }

    // Run First schedule twice:
    // 1st run: write_scene_tree_messages writes to buffer B, read_scene_tree_messages
    //          reads from buffer A (empty), then message_update_system flips buffers
    // 2nd run: read_scene_tree_messages now reads from buffer A (has messages)
    app.world_mut().run_schedule(First);
    app.world_mut().run_schedule(First);

    // Verify entities were created
    let node_index = app.world().resource::<NodeEntityIndex>();
    let result = node_index.len() as i32;

    // Cleanup - remove nodes from scene tree
    for mut node in nodes {
        node.queue_free();
    }

    result
}

/// Benchmark: Process NodeAdded messages without pre-analyzed types (fallback path)
///
/// This measures the performance when type information is NOT pre-analyzed,
/// forcing the system to detect node types via FFI calls. This is the slower
/// fallback path used when the optimized GDScript watcher is not available.
#[bench(repeat = 3)]
fn scene_tree_process_node_added_fallback() -> i32 {
    let (mut app, sender) = setup_scene_tree_benchmark_app();
    let nodes = create_scene_tree_nodes();

    // Create messages WITHOUT pre-analyzed types (fallback path)
    let messages: Vec<SceneTreeMessage> = nodes
        .iter()
        .map(|node| SceneTreeMessage {
            node_id: GodotNodeHandle::from(node.instance_id()),
            message_type: SceneTreeMessageType::NodeAdded,
            node_type: None, // Force FFI-based type detection
            node_name: None, // Force FFI-based name lookup
            parent_id: None, // Force FFI-based parent lookup
            collision_mask: None,
            groups: None,
        })
        .collect();

    // Send all messages through the channel
    for msg in messages {
        sender.send(msg).expect("Send should succeed");
    }

    // Run First schedule twice (message_update_system flips buffers after first run)
    app.world_mut().run_schedule(First);
    app.world_mut().run_schedule(First);

    // Verify entities were created
    let node_index = app.world().resource::<NodeEntityIndex>();
    let result = node_index.len() as i32;

    // Cleanup
    for mut node in nodes {
        node.queue_free();
    }

    result
}

/// Benchmark: Process sparse NodeRenamed messages on an already-populated index.
///
/// This captures per-frame overhead for tiny scene-tree updates after startup,
/// when many entities are already tracked.
#[bench(repeat = 3)]
fn scene_tree_process_node_renamed_sparse_updates() -> i32 {
    let (mut app, sender) = setup_scene_tree_benchmark_app();
    let nodes = create_scene_tree_nodes();

    for msg in create_node_added_messages(&nodes) {
        sender.send(msg).expect("Send should succeed");
    }
    app.world_mut().run_schedule(First);
    app.world_mut().run_schedule(First);

    let target_node = nodes
        .first()
        .expect("At least one scene tree node should exist");
    let target_handle = GodotNodeHandle::from(target_node.instance_id());

    for i in 0..SCENE_TREE_SPARSE_RENAME_FRAMES {
        sender
            .send(SceneTreeMessage {
                node_id: target_handle,
                message_type: SceneTreeMessageType::NodeRenamed,
                node_type: None,
                node_name: Some(format!("SparseRename_{i}")),
                parent_id: None,
                collision_mask: None,
                groups: None,
            })
            .expect("Send should succeed");

        // message_update_system double-buffering requires two First runs.
        app.world_mut().run_schedule(First);
        app.world_mut().run_schedule(First);
    }

    let node_index_len = app.world().resource::<NodeEntityIndex>().len();

    for mut node in nodes {
        node.queue_free();
    }

    assert_eq!(node_index_len, SCENE_TREE_NODE_COUNT);
    SCENE_TREE_SPARSE_RENAME_FRAMES as i32
}

// =============================================================================
// Scene Tree Collision Body Benchmarks
// =============================================================================
// These benchmarks measure the performance of processing scene tree messages
// for collision bodies (Area3D nodes), which require connecting collision signals.

const COLLISION_BODY_COUNT: usize = 100;

// Collision mask constants (must match the ones in scene_tree/plugin.rs)
const COLLISION_MASK_BODY_ENTERED: u8 = 1 << 0;
const COLLISION_MASK_BODY_EXITED: u8 = 1 << 1;
const COLLISION_MASK_AREA_ENTERED: u8 = 1 << 2;
const COLLISION_MASK_AREA_EXITED: u8 = 1 << 3;
const COLLISION_PROCESS_NODE_COUNT: usize = 200;
const COLLISION_PROCESS_CYCLES: usize = 200;

/// Creates Area3D nodes for collision body benchmarking.
/// These nodes have collision signals that need to be connected.
fn create_collision_body_nodes() -> Vec<Gd<Node>> {
    let scene_tree = get_scene_tree();
    let root = scene_tree.get_root().expect("Root should exist");

    let mut nodes: Vec<Gd<Node>> = Vec::with_capacity(COLLISION_BODY_COUNT);

    for i in 0..COLLISION_BODY_COUNT {
        let mut area = Area3D::new_alloc();
        area.set_name(&format!("BenchArea3D_{i}"));
        root.clone().add_child(&area);
        nodes.push(area.upcast());
    }

    nodes
}

/// Ensures a CollisionWatcher node exists under BevyAppSingleton, mirroring production layout.
fn ensure_collision_watcher() -> Gd<Node> {
    let scene_tree = get_scene_tree();
    let root = scene_tree.get_root().expect("Root should exist");

    if let Some(watcher) = root.try_get_node_as::<Node>("BevyAppSingleton/CollisionWatcher") {
        return watcher;
    }

    let bevy_app = root
        .try_get_node_as::<Node>("BevyAppSingleton")
        .expect("BevyAppSingleton autoload should exist in benchmark scene");

    let mut watcher = CollisionWatcher::new_alloc();
    watcher.set_name("CollisionWatcher");
    bevy_app.clone().add_child(&watcher);
    watcher.upcast()
}

/// Creates SceneTreeMessage events for collision body nodes with pre-analyzed collision masks.
fn create_collision_body_messages(nodes: &[Gd<Node>]) -> Vec<SceneTreeMessage> {
    let full_mask = COLLISION_MASK_BODY_ENTERED
        | COLLISION_MASK_BODY_EXITED
        | COLLISION_MASK_AREA_ENTERED
        | COLLISION_MASK_AREA_EXITED;

    nodes
        .iter()
        .map(|node| SceneTreeMessage {
            node_id: GodotNodeHandle::from(node.instance_id()),
            message_type: SceneTreeMessageType::NodeAdded,
            node_type: Some("Area3D".to_string()),
            node_name: Some(node.get_name().to_string()),
            parent_id: node.get_parent().map(|p| p.instance_id()),
            collision_mask: Some(full_mask),
            groups: Some(vec![]),
        })
        .collect()
}

/// Benchmark: Process collision body NodeAdded messages (optimized path)
///
/// This measures the performance of processing Area3D nodes with collision
/// signal connection using the optimized GDScript bulk operations path.
/// Each Area3D has 4 collision signals that get connected.
#[bench(repeat = 3)]
fn scene_tree_process_collision_bodies_optimized() -> i32 {
    // Create watcher FIRST, before app setup
    let watcher = ensure_collision_watcher();

    let (mut app, sender) = setup_scene_tree_benchmark_app();
    let nodes = create_collision_body_nodes();

    // Verify watcher is in tree
    let scene_tree = get_scene_tree();
    let root = scene_tree.get_root().expect("Root should exist");
    let watcher_found = root
        .try_get_node_as::<Node>("BevyAppSingleton/CollisionWatcher")
        .is_some();
    if !watcher_found {
        godot::prelude::godot_error!("[BENCH] CollisionWatcher not found in tree!");
    }

    // Create messages with pre-analyzed collision masks (optimized path)
    let messages = create_collision_body_messages(&nodes);

    for msg in messages {
        sender.send(msg).expect("Send should succeed");
    }

    // Run First schedule twice (message_update_system flips buffers after first run)
    app.world_mut().run_schedule(First);
    app.world_mut().run_schedule(First);

    let node_index = app.world().resource::<NodeEntityIndex>();
    let result = node_index.len() as i32;

    for mut node in nodes {
        node.queue_free();
    }

    // Clean up watcher
    watcher.clone().free();

    result
}

/// Benchmark: Process collision body NodeAdded messages (fallback path)
///
/// This measures the performance when collision masks are NOT pre-analyzed,
/// forcing the system to detect collision signals via FFI calls and connect
/// them individually.
#[bench(repeat = 3)]
fn scene_tree_process_collision_bodies_fallback() -> i32 {
    let (mut app, sender) = setup_scene_tree_benchmark_app();
    let nodes = create_collision_body_nodes();

    // Ensure CollisionWatcher exists so signals get connected
    let watcher = ensure_collision_watcher();

    // Create messages WITHOUT pre-analyzed collision masks (fallback path)
    let messages: Vec<SceneTreeMessage> = nodes
        .iter()
        .map(|node| SceneTreeMessage {
            node_id: GodotNodeHandle::from(node.instance_id()),
            message_type: SceneTreeMessageType::NodeAdded,
            node_type: None,
            node_name: None,
            parent_id: None,
            collision_mask: None, // Force FFI-based collision mask detection
            groups: None,
        })
        .collect();

    for msg in messages {
        sender.send(msg).expect("Send should succeed");
    }

    // Run First schedule twice (message_update_system flips buffers after first run)
    app.world_mut().run_schedule(First);
    app.world_mut().run_schedule(First);

    let node_index = app.world().resource::<NodeEntityIndex>();
    let result = node_index.len() as i32;

    for mut node in nodes {
        node.queue_free();
    }

    // Clean up watcher
    watcher.clone().free();

    result
}

type CollisionBenchSender = mpsc::Sender<RawCollisionMessage>;

/// Setup app for collision message processing benchmarks.
/// Returns (app, scene_tree_sender, collision_sender).
fn setup_collision_processing_benchmark_app()
-> (App, mpsc::Sender<SceneTreeMessage>, CollisionBenchSender) {
    let mut app = App::new();
    app.init_schedule(First);
    app.init_schedule(PreUpdate);
    app.init_schedule(PrePhysicsUpdate);

    app.insert_non_send_resource(GodotMainThread);
    app.init_resource::<SceneTreeComponentRegistry>();

    let (scene_sender, scene_receiver) = mpsc::unbounded::<SceneTreeMessage>();
    let (collision_sender, collision_receiver) = mpsc::unbounded::<RawCollisionMessage>();

    app.add_plugins((GodotSceneTreePlugin::default(), GodotCollisionsPlugin));
    app.insert_resource(SceneTreeMessageReader::new(scene_receiver));
    app.insert_resource(CollisionMessageReader::new(collision_receiver));

    (app, scene_sender, collision_sender)
}

/// Creates plain Node instances for collision-processing benchmarks.
fn create_collision_processing_nodes() -> Vec<Gd<Node>> {
    let scene_tree = get_scene_tree();
    let root = scene_tree.get_root().expect("Root should exist");

    let mut nodes = Vec::with_capacity(COLLISION_PROCESS_NODE_COUNT + 1);
    for i in 0..=COLLISION_PROCESS_NODE_COUNT {
        let mut node = Node::new_alloc();
        node.set_name(&format!("CollisionProcessNode_{i}"));
        root.clone().add_child(&node);
        nodes.push(node);
    }
    nodes
}

fn create_collision_processing_node_added_messages(nodes: &[Gd<Node>]) -> Vec<SceneTreeMessage> {
    nodes
        .iter()
        .map(|node| SceneTreeMessage {
            node_id: GodotNodeHandle::from(node.instance_id()),
            message_type: SceneTreeMessageType::NodeAdded,
            node_type: Some("Node".to_string()),
            node_name: Some(node.get_name().to_string()),
            parent_id: node.get_parent().map(|parent| parent.instance_id()),
            collision_mask: Some(0),
            groups: Some(vec![]),
        })
        .collect()
}

/// Benchmark: process a burst of collision start/end messages.
///
/// This focuses on `process_godot_collisions` and `CollisionState` update costs
/// by sending repeated start/end cycles for one origin colliding with many targets.
#[bench(repeat = 3)]
fn collisions_process_start_end_burst() -> i32 {
    let (mut app, scene_sender, collision_sender) = setup_collision_processing_benchmark_app();
    let nodes = create_collision_processing_nodes();

    for msg in create_collision_processing_node_added_messages(&nodes) {
        scene_sender.send(msg).expect("Send should succeed");
    }

    // Populate NodeEntityIndex via scene-tree processing
    app.world_mut().run_schedule(First);
    app.world_mut().run_schedule(First);

    let origin = GodotNodeHandle::from(nodes[0].instance_id());
    let targets: Vec<GodotNodeHandle> = nodes[1..]
        .iter()
        .map(|node| GodotNodeHandle::from(node.instance_id()))
        .collect();

    for _ in 0..COLLISION_PROCESS_CYCLES {
        for &target in &targets {
            collision_sender
                .send(RawCollisionMessage {
                    event_type: CollisionMessageType::Started,
                    origin,
                    target,
                })
                .expect("Send should succeed");
        }
        for &target in &targets {
            collision_sender
                .send(RawCollisionMessage {
                    event_type: CollisionMessageType::Ended,
                    origin,
                    target,
                })
                .expect("Send should succeed");
        }
    }

    app.world_mut().run_schedule(PrePhysicsUpdate);

    let active = app.world().resource::<CollisionState>().len();

    for mut node in nodes {
        node.queue_free();
    }

    // Expect zero active collisions after balanced Started/Ended bursts.
    assert_eq!(active, 0);
    (COLLISION_PROCESS_NODE_COUNT * COLLISION_PROCESS_CYCLES * 2) as i32
}

// =============================================================================
// Input Action-Checking Benchmarks
// =============================================================================

const INPUT_EVENT_COUNT: usize = 100;
const INPUT_ACTION_COUNT: usize = 50;

type InputActionBenchSender = mpsc::Sender<(InputEventType, Gd<godot::classes::InputEvent>)>;

/// Setup for input actionaction benchmark. Returns (app, sender, action_names for cleanup).
fn setup_input_action_benchmark_app() -> (App, InputActionBenchSender, Vec<StringName>) {
    let (sender, receiver) = mpsc::unbounded();

    let mut app = App::new();

    app.init_schedule(First);
    app.insert_non_send_resource(GodotMainThread);

    app.add_plugins(GodotInputEventPlugin);
    app.insert_non_send_resource(InputEventReader(receiver));

    let mut input_map = InputMap::singleton();
    let mut action_names = Vec::with_capacity(INPUT_ACTION_COUNT);
    for i in 0..INPUT_ACTION_COUNT {
        let name = StringName::from(&format!("bench_act_{i}"));
        input_map.add_action(&name);
        action_names.push(name);
    }

    (app, sender, action_names)
}

/// Runs write_input_messages (First) with INPUT_EVENT_COUNT Normal events and INPUT_ACTION_COUNT InputMap actions.
#[bench(repeat = 3)]
fn input_action_checking_many_events_many_actions() -> i32 {
    let (mut app, sender, action_names) = setup_input_action_benchmark_app();

    let events: Vec<_> = (0..INPUT_EVENT_COUNT)
        .map(|_| {
            let mut key_ev = InputEventKey::new_gd();
            key_ev.set_keycode(Key::SPACE);
            key_ev.set_pressed(true);
            key_ev.upcast::<godot::classes::InputEvent>()
        })
        .collect();

    for event in events {
        let _ = sender.send((InputEventType::Normal, event));
    }
    drop(sender);

    app.world_mut().run_schedule(First);

    let mut input_map = InputMap::singleton();
    for name in &action_names {
        input_map.erase_action(name);
    }

    (INPUT_EVENT_COUNT * INPUT_ACTION_COUNT) as i32
}

// =============================================================================
// Packed Scene Spawning Benchmarks
// =============================================================================
// These benchmarks measure the performance of spawning Godot scenes from Bevy,
// using the real spawn_scene() system in PostUpdate.

const PACKED_SCENE_COUNT: usize = 100;

/// Creates a Bevy App with packed scene spawning infrastructure.
/// IMPORTANT: Only run PostUpdate — never PreStartup, which would panic
/// without a SceneTreeWatcher in the scene tree.
fn setup_packed_scene_benchmark_app() -> App {
    let mut app = App::new();

    app.init_schedule(First);
    app.init_schedule(PostUpdate);

    app.insert_non_send_resource(GodotMainThread);
    app.init_resource::<SceneTreeComponentRegistry>();

    // Scene tree plugin provides SceneTreeRef (needed by spawn_scene)
    app.add_plugins(GodotSceneTreePlugin::default());
    // Packed scene plugin adds spawn_scene system to PostUpdate
    app.add_plugins(GodotPackedScenePlugin);
    // Asset plugin provides AssetServer + Assets<GodotResource> (needed by spawn_scene)
    app.add_plugins(godot_bevy::plugins::assets::GodotAssetsPlugin);

    for _ in 0..PACKED_SCENE_COUNT {
        app.world_mut()
            .spawn(GodotScene::from_path("res://test_spawn_scene.tscn"));
    }

    app
}

/// Cleanup spawned scene instances by querying GodotNodeHandle entities.
fn cleanup_packed_scene_nodes(app: &mut App) {
    let mut query = app.world_mut().query::<&GodotNodeHandle>();
    let handles: Vec<GodotNodeHandle> = query.iter(app.world()).copied().collect();

    for handle in handles {
        if let Ok(node) = Gd::<Node>::try_from_instance_id(handle.instance_id()) {
            node.free();
        }
    }
}

/// Benchmark: Batch spawn 100 instances of the same packed scene
///
/// This runs the real spawn_scene() system which:
/// 1. Loads via ResourceLoader (1st instance)
/// 2. Hits per-frame HashMap cache (remaining 99)
/// 3. Instantiates each PackedScene
/// 4. Adds each instance to the scene tree
/// 5. Inserts GodotNodeHandle on each entity
#[bench(repeat = 3)]
fn packed_scene_batch_spawn() -> i32 {
    let mut app = setup_packed_scene_benchmark_app();

    // Run PostUpdate which contains the spawn_scene system
    app.world_mut().run_schedule(PostUpdate);

    let result = PACKED_SCENE_COUNT as i32;

    cleanup_packed_scene_nodes(&mut app);

    result
}

// =============================================================================
// Signal System Benchmarks
// =============================================================================
// These benchmarks measure the performance of the Godot signal → Bevy observer
// pipeline: signal connection setup, per-frame dispatch throughput, and idle overhead.

const SIGNAL_NODE_COUNT: usize = 200;

#[derive(Event, Clone, Debug)]
struct BenchSignalEvent {
    #[allow(dead_code)]
    value: i32,
}

#[derive(Resource, Default)]
struct SignalCounter(i32);

/// System that connects signals on all entities with GodotNodeHandle.
/// Added to Update schedule, run once during setup, then schedule is not run again.
fn connect_bench_signals(
    query: bevy::prelude::Query<(bevy::prelude::Entity, &GodotNodeHandle)>,
    signals: GodotSignals<BenchSignalEvent>,
) {
    for (entity, handle) in query.iter() {
        signals.connect(*handle, "bench_signal", Some(entity), |_, _, _| {
            Some(BenchSignalEvent { value: 1 })
        });
    }
}

/// Creates a signal benchmark app with N nodes that have custom "bench_signal" user signals.
/// Returns the app and the nodes (to keep them alive and for emitting signals).
fn setup_signal_benchmark_app() -> (App, Vec<Gd<Node>>) {
    let mut app = App::new();

    app.init_schedule(First);
    app.init_schedule(Update);
    app.init_schedule(Last);

    app.add_plugins(GodotSignalsPlugin::<BenchSignalEvent>::default());
    app.insert_non_send_resource(GodotMainThread);
    app.init_resource::<SignalCounter>();
    app.add_observer(
        |_: On<BenchSignalEvent>, mut counter: ResMut<SignalCounter>| {
            counter.0 += 1;
        },
    );

    let mut nodes = Vec::with_capacity(SIGNAL_NODE_COUNT);

    for i in 0..SIGNAL_NODE_COUNT {
        let mut node = Node::new_alloc();
        node.set_name(&format!("BenchSignalNode_{i}"));
        node.add_user_signal("bench_signal");

        let handle = GodotNodeHandle::new(node.clone());
        app.world_mut().spawn(handle);
        nodes.push(node);
    }

    (app, nodes)
}

/// Sets up signal connections by running the connect system once.
/// After this call, all nodes have their "bench_signal" connected to the observer pipeline.
fn connect_all_bench_signals(app: &mut App) {
    app.add_systems(Update, connect_bench_signals);
    app.world_mut().run_schedule(Update); // Queues connections to PendingSignalConnections
    app.world_mut().run_schedule(Last); // Processes pending → actually connects via FFI
}

/// Benchmark: Signal dispatch throughput (full pipeline)
///
/// Measures the complete signal pipeline per frame:
/// 1. Godot signal emission (emit_signal on each node)
/// 2. Callable closure runs → Variant cloning → crossbeam channel push
/// 3. drain_and_trigger_signals exclusive system drains channel
/// 4. world.trigger() fires observer for each event
#[bench(repeat = 3)]
fn signal_dispatch_throughput() -> i32 {
    let (mut app, mut nodes) = setup_signal_benchmark_app();
    connect_all_bench_signals(&mut app);

    let signal_name = StringName::from("bench_signal");

    // Emit signals on all nodes (synchronously runs callable closures,
    // pushing events to the crossbeam channel)
    for node in &mut nodes {
        node.emit_signal(&signal_name, &[]);
    }

    // Drain channel and trigger observers
    app.world_mut().run_schedule(First);

    // Verify all signals were dispatched
    let counter = app.world().resource::<SignalCounter>();
    assert_eq!(counter.0, SIGNAL_NODE_COUNT as i32);

    let result = SIGNAL_NODE_COUNT as i32;

    for node in nodes {
        node.free();
    }

    result
}

/// Benchmark: Signal connection setup cost (FFI)
///
/// Measures the cost of process_pending_signal_connections in the Last schedule:
/// 200x Callable::from_fn() creation + 200x node.connect() FFI calls.
#[bench(repeat = 3)]
fn signal_connection_setup() -> i32 {
    let (mut app, nodes) = setup_signal_benchmark_app();

    // Queue connections (runs connect_bench_signals in Update)
    app.add_systems(Update, connect_bench_signals);
    app.world_mut().run_schedule(Update);

    // Measure: process pending connections (FFI: Callable creation + node.connect)
    app.world_mut().run_schedule(Last);

    let result = SIGNAL_NODE_COUNT as i32;

    for node in nodes {
        node.free();
    }

    result
}
