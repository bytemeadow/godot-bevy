use super::scene_tree::SceneTreeRef;
use crate::interop::{GodotAccess, GodotNodeHandle};
use crate::plugins::assets::GodotResource;
use crate::plugins::signals::{
    DeferredSignalConnectionTrait, DeferredSignalConnections, SignalConnectionSpec, SignalSender,
};
use crate::plugins::transforms::IntoGodotTransform;
use crate::plugins::transforms::IntoGodotTransform2D;
use bevy_app::{App, Plugin, PostUpdate};
use bevy_asset::{Assets, Handle};
use bevy_ecs::error::ContextExt;
use bevy_ecs::event::Event;
use bevy_ecs::prelude::Res;
use bevy_ecs::{
    component::Component,
    entity::Entity,
    query::Without,
    system::{Commands, Query},
};
use bevy_transform::components::Transform;
use godot::classes::{Node, Node2D, Node3D, PackedScene, ResourceLoader};
use godot::obj::{Gd, GodotClass, Inherits};
use godot::prelude::Variant;
use std::collections::HashMap;
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
#[derive(Component)]
pub struct GodotScene {
    resource: GodotSceneResource,
    parent: Option<GodotNodeHandle>,
    deferred_signal_connections: Vec<Box<dyn DeferredSignalConnectionTrait>>,
    deferred_setups: Vec<Box<SceneSetup>>,
}

type SceneSetup = dyn FnOnce(&Gd<Node>) -> bevy_ecs::error::Result + Send + Sync;

impl std::fmt::Debug for GodotScene {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("GodotScene")
            .field("resource", &self.resource)
            .field("parent", &self.parent)
            .field(
                "deferred_signal_connections",
                &self.deferred_signal_connections,
            )
            .field("deferred_setups", &self.deferred_setups.len())
            .finish()
    }
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
            deferred_setups: Vec::new(),
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
            deferred_setups: Vec::new(),
        }
    }

    /// Set the parent node for this scene when spawned.
    pub fn with_parent(mut self, parent: GodotNodeHandle) -> Self {
        self.parent = Some(parent);
        self
    }

    /// Configure the scene root before it enters the scene tree and runs `_ready`.
    /// Setups run once, in the order added, after the entity's `Transform` is applied.
    /// If the root cannot be cast to `T`,
    /// spawning returns an error with the scene, entity, and class names, and
    /// removes the `GodotScene` component to cancel the spawn.
    ///
    /// # Example
    /// ```no_run
    /// # use bevy_ecs::prelude::World;
    /// # use godot::classes::Area2D;
    /// # use godot_bevy::prelude::*;
    /// # let mut world = World::new();
    /// world.spawn(
    ///     GodotScene::from_path("res://scenes/bullet.tscn")
    ///         .with_setup::<Area2D, _>(|root| {
    ///             root.set_collision_mask(4);
    ///         }),
    /// );
    /// ```
    pub fn with_setup<T, F>(mut self, setup: F) -> Self
    where
        T: GodotClass + Inherits<Node>,
        F: FnOnce(&mut Gd<T>) + Send + Sync + 'static,
    {
        self.deferred_setups.push(Box::new(move |root| {
            let mut root = root.clone().try_cast::<T>().map_err(|root| {
                format!(
                    "expected root class {}, actual class {}",
                    T::class_id(),
                    root.get_class()
                )
            })?;
            setup(&mut root);
            Ok(())
        }));
        self
    }

    /// Connect a Godot signal from a child node to trigger a Bevy event.
    /// The signal will be connected when the scene is spawned, and observers
    /// will be triggered when the signal fires.
    ///
    /// # Requirements
    /// This method requires [`GodotSignalsPlugin<T>`] to be added to your app for the
    /// event type `T` you're using. If the plugin is not added, the signal connections
    /// will be ignored and an error will be logged at runtime.
    ///
    /// # Arguments
    /// * `node_path` - Path relative to the scene root (e.g., "VBox/MyButton" or "." for root node).
    ///   Argument supports the same syntax as [Node.get_node](https://docs.godotengine.org/en/stable/classes/class_node.html#class-node-method-get-node).
    /// * `signal_name` - Name of the Godot signal to connect (e.g., "pressed").
    /// * `mapper` - Closure that maps signal arguments to your event type.
    ///   * The closure receives three arguments: `args`, `node_handle`, and `entity`:
    ///     - `args: &[Variant]`: raw Godot arguments (clone if you need detailed parsing).
    ///     - `node_handle: GodotNodeHandle`: emitting node handle.
    ///     - `entity: Option<Entity>`: Bevy entity the GodotScene component is attached to (Always Some).
    ///   * The closure returns an optional event, or None to skip triggering.
    ///
    /// # Example
    /// ```no_run
    /// # use bevy_asset::Handle;
    /// # use bevy_ecs::prelude::{Event, World};
    /// # use godot_bevy::prelude::*;
    /// # #[derive(Event, Clone, Debug)]
    /// # struct MyEvent;
    /// # let scene: Handle<GodotResource> = Handle::default();
    /// # let mut world = World::new();
    /// let mut entity = world.spawn_empty();
    /// entity.insert(
    ///     GodotScene::from_handle(scene).with_signal_connection::<MyEvent, _>(
    ///         "VBox/MyButton",
    ///         "pressed",
    ///         |_args, _node_id, _entity| {
    ///             Some(MyEvent)
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
        T: Event + Clone + Send + std::fmt::Debug + 'static,
        for<'a> T::Trigger<'a>: Default,
        F: Fn(&[Variant], GodotNodeHandle, Option<Entity>) -> Option<T> + Send + Sync + 'static,
    {
        self.deferred_signal_connections
            .push(Box::new(SignalConnectionSpec {
                node_path: node_path.to_string(),
                signal_name: signal_name.to_string(),
                connections: DeferredSignalConnections::<T>::with_connection(signal_name, mapper),
            }));
        self
    }
}

fn spawn_scene(
    mut commands: Commands,
    mut new_scenes: Query<(&mut GodotScene, Entity, Option<&Transform>), Without<GodotNodeHandle>>,
    mut scene_tree: SceneTreeRef,
    assets: Res<Assets<GodotResource>>,
    signal_sender: Option<Res<SignalSender>>,
    mut godot: GodotAccess,
) -> bevy_ecs::error::Result {
    // This avoids repeated ResourceLoader.load() calls when spawning multiple
    // instances of the same scene in a single frame (~22x faster).
    let mut local_cache: HashMap<String, Gd<PackedScene>> = HashMap::new();

    for (mut scene, ent, transform) in new_scenes.iter_mut() {
        let packed_scene: Gd<PackedScene> = match &scene.resource {
            GodotSceneResource::Handle(handle) => {
                let resource = assets
                    .get(handle)
                    .with_context(|| {
                        format!("failed to find packed scene {handle:?} in assets for entity {ent}")
                    })?
                    .get()
                    .clone();
                match resource.try_cast::<PackedScene>() {
                    Ok(ps) => ps,
                    Err(resource) => {
                        error!(
                            "failed to cast scene {:?} for entity {ent} to PackedScene: {resource:?}",
                            scene.resource
                        );
                        continue;
                    }
                }
            }
            GodotSceneResource::Path(path) => {
                if let Some(cached) = local_cache.get(path) {
                    cached.clone()
                } else {
                    let resource = godot
                        .singleton::<ResourceLoader>()
                        .load(path.as_str())
                        .with_context(|| {
                            format!("failed to load packed scene {path:?} for entity {ent}")
                        })?;

                    match resource.try_cast::<PackedScene>() {
                        Ok(ps) => {
                            local_cache.insert(path.clone(), ps.clone());
                            ps
                        }
                        Err(resource) => {
                            error!(
                                "failed to cast scene {:?} for entity {ent} to PackedScene: {resource:?}",
                                scene.resource
                            );
                            continue;
                        }
                    }
                }
            }
        };

        let instance = match packed_scene.instantiate() {
            Some(instance) => instance,
            None => {
                error!(
                    "failed to instantiate packed scene {:?} for entity {ent}",
                    scene.resource
                );
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
                    "failed to set transform on scene {:?} for entity {ent}: root is neither Node2D nor Node3D",
                    scene.resource
                )
            }
        }

        for setup in std::mem::take(&mut scene.deferred_setups) {
            if let Err(error) = setup(&instance).with_context(|| {
                format!(
                    "failed to set up scene {:?} for entity {ent}",
                    scene.resource
                )
            }) {
                instance.free();
                commands.entity(ent).remove::<GodotScene>();
                return Err(error);
            }
        }

        if !scene.deferred_signal_connections.is_empty() {
            if let Some(ref sender) = signal_sender {
                for deferred_connection in scene.deferred_signal_connections.drain(..) {
                    deferred_connection.connect(&instance, ent, sender);
                }
            } else {
                error!(
                    "failed to connect signals for scene {:?} on entity {ent}: GodotSignalsPlugin is not added. \
                     Add GodotSignalsPlugin<YourEventType> to enable signal connections.",
                    scene.resource
                );
                scene.deferred_signal_connections.clear();
            }
        }

        match scene.parent {
            Some(parent_id) => {
                let mut parent = godot.try_get::<Node>(parent_id).with_context(|| {
                    format!(
                        "failed to find parent {parent_id:?} for scene {:?} on entity {ent}",
                        scene.resource
                    )
                })?;
                parent.add_child(&instance);
            }
            None => {
                scene_tree
                    .get()
                    .get_root()
                    .with_context(|| {
                        format!(
                            "failed to find scene tree root for scene {:?} on entity {ent}",
                            scene.resource
                        )
                    })?
                    .add_child(&instance);
            }
        }

        commands.entity(ent).insert(GodotNodeHandle::new(instance));
    }
    Ok(())
}
