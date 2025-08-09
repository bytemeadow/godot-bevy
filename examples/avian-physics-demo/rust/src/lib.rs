#![allow(unexpected_cfgs)] // silence potential `tracy_trace` feature config warning brought in by `bevy_app` macro

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
    Added, App, AppExtStates, Assets, Commands, Component, Entity, Event, EventReader, EventWriter,
    Handle, Mesh, OnExit, Plugin, Query, Res, Resource, Result, States, Transform, Vec3, debug,
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
                PhysicsPlugins::default(),
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
            .add_systems(PhysicsUpdate, add_avian_collider)
            .add_event::<ColliderRequired>();
    }
}

#[derive(Component)]
pub struct SimpleBoxTag;

#[derive(AssetCollection, Resource, Debug)]
pub struct GameAssets {
    #[asset(path = "scenes/simple_box.tscn")]
    simple_box_scene: Handle<GodotResource>,

    #[asset(path = "scenes/floor.tscn")]
    floor_scene: Handle<GodotResource>,
}

#[derive(Event)]
struct ColliderRequired(Entity);

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

fn spawn_entities(
    mut commands: Commands,
    assets: Res<GameAssets>,
    mut events: EventWriter<ColliderRequired>,
) {
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
    let commands = commands.spawn((
        SimpleBoxTag,
        RigidBody::Dynamic,
        // NOTE: Instead of manually inserting a collider here, we
        // do it dynamically in `add_avian_collider` below so we can query
        // the loaded Godot BoxMesh for dimensions and set the
        // Collider size appropriately
        GodotScene::from_handle(assets.simple_box_scene.clone()),
        AngularVelocity(Vec3::new(1.0, 2.0, 3.0)),
        // Initialize a bevy transform with the correct starting position so avian's
        // physics simulation is aware of our position
        Transform::default().with_translation(Vec3::new(0., 10., 0.)),
    ));

    events.write(ColliderRequired(commands.id()));
}

fn add_avian_collider(
    mut commands: Commands,
    mut events: EventReader<ColliderRequired>,
    mut query: Query<&mut GodotNodeHandle, Added<RigidBody>>,
) -> Result {
    for collider_required in events.read() {
        let mut entity_commands = commands.get_entity(collider_required.0).unwrap();
        if let Ok(mut node_handle) = query.get_mut(entity_commands.id())
            && let Ok(box_mesh) = node_handle
                .get::<MeshInstance3D>()
                .get_mesh()
                .unwrap()
                .try_cast::<BoxMesh>()
        {
            let box_mesh_size = box_mesh.get_size();
            entity_commands.insert_if_new(Collider::cuboid(
                box_mesh_size.x,
                box_mesh_size.y,
                box_mesh_size.z,
            ));

            debug!(
                "Added collider matching Godot's BoxMesh size of {:?}",
                box_mesh_size
            );
        }
        // You can, of course, add support for the other godot Mesh types
    }

    Ok(())
}
