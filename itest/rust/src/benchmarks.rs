//! Benchmarks for godot-bevy systems
//!
//! These benchmarks test the actual godot-bevy systems rather than raw FFI overhead.
//! They measure real-world performance of syncing transforms between Bevy and Godot.

use godot::classes::{Node2D, Node3D};
use godot::obj::NewAlloc;
use godot::prelude::*;
use godot_bevy::bevy_app::{App, Last, PreUpdate};
use godot_bevy::bevy_math::Vec3;
use godot_bevy::bevy_transform::components::Transform as BevyTransform;
use godot_bevy::interop::{GodotMainThread, GodotNodeHandle, Node2DMarker, Node3DMarker};
use godot_bevy::plugins::transforms::{
    GodotTransformSyncPlugin, GodotTransformSyncPluginExt, TransformSyncMetadata, TransformSyncMode,
};
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
