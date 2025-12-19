use avian3d::{
    collision::CollisionDiagnostics,
    dynamics::solver::SolverDiagnostics,
    prelude::{
        AngularVelocity, Collider, Gravity, PhysicsPlugins, RigidBody, SpatialQueryDiagnostics,
    },
};
use bevy::app::Startup;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::schedule::common_conditions::run_once;
use bevy::ecs::system::ResMut;
use bevy::prelude::{
    Added, App, AppExtStates, AssetEvent, Assets, Commands, Component, Entity, Handle, Mesh,
    OnExit, Plugin, Query, Res, Resource, States, Transform, Vec3, debug,
};
use bevy::{scene::ScenePlugin, state::app::StatesPlugin};
use bevy_asset_loader::{
    asset_collection::AssetCollection,
    loading_state::{LoadingState, LoadingStateAppExt, config::ConfigureLoadingState},
};
use godot::classes::{BoxMesh, MeshInstance3D};
use godot_bevy::prelude::{
    GodotAssetsPlugin, GodotBevyLogPlugin, GodotNodeHandle, GodotPackedScenePlugin, GodotResource,
    GodotScene, GodotTransformSyncPlugin, PhysicsUpdate, SceneTreeConfig, bevy_app,
    godot_prelude::{ExtensionLibrary, gdextension},
    main_thread_system,
};
use std::fmt::Debug;

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(AvianPhysicsDemo);
}

struct AvianPhysicsDemo;

#[derive(Debug, Default, Clone, Eq, PartialEq, Hash, States)]
enum GameState {
    #[default]
    LoadAssets,
    InGame,
}

impl Plugin for AvianPhysicsDemo {
    fn build(&self, app: &mut App) {
        app.add_plugins(StatesPlugin)
            .add_plugins(GodotAssetsPlugin)
            .add_plugins(GodotPackedScenePlugin)
            .add_plugins(GodotBevyLogPlugin::default())
            .add_plugins(GodotTransformSyncPlugin::default())
            .add_plugins((
                // Plugins required by Avian
                ScenePlugin,
                // Configure Avian to use godot-bevy's PhysicsUpdate schedule instead of FixedPostUpdate
                PhysicsPlugins::new(PhysicsUpdate),
            ))
            // The following 4 resource initializations are required by Avian
            .init_resource::<Assets<Mesh>>()
            .init_resource::<CollisionDiagnostics>()
            .init_resource::<SolverDiagnostics>()
            .init_resource::<SpatialQueryDiagnostics>()
            .insert_resource(Gravity::default())
            .init_state::<GameState>()
            .add_loading_state(
                LoadingState::new(GameState::LoadAssets)
                    .load_collection::<GameAssets>()
                    .continue_to_state(GameState::InGame),
            )
            .add_systems(Startup, update_scene_tree_config.run_if(run_once))
            .add_systems(OnExit(GameState::LoadAssets), spawn_entities)
            .add_systems(PhysicsUpdate, process_collider_from_godot_mesh)
            // Register AssetEvent<Mesh> since we're manually initializing Assets<Mesh> without the full asset plugin
            .add_message::<AssetEvent<Mesh>>();
    }
}

#[derive(Component)]
pub struct SimpleBoxTag;

/// Marker component that automatically adds an Avian collider based on the Godot mesh dimensions.
/// Add this component to entities with a GodotScene, and when the GodotNodeHandle is ready,
/// the system will extract the mesh dimensions and insert the appropriate Collider component.
#[derive(Component)]
pub struct ColliderFromGodotMesh;

/// System that processes entities with ColliderFromGodotMesh marker and newly added GodotNodeHandles.
/// This follows the same pattern as TypedDeferredSignalConnections in godot-bevy.
fn process_collider_from_godot_mesh(
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

#[derive(AssetCollection, Resource, Debug)]
pub struct GameAssets {
    #[asset(path = "scenes/simple_box.tscn")]
    simple_box_scene: Handle<GodotResource>,

    #[asset(path = "scenes/floor.tscn")]
    floor_scene: Handle<GodotResource>,
}

// NOTE: Would really prefer initialize these values by adding the GodotSceneTreePlugin plugin
// ourselves, but that's somewhat hardcoded at the moment, so this is a workaround until we
// fix that
#[main_thread_system]
fn update_scene_tree_config(mut config: ResMut<SceneTreeConfig>) {
    // When true, adds a parent child entity relationship in ECS
    // that mimics Godot's parent child node relationship.
    // NOTE: You should **disable** this if you want to use Avian Physics,
    // as it is incompatible, i.e., Avian Physics has its own notions
    // for what parent/child entity relatonships mean
    config.add_child_relationship = false;
}

fn spawn_entities(mut commands: Commands, assets: Res<GameAssets>) {
    //
    // Spawn a static floor
    //
    commands.spawn((
        RigidBody::Static,
        Collider::cuboid(10.0, 0.0, 10.0),
        GodotScene::from_handle(assets.floor_scene.clone()),
    ));

    //
    // Spawn a falling cuboid body with an initial angular velocity
    //
    commands.spawn((
        SimpleBoxTag,
        RigidBody::Dynamic,
        // ColliderFromGodotMesh automatically extracts the BoxMesh dimensions
        // and inserts the appropriate Collider component when the GodotNodeHandle is ready
        ColliderFromGodotMesh,
        GodotScene::from_handle(assets.simple_box_scene.clone()),
        AngularVelocity(Vec3::new(1.0, 2.0, 3.0)),
        // Initialize a bevy transform with the correct starting position so avian's
        // physics simulation is aware of our position
        Transform::default().with_translation(Vec3::new(0., 10., 0.)),
    ));
}
