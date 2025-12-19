//! Reusable Avian Physics + Godot integration patterns.
//!
//! This module provides helper bundles and systems for integrating Avian Physics
//! with godot-bevy. You can copy this into your own project as a starting point.

#![allow(dead_code)]

use avian3d::prelude::{AngularVelocity, Collider, RigidBody};
use bevy::asset::Handle;
use bevy::prelude::{Added, Bundle, Commands, Component, Entity, Query, Transform, Vec3, debug};
use godot::classes::{BoxMesh, MeshInstance3D};
use godot_bevy::prelude::{GodotNodeHandle, GodotResource, GodotScene};

/// Marker component that automatically adds an Avian collider based on the Godot mesh dimensions.
/// Add this component to entities with a GodotScene, and when the GodotNodeHandle is ready,
/// the system will extract the mesh dimensions and insert the appropriate Collider component.
///
/// Currently supports:
/// - BoxMesh → Collider::cuboid
///
/// Future extensions could support SphereMesh, CapsuleMesh, etc.
#[derive(Component)]
pub struct ColliderFromGodotMesh;

/// System that processes entities with ColliderFromGodotMesh marker and newly added GodotNodeHandles.
/// Add this system to your app with:
/// ```ignore
/// app.add_systems(PhysicsUpdate, process_collider_from_godot_mesh);
/// ```
pub fn process_collider_from_godot_mesh(
    mut commands: Commands,
    query: Query<(Entity, &GodotNodeHandle, &ColliderFromGodotMesh), Added<GodotNodeHandle>>,
) {
    for (entity, node_handle, _marker) in query.iter() {
        // Try to get BoxMesh and extract dimensions
        let mut node_handle_mut = node_handle.clone();
        if let Some(mesh_instance) = node_handle_mut.try_get::<MeshInstance3D>() {
            if let Some(mesh) = mesh_instance.get_mesh() {
                if let Ok(box_mesh) = mesh.try_cast::<BoxMesh>() {
                    let size = box_mesh.get_size();

                    commands
                        .entity(entity)
                        .insert(Collider::cuboid(size.x, size.y, size.z))
                        .remove::<ColliderFromGodotMesh>();

                    debug!(
                        "ColliderFromGodotMesh: Added collider matching BoxMesh size of {:?}",
                        size
                    );
                }
            }
        }
        // You can extend this with support for other Godot mesh types (SphereMesh, CapsuleMesh, etc.)
    }
}

/// A dynamic physics box that uses a Godot scene for visualization.
/// The collider is automatically sized to match the BoxMesh in the scene.
#[derive(Bundle)]
pub struct GodotPhysicsBox {
    pub scene: GodotScene,
    pub body: RigidBody,
    pub collider_marker: ColliderFromGodotMesh,
    pub transform: Transform,
}

impl GodotPhysicsBox {
    /// Create a new dynamic physics box at the given position
    pub fn dynamic(scene_handle: Handle<GodotResource>, position: Vec3) -> Self {
        Self {
            scene: GodotScene::from_handle(scene_handle),
            body: RigidBody::Dynamic,
            collider_marker: ColliderFromGodotMesh,
            transform: Transform::from_translation(position),
        }
    }

    /// Create a new dynamic physics box with angular velocity
    pub fn dynamic_with_spin(
        scene_handle: Handle<GodotResource>,
        position: Vec3,
        angular_velocity: Vec3,
    ) -> (Self, AngularVelocity) {
        (
            Self::dynamic(scene_handle, position),
            AngularVelocity(angular_velocity),
        )
    }

    /// Create a new static physics box at the given position
    pub fn stationary(scene_handle: Handle<GodotResource>, position: Vec3) -> Self {
        Self {
            scene: GodotScene::from_handle(scene_handle),
            body: RigidBody::Static,
            collider_marker: ColliderFromGodotMesh,
            transform: Transform::from_translation(position),
        }
    }
}

/// A static physics object with a manual collider that uses a Godot scene for visualization.
/// Use this when you want to manually specify the collider dimensions instead of deriving
/// them from the Godot mesh.
#[derive(Bundle)]
pub struct GodotPhysicsStatic {
    pub scene: GodotScene,
    pub body: RigidBody,
    pub collider: Collider,
}

impl GodotPhysicsStatic {
    /// Create a new static object with a cuboid collider
    pub fn cuboid(
        scene_handle: Handle<GodotResource>,
        width: f32,
        height: f32,
        depth: f32,
    ) -> Self {
        Self {
            scene: GodotScene::from_handle(scene_handle),
            body: RigidBody::Static,
            collider: Collider::cuboid(width, height, depth),
        }
    }

    /// Create a new static object with any collider
    pub fn new(scene_handle: Handle<GodotResource>, collider: Collider) -> Self {
        Self {
            scene: GodotScene::from_handle(scene_handle),
            body: RigidBody::Static,
            collider,
        }
    }
}
