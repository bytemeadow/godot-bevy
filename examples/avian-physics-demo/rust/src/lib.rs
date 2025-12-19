mod api;

use api::{GodotPhysicsBox, GodotPhysicsStatic, process_collider_from_godot_mesh};
use avian3d::prelude::PhysicsPlugins;
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
    GodotTransformSyncPlugin, PhysicsUpdate, bevy_app,
    godot_prelude::{ExtensionLibrary, gdextension},
};
use std::fmt::Debug;

#[bevy_app(scene_tree_add_child_relationship = false)]
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
            // Assets<Mesh> is required by Avian but normally comes from DefaultPlugins
            // We don't use DefaultPlugins in godot-bevy, so we initialize it manually
            .init_resource::<Assets<Mesh>>()
            .init_state::<GameState>()
            .add_loading_state(
                LoadingState::new(GameState::LoadAssets)
                    .load_collection::<GameAssets>()
                    .continue_to_state(GameState::InGame),
            )
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
