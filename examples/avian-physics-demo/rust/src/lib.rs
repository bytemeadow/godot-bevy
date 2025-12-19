mod api;

use api::{GodotPhysicsBox, GodotPhysicsStatic, process_collider_from_godot_mesh};
use avian3d::{
    collision::CollisionDiagnostics,
    dynamics::solver::SolverDiagnostics,
    prelude::{Gravity, PhysicsPlugins, SpatialQueryDiagnostics},
};
use bevy::app::Startup;
use bevy::ecs::schedule::IntoScheduleConfigs;
use bevy::ecs::schedule::common_conditions::run_once;
use bevy::ecs::system::ResMut;
use bevy::prelude::{
    App, AppExtStates, AssetEvent, Assets, Commands, Handle, Mesh, OnExit, Plugin, Res, Resource,
    States, Vec3,
};
use bevy::{scene::ScenePlugin, state::app::StatesPlugin};
use bevy_asset_loader::{
    asset_collection::AssetCollection,
    loading_state::{LoadingState, LoadingStateAppExt, config::ConfigureLoadingState},
};
use godot_bevy::prelude::{
    GodotAssetsPlugin, GodotBevyLogPlugin, GodotPackedScenePlugin, GodotResource,
    GodotTransformSyncPlugin, PhysicsUpdate, SceneTreeConfig, bevy_app,
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
    // Static physics object with a collision shape (cylinder floor)
    commands.spawn(GodotPhysicsStatic::cylinder(
        assets.floor_scene.clone(),
        4.0, // radius
        0.1, // height
    ));

    // Dynamic physics object with a collision shape and initial angular velocity
    commands.spawn(GodotPhysicsBox::dynamic_with_spin(
        assets.simple_box_scene.clone(),
        Vec3::new(0.0, 4.0, 0.0),
        Vec3::new(2.5, 3.5, 1.5),
    ));
}
