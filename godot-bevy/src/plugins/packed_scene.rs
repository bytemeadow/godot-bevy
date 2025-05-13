use std::str::FromStr;

use bevy::{
    app::{App, Plugin, PostUpdate},
    ecs::{
        component::Component,
        entity::Entity,
        query::Without,
        system::{Commands, Query},
    },
    log::tracing,
};
use godot::{
    builtin::{GString, Transform2D, Transform3D, Vector2, Vector3},
    classes::{Node2D, Node3D, PackedScene, ResourceLoader},
};

use crate::bridge::{GodotNodeHandle, GodotResourceHandle};

use super::core::SceneTreeRef;

pub struct PackedScenePlugin;
impl Plugin for PackedScenePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PostUpdate, spawn_scene);
    }
}

/// A to-be-instanced-and-spawned Godot scene.
///
/// [`GodotScene`]s that are spawned/inserted into the bevy world will be instanced from the provided
/// handle/path and the instance will be added as an [`GodotNodeHandle`] in the next PostUpdateFlush set.
/// (see [`spawn_scene`])
#[derive(Debug, Component)]
pub struct GodotScene {
    resource: GodotSceneResource,
    transform: Option<GodotSceneTransform>,
}

#[derive(Debug)]
enum GodotSceneResource {
    Resource(GodotResourceHandle),
    Path(String),
}

#[derive(Debug)]
enum GodotSceneTransform {
    Transform2D(Transform2D),
    Transform3D(Transform3D),
}

impl GodotScene {
    /// Instantiate the godot scene from an GodotResourceHandle.
    pub fn from_resource(res: GodotResourceHandle) -> Self {
        Self {
            resource: GodotSceneResource::Resource(res),
            transform: None,
        }
    }

    /// Instantiate the godot scene from the given path.
    ///
    /// Note that this will call [`ResourceLoader`].load() - which is a blocking load.
    /// If you want "preload" functionality, you should load your resources into a Bevy [`Resource`]
    /// and use from_resource().
    pub fn from_path(path: &str) -> Self {
        Self {
            resource: GodotSceneResource::Path(path.to_string()),
            transform: None,
        }
    }

    pub fn with_transform3d(mut self, transform: Transform3D) -> Self {
        self.transform = Some(GodotSceneTransform::Transform3D(transform));
        self
    }

    pub fn with_transform2d(mut self, transform: Transform2D) -> Self {
        self.transform = Some(GodotSceneTransform::Transform2D(transform));
        self
    }

    pub fn with_translation3d(mut self, translation: Vector3) -> Self {
        self.transform = Some(GodotSceneTransform::Transform3D(
            Transform3D::IDENTITY.translated(translation),
        ));
        self
    }

    pub fn with_translation2d(mut self, translation: Vector2) -> Self {
        self.transform = Some(GodotSceneTransform::Transform2D(
            Transform2D::IDENTITY.translated(translation),
        ));
        self
    }
}

#[derive(Component, Debug, Default)]
struct GodotSceneSpawned;

fn spawn_scene(
    mut commands: Commands,
    mut new_scenes: Query<(&mut GodotScene, Entity), Without<GodotSceneSpawned>>,
    mut scene_tree: SceneTreeRef,
) {
    for (mut scene, ent) in new_scenes.iter_mut() {
        let packed_scene = match &mut scene.resource {
            GodotSceneResource::Resource(res) => res.get(),
            GodotSceneResource::Path(path) => ResourceLoader::singleton()
                .load(&GString::from_str(path).expect("path to be a valid GString"))
                .expect("packed scene to load"),
        };

        let instance = packed_scene
            .try_cast::<PackedScene>()
            .expect("resource to be a packed scene")
            .instantiate()
            .unwrap();

        match scene_tree
            .get()
            .get_root()
            .unwrap()
            .get_node_or_null("BevyAppSingleton")
        {
            Some(mut app) => app.add_child(&instance),
            None => {
                tracing::error!(
                    "attempted to add a child to the BevyAppSingleton autoload, but the BevyAppSingleton autoload wasn't found"
                );
                return;
            }
        }

        if let Some(transform) = &scene.transform {
            match transform {
                GodotSceneTransform::Transform2D(transform) => {
                    match instance.clone().try_cast::<Node2D>().ok() {
                        Some(mut node2d) => node2d.set_global_transform(*transform),
                        None => tracing::error!(
                            "attempted to spawn a scene with a transform on Node that did not inherit from Node3D, the transform was not set"
                        ),
                    }
                }
                GodotSceneTransform::Transform3D(transform) => {
                    match instance.clone().try_cast::<Node3D>().ok() {
                        Some(mut node3d) => node3d.set_global_transform(*transform),
                        None => tracing::error!(
                            "attempted to spawn a scene with a transform on Node that did not inherit from Node3D, the transform was not set"
                        ),
                    }
                }
            }
        }

        commands
            .entity(ent)
            .insert(GodotNodeHandle::new(instance))
            .insert(GodotSceneSpawned);
    }
}
