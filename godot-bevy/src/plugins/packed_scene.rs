use super::scene_tree::SceneTreeRef;
use crate::plugins::assets::GodotResource;
use crate::plugins::signals::{
    DeferredSignalConnection, GlobalTypedSignalSender, SignalConnectionSpec,
    TypedDeferredSignalConnections,
};
use crate::plugins::transforms::IntoGodotTransform2D;
use crate::prelude::main_thread_system;
use crate::{interop::GodotNodeHandle, plugins::transforms::IntoGodotTransform};
use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{Assets, Handle};
use bevy_ecs::message::Message;
use bevy_ecs::system::NonSend;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::Without,
    system::{Commands, Query, ResMut},
};
use bevy_transform::components::Transform;
use godot::prelude::Variant;
use godot::{
    builtin::GString,
    classes::{Node, Node2D, Node3D, PackedScene, ResourceLoader},
    obj::Singleton,
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
    deferred_signal_connections: Vec<Box<dyn DeferredSignalConnection>>,
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
            deferred_signal_connections: Vec::new(),
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
            deferred_signal_connections: Vec::new(),
        }
    }

    /// Set the parent node for this scene when spawned.
    pub fn with_parent(mut self, parent: GodotNodeHandle) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Connect a typed Godot signal from a child node to a Bevy `Message`.
    /// The signal will be connected when the scene is spawned.
    ///
    /// # Arguments
    /// * `node_path` - Path to the node relative to the scene root (e.g., "VBox/MyButton").
    /// * `signal_name` - Name of the Godot signal to connect (e.g., "pressed").
    /// * `mapper` - Closure that maps signal arguments to your typed message.
    ///
    /// # Example
    /// ```ignore
    /// let scene: Handle<GodotResource> = ...;
    /// let entity = world.spawn_empty();
    /// entity.insert(
    ///     GodotScene::from_handle(scene).with_signal_connection::<MyMessage>(
    ///         "VBox/MyButton",
    ///         "pressed",
    ///         |args, _node, _entity| {
    ///             Some(MyMessage::from_args(args))
    ///         }
    ///     )
    /// );
    /// ```
    pub fn with_signal_connection<T, F>(
        mut self,
        node_path: &str,
        signal_name: &str,
        mapper: F,
    ) -> Self
    where
        T: Message + Send + std::fmt::Debug + 'static,
        F: Fn(&[Variant], &GodotNodeHandle, Option<Entity>) -> Option<T> + Send + Sync + 'static,
    {
        self.deferred_signal_connections
            .push(Box::new(SignalConnectionSpec {
                node_path: node_path.to_string(),
                signal_name: signal_name.to_string(),
                connections: TypedDeferredSignalConnections::<T>::with_connection(
                    signal_name,
                    mapper,
                ),
            }));
        self
    }
}

#[main_thread_system]
fn spawn_scene(
    mut commands: Commands,
    mut new_scenes: Query<(&mut GodotScene, Entity, Option<&Transform>), Without<GodotNodeHandle>>,
    mut scene_tree: SceneTreeRef,
    mut assets: ResMut<Assets<GodotResource>>,
    typed_message_sender: NonSend<GlobalTypedSignalSender>,
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

        // Connect signals
        for deferred_connection in scene.deferred_signal_connections.drain(..) {
            deferred_connection.connect(&instance, ent, &typed_message_sender);
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
