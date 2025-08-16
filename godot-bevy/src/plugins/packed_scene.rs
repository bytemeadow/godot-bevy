use super::scene_tree::SceneTreeRef;
use crate::plugins::assets::GodotResource;
use crate::plugins::transforms::IntoGodotTransform2D;
use crate::prelude::main_thread_system;
use crate::{interop::GodotNodeHandle, plugins::transforms::IntoGodotTransform};
use bevy::{
    app::{App, Plugin, PostUpdate},
    asset::{Assets, Handle},
    ecs::{
        component::Component,
        entity::Entity,
        query::Without,
        system::{Commands, Query, ResMut},
    },
    log::tracing,
    transform::components::Transform,
};
use godot::{
    builtin::GString,
    classes::{Node, Node2D, Node3D, PackedScene, ResourceLoader},
};
use std::str::FromStr;
use tracing::error;

#[derive(Default)]
pub struct GodotPackedScenePlugin;
impl Plugin for GodotPackedScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, spawn_scene);
    }
}

// silence warning about the following docs referring to private `spawn_scene`
#[allow(rustdoc::private_intra_doc_links)]
/// A to-be-instanced-and-spawned Godot scene.
///
/// [`GodotScene`]s that are spawned/inserted into the bevy world will be instanced from the provided
/// handle/path and the instance will be added as an [`GodotNodeHandle`] in the next PostUpdateFlush set.
/// (see [`spawn_scene`])
#[derive(Debug, Component)]
pub struct GodotScene {
    resource: GodotSceneResource,
    parent: Option<GodotNodeHandle>,
}

#[derive(Debug)]
enum GodotSceneResource {
    Handle(Handle<GodotResource>),
    Path(String),
}

impl GodotScene {
    /// Instantiate the godot scene from a Bevy `Handle<GodotResource>` and add it to the
    /// scene tree root. This is the preferred method when using Bevy's asset system.
    pub fn from_handle(handle: Handle<GodotResource>) -> Self {
        Self {
            resource: GodotSceneResource::Handle(handle),
            parent: None,
        }
    }

    /// Instantiate the godot scene from the given path and add it to the scene tree root.
    ///
    /// Note that this will call [`ResourceLoader`].load() - which is a blocking load.
    /// If you want async loading, you should load your resources through Bevy's AssetServer
    /// and use from_handle().
    pub fn from_path(path: &str) -> Self {
        Self {
            resource: GodotSceneResource::Path(path.to_string()),
            parent: None,
        }
    }

    /// Set the parent node for this scene when spawned.
    pub fn with_parent(mut self, parent: GodotNodeHandle) -> Self {
        self.parent = Some(parent);
        self
    }
}

#[main_thread_system]
fn spawn_scene(
    mut commands: Commands,
    mut new_scenes: Query<(&mut GodotScene, Entity, Option<&Transform>), Without<GodotNodeHandle>>,
    mut scene_tree: SceneTreeRef,
    mut assets: ResMut<Assets<GodotResource>>,
) {
    for (mut scene, ent, transform) in new_scenes.iter_mut() {
        let packed_scene = match &scene.resource {
            GodotSceneResource::Handle(handle) => assets
                .get_mut(handle)
                .expect("packed scene to exist in assets")
                .get()
                .clone(),
            GodotSceneResource::Path(path) => ResourceLoader::singleton()
                .load(&GString::from_str(path).expect("path to be a valid GString"))
                .expect("packed scene to load"),
        };

        let packed_scene_cast = packed_scene.clone().try_cast::<PackedScene>();
        if packed_scene_cast.is_err() {
            error!("Resource is not a PackedScene: {:?}", packed_scene);
            continue;
        }

        let packed_scene = packed_scene_cast.unwrap();

        let instance = match packed_scene.instantiate() {
            Some(instance) => instance,
            None => {
                error!("Failed to instantiate PackedScene");
                continue;
            }
        };

        if let Some(transform) = transform {
            if let Ok(mut node) = instance.clone().try_cast::<Node3D>() {
                node.set_global_transform(transform.to_godot_transform());
            } else if let Ok(mut node) = instance.clone().try_cast::<Node2D>() {
                node.set_global_transform(transform.to_godot_transform_2d());
            } else {
                error!(
                    "attempted to spawn a scene with a transform on Node that did not inherit from Node, the transform was not set"
                )
            }
        }

        match &mut scene.parent {
            Some(parent) => {
                let mut parent = parent.get::<Node>();
                parent.add_child(&instance);
            }
            None => {
                scene_tree.get().get_root().unwrap().add_child(&instance);
            }
        }

        commands.entity(ent).insert(GodotNodeHandle::new(instance));
    }
}
