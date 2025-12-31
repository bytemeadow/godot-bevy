use super::node_type_checking_generated::{
    add_comprehensive_node_type_markers, add_node_type_markers_from_string,
    remove_comprehensive_node_type_markers,
};
use crate::plugins::core::SceneTreeComponentRegistry;
use crate::prelude::{GodotScene, main_thread_system};
use crate::{
    interop::{GodotNodeHandle, GodotNodeId},
    plugins::collisions::{
        AREA_ENTERED, AREA_EXITED, BODY_ENTERED, BODY_EXITED, COLLISION_START_SIGNALS,
        CollisionMessageType, Collisions,
    },
};
use bevy_app::{App, First, Plugin, PreStartup};
use bevy_ecs::{
    component::Component,
    entity::Entity,
    message::{Message, MessageReader, MessageWriter, message_update_system},
    prelude::{Name, ReflectComponent, ReflectResource, Resource},
    schedule::IntoScheduleConfigs,
    system::{Commands, NonSendMut, Query, Res, ResMut, SystemParam},
};
use bevy_reflect::Reflect;
use godot::{
    builtin::GString,
    classes::{Engine, Node, SceneTree},
    meta::ToGodot,
    obj::{Gd, Inherits, InstanceId, Singleton},
    prelude::GodotConvert,
};
use std::collections::HashMap;
use std::marker::PhantomData;
use tracing::{debug, trace, warn};

/// A resource that maintains an O(1) lookup from Godot `InstanceId` to Bevy `Entity`.
///
/// This index is automatically maintained by the scene tree plugin as entities are
/// added and removed. Use this for efficient collision handling, signal routing,
/// and any scenario where you need to find the Bevy entity for a Godot node.
///
/// # Example
///
/// ```ignore
/// fn handle_collision(
///     index: Res<NodeEntityIndex>,
///     // ... other params
/// ) {
///     let colliding_instance_id = /* from collision event */;
///     if let Some(entity) = index.get(colliding_instance_id) {
///         // Do something with the entity
///     }
/// }
/// ```
#[derive(Resource, Default, Debug, Reflect)]
#[reflect(Resource)]
pub struct NodeEntityIndex {
    #[reflect(ignore)]
    index: HashMap<InstanceId, Entity>,
}

impl NodeEntityIndex {
    /// Look up the Bevy `Entity` for a Godot `InstanceId`.
    ///
    /// Returns `None` if no entity is registered for this instance ID.
    #[inline]
    pub fn get(&self, instance_id: InstanceId) -> Option<Entity> {
        self.index.get(&instance_id).copied()
    }

    /// Check if an entity exists for the given `InstanceId`.
    #[inline]
    pub fn contains(&self, instance_id: InstanceId) -> bool {
        self.index.contains_key(&instance_id)
    }

    /// Look up the Bevy `Entity` for a Godot `GodotNodeId`.
    #[inline]
    pub fn get_id(&self, node_id: GodotNodeId) -> Option<Entity> {
        self.get(node_id.instance_id())
    }

    /// Check if an entity exists for the given `GodotNodeId`.
    #[inline]
    pub fn contains_id(&self, node_id: GodotNodeId) -> bool {
        self.contains(node_id.instance_id())
    }

    /// Returns the number of entries in the index.
    #[inline]
    pub fn len(&self) -> usize {
        self.index.len()
    }

    /// Returns true if the index is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.index.is_empty()
    }

    /// Insert a mapping from `InstanceId` to `Entity`.
    ///
    /// This is called internally by the scene tree plugin.
    #[inline]
    pub(crate) fn insert(&mut self, instance_id: InstanceId, entity: Entity) {
        self.index.insert(instance_id, entity);
    }

    /// Remove a mapping by `InstanceId`.
    ///
    /// This is called internally by the scene tree plugin.
    #[inline]
    pub(crate) fn remove(&mut self, instance_id: InstanceId) -> Option<Entity> {
        self.index.remove(&instance_id)
    }
}

/// Unified scene tree plugin that provides:
/// - SceneTreeRef for accessing the Godot scene tree
/// - Scene tree messages (NodeAdded, NodeRemoved, NodeRenamed)
/// - Automatic entity creation and mirroring for scene tree nodes
/// - Custom GodotChildOf/GodotChildren relationship for scene tree hierarchy
///
/// This plugin is always included in the core plugins and provides
/// complete scene tree integration out of the box.
pub struct GodotSceneTreePlugin {
    /// When true, despawning a parent entity will automatically despawn all children
    /// via the GodotChildren on_despawn hook.
    ///
    /// Set to false if you want to manually manage entity lifetimes independently
    /// of the Godot scene tree (e.g., for object pooling or entities that outlive their nodes).
    ///
    /// `ProtectedNodeEntity` children are never despawned automatically.
    pub auto_despawn_children: bool,
}

impl Default for GodotSceneTreePlugin {
    fn default() -> Self {
        Self {
            auto_despawn_children: true,
        }
    }
}

/// Configuration resource for scene tree behavior
#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct SceneTreeConfig {
    /// When true, despawning a parent entity will automatically despawn all children
    /// via the GodotChildren on_despawn hook.
    ///
    /// Set to false if you want to manually manage entity lifetimes independently
    /// of the Godot scene tree (e.g., for object pooling or entities that outlive their nodes).
    ///
    /// `ProtectedNodeEntity` children are never despawned automatically.
    pub auto_despawn_children: bool,
}

impl Plugin for GodotSceneTreePlugin {
    fn build(&self, app: &mut App) {
        // Auto-register all discovered AutoSyncBundle plugins
        super::autosync::register_all_autosync_bundles(app);

        app.init_non_send_resource::<SceneTreeRefImpl>()
            .init_resource::<NodeEntityIndex>()
            .insert_resource(SceneTreeConfig {
                auto_despawn_children: self.auto_despawn_children,
            })
            .add_message::<SceneTreeMessage>()
            .add_systems(
                PreStartup,
                (connect_scene_tree, initialize_scene_tree).chain(),
            )
            .add_systems(
                First,
                (
                    write_scene_tree_messages.before(message_update_system),
                    read_scene_tree_messages.before(message_update_system),
                ),
            );
    }
}

#[derive(SystemParam)]
pub struct SceneTreeRef<'w, 's> {
    gd: NonSendMut<'w, SceneTreeRefImpl>,
    phantom: PhantomData<&'s ()>,
}

impl<'w, 's> SceneTreeRef<'w, 's> {
    pub fn get(&mut self) -> Gd<SceneTree> {
        self.gd.0.clone()
    }
}

#[doc(hidden)]
#[derive(Debug)]
pub(crate) struct SceneTreeRefImpl(Gd<SceneTree>);

impl SceneTreeRefImpl {
    fn get_ref() -> Gd<SceneTree> {
        Engine::singleton()
            .get_main_loop()
            .unwrap()
            .cast::<SceneTree>()
    }
}

impl Default for SceneTreeRefImpl {
    fn default() -> Self {
        Self(Self::get_ref())
    }
}

#[main_thread_system]
fn initialize_scene_tree(
    mut commands: Commands,
    mut scene_tree: SceneTreeRef,
    mut entities: Query<(&mut GodotNodeHandle, Entity, Option<&ProtectedNodeEntity>)>,
    component_registry: Res<SceneTreeComponentRegistry>,
    mut node_index: ResMut<NodeEntityIndex>,
) {
    let root = scene_tree.get().get_root().unwrap();

    // Check if we have the optimized GDScript watcher for type pre-analysis
    let optimized_watcher = root
        .try_get_node_as::<Node>("/root/BevyAppSingleton/OptimizedSceneTreeWatcher")
        .or_else(|| root.try_get_node_as::<Node>("BevyAppSingleton/OptimizedSceneTreeWatcher"));

    let messages = if let Some(mut watcher) = optimized_watcher {
        // Use optimized GDScript watcher to analyze the initial tree with type information
        tracing::info!("Using optimized initial tree analysis with type pre-analysis");

        let analysis_result = watcher.call("analyze_initial_tree", &[]);
        let result_dict = analysis_result.to::<godot::builtin::VarDictionary>();
        let instance_ids = result_dict
            .get("instance_ids")
            .unwrap()
            .to::<godot::builtin::PackedInt64Array>();
        let node_types = result_dict
            .get("node_types")
            .unwrap()
            .to::<godot::builtin::PackedStringArray>();

        let mut messages = Vec::new();
        let len = instance_ids.len().min(node_types.len());
        for i in 0..len {
            if let (Some(id), Some(type_gstring)) = (instance_ids.get(i), node_types.get(i)) {
                let type_str = type_gstring.to_string();

                messages.push(SceneTreeMessage {
                    node_id: GodotNodeId::from(godot::prelude::InstanceId::from_i64(
                        id,
                    )),
                    message_type: SceneTreeMessageType::NodeAdded,
                    node_type: Some(type_str),
                });
            }
        }

        messages
    } else {
        // Use fallback traversal without type optimization
        tracing::info!("Using fallback initial tree analysis (no type optimization)");
        traverse_fallback(root.upcast())
    };

    create_scene_tree_entity(
        &mut commands,
        messages,
        &mut scene_tree,
        &mut entities,
        &component_registry,
        &mut node_index,
    );
}

fn traverse_fallback(node: Gd<Node>) -> Vec<SceneTreeMessage> {
    fn traverse_recursive(node: Gd<Node>, messages: &mut Vec<SceneTreeMessage>) {
        messages.push(SceneTreeMessage {
            node_id: GodotNodeId::from(node.instance_id()),
            message_type: SceneTreeMessageType::NodeAdded,
            node_type: None, // No type optimization available
        });

        for child in node.get_children().iter_shared() {
            traverse_recursive(child, messages);
        }
    }

    let mut messages = Vec::new();
    traverse_recursive(node, &mut messages);
    messages
}

#[derive(Debug, Clone, Message)]
pub struct SceneTreeMessage {
    pub node_id: GodotNodeId,
    pub message_type: SceneTreeMessageType,
    pub node_type: Option<String>, // Pre-analyzed node type from GDScript watcher
}

#[derive(Copy, Clone, Debug, GodotConvert)]
#[godot(via = GString)]
pub enum SceneTreeMessageType {
    NodeAdded,
    NodeRemoved,
    NodeRenamed,
}

/// Helper function to recursively search for a node by name
fn find_node_by_name(parent: &Gd<Node>, name: &str) -> Option<Gd<Node>> {
    // Check if this node matches
    if parent.get_name().to_string() == name {
        return Some(parent.clone());
    }

    // Search children recursively
    for i in 0..parent.get_child_count() {
        if let Some(child) = parent.get_child(i) {
            let child_node = child.cast::<Node>();
            if let Some(found) = find_node_by_name(&child_node, name) {
                return Some(found);
            }
        }
    }

    None
}

#[main_thread_system]
fn connect_scene_tree(mut scene_tree: SceneTreeRef) {
    let mut scene_tree_gd = scene_tree.get();
    let root = scene_tree_gd.get_root().unwrap();

    // Try multiple paths to find the SceneTreeWatcher - support both production and test environments
    let watcher = root
        .try_get_node_as::<Node>("/root/BevyAppSingleton/SceneTreeWatcher")
        .or_else(|| {
            // Try without the full path for test environments
            root.try_get_node_as::<Node>("BevyAppSingleton/SceneTreeWatcher")
        })
        .or_else(|| {
            // Fallback: search entire tree for any SceneTreeWatcher (for test environments)
            tracing::debug!("Searching entire scene tree for SceneTreeWatcher");
            find_node_by_name(&root.clone().upcast(), "SceneTreeWatcher")
        })
        .unwrap_or_else(|| {
            panic!("SceneTreeWatcher not found. Searched /root/BevyAppSingleton/SceneTreeWatcher, BevyAppSingleton/SceneTreeWatcher, and entire tree.");
        });

    // Check if we have the optimized GDScript watcher
    let optimized_watcher = root
        .try_get_node_as::<Node>("/root/BevyAppSingleton/OptimizedSceneTreeWatcher")
        .or_else(|| root.try_get_node_as::<Node>("BevyAppSingleton/OptimizedSceneTreeWatcher"))
        .or_else(|| {
            // Fallback: search entire tree
            find_node_by_name(&root.clone().upcast(), "OptimizedSceneTreeWatcher")
        });

    if optimized_watcher.is_some() {
        // The optimized GDScript watcher handles scene tree connections and forwards
        // pre-analyzed messages to the Rust watcher (which has the MPSC sender)
        // No need to connect here - it connects automatically in its _ready()
        tracing::info!("Using optimized GDScript scene tree watcher with type pre-analysis");
    } else {
        // Fallback to direct connection without type optimization
        tracing::info!("Using fallback scene tree connection (no type optimization)");

        scene_tree_gd.connect(
            "node_added",
            &watcher
                .callable("scene_tree_event")
                .bind(&[SceneTreeMessageType::NodeAdded.to_variant()]),
        );

        scene_tree_gd.connect(
            "node_removed",
            &watcher
                .callable("scene_tree_event")
                .bind(&[SceneTreeMessageType::NodeRemoved.to_variant()]),
        );

        scene_tree_gd.connect(
            "node_renamed",
            &watcher
                .callable("scene_tree_event")
                .bind(&[SceneTreeMessageType::NodeRenamed.to_variant()]),
        );
    }
}

#[derive(Component, Debug, Reflect)]
#[reflect(Component)]
pub struct Groups {
    groups: Vec<String>,
}

impl Groups {
    pub fn is(&self, group_name: &str) -> bool {
        self.groups.iter().any(|name| name == group_name)
    }
}

impl<T: Inherits<Node>> From<&Gd<T>> for Groups {
    fn from(node: &Gd<T>) -> Self {
        Groups {
            groups: node
                .clone()
                .upcast::<Node>()
                .get_groups()
                .iter_shared()
                .map(|variant| variant.to_string())
                .collect(),
        }
    }
}

#[doc(hidden)]
pub struct SceneTreeMessageReader(pub std::sync::mpsc::Receiver<SceneTreeMessage>);

fn write_scene_tree_messages(
    message_reader: NonSendMut<SceneTreeMessageReader>,
    mut message_writer: MessageWriter<SceneTreeMessage>,
) {
    message_writer.write_batch(message_reader.0.try_iter());
}

/// Marks an entity so it is not despawned when its corresponding Godot Node is freed, breaking
/// the usual 1-to-1 lifetime between them. This allows game logic to keep running on entities
/// that have no Node, such as simulating off-screen factory machines or NPCs in inactive scenes.
/// A Godot Node can be re-associated later by adding a `GodotScene` component to the **entity.**
#[derive(Component)]
pub struct ProtectedNodeEntity;

fn create_scene_tree_entity(
    commands: &mut Commands,
    messages: impl IntoIterator<Item = SceneTreeMessage>,
    scene_tree: &mut SceneTreeRef,
    entities: &mut Query<(&mut GodotNodeHandle, Entity, Option<&ProtectedNodeEntity>)>,
    component_registry: &SceneTreeComponentRegistry,
    node_index: &mut NodeEntityIndex,
) {
    // Build local mapping with protection info (only needed during this function)
    let mut ent_mapping = entities
        .iter()
        .map(|(reference, ent, protected)| (reference.instance_id(), (ent, protected)))
        .collect::<HashMap<_, _>>();
    let scene_root = scene_tree.get().get_root().unwrap();

    // CollisionWatcher is optional - only required if GodotCollisionsPlugin is added
    let collision_watcher = scene_root
        .try_get_node_as::<Node>("/root/BevyAppSingleton/CollisionWatcher")
        .or_else(|| {
            // Try without the full path for test environments
            scene_root.try_get_node_as::<Node>("BevyAppSingleton/CollisionWatcher")
        })
        .or_else(|| {
            // Fallback: search entire tree for any CollisionWatcher (for test environments)
            tracing::debug!("Searching entire scene tree for CollisionWatcher");
            find_node_by_name(&scene_root.clone().upcast(), "CollisionWatcher")
        });

    for message in messages.into_iter() {
        trace!(target: "godot_scene_tree_messages", message = ?message);

        let SceneTreeMessage {
            node_id,
            message_type,
            node_type,
        } = message;
        let instance_id = node_id.instance_id();
        let mut node_handle = GodotNodeHandle::from_id(node_id);
        let ent = ent_mapping.get(&instance_id).cloned();

        match message_type {
            SceneTreeMessageType::NodeAdded => {
                // Skip nodes that have been freed before we process them (can happen in tests)
                if !instance_id.lookup_validity() {
                    continue;
                }

                let mut ent = if let Some((ent, _)) = ent {
                    commands.entity(ent)
                } else {
                    commands.spawn_empty()
                };

                ent.insert(GodotNodeHandle::from_id(node_id))
                    .insert(Name::from(
                        node_handle.get::<Node>().get_name().to_string(),
                    ));

                // Add node type marker components - use optimized version if available
                if let Some(ref node_type_str) = node_type {
                    // Use pre-analyzed type from GDScript watcher (much faster)
                    add_node_type_markers_from_string(&mut ent, node_type_str);
                } else {
                    // Fallback to comprehensive analysis with FFI calls
                    add_comprehensive_node_type_markers(&mut ent, &mut node_handle);
                }

                let mut node = node_handle.get::<Node>();

                // Check if the node is a collision body (Area2D, Area3D, RigidBody2D, RigidBody3D, etc.)
                // These nodes typically have collision detection capabilities
                // Only connect if CollisionWatcher exists (i.e., GodotCollisionsPlugin was added)
                if let Some(ref collision_watcher) = collision_watcher {
                    let is_collision_body = COLLISION_START_SIGNALS
                        .iter()
                        .any(|&signal| node.has_signal(signal));

                    if is_collision_body {
                        debug!(target: "godot_scene_tree_collisions",
                               node_id = node.instance_id().to_string(),
                               "is collision body");

                        let node_clone = node.clone();

                        if node.has_signal(BODY_ENTERED) {
                            node.connect(
                                BODY_ENTERED,
                                &collision_watcher.callable("collision_event").bind(&[
                                    node_clone.to_variant(),
                                    CollisionMessageType::Started.to_variant(),
                                ]),
                            );
                        }

                        if node.has_signal(BODY_EXITED) {
                            node.connect(
                                BODY_EXITED,
                                &collision_watcher.callable("collision_event").bind(&[
                                    node_clone.to_variant(),
                                    CollisionMessageType::Ended.to_variant(),
                                ]),
                            );
                        }

                        if node.has_signal(AREA_ENTERED) {
                            node.connect(
                                AREA_ENTERED,
                                &collision_watcher.callable("collision_event").bind(&[
                                    node_clone.to_variant(),
                                    CollisionMessageType::Started.to_variant(),
                                ]),
                            );
                        }

                        if node.has_signal(AREA_EXITED) {
                            node.connect(
                                AREA_EXITED,
                                &collision_watcher.callable("collision_event").bind(&[
                                    node_clone.to_variant(),
                                    CollisionMessageType::Ended.to_variant(),
                                ]),
                            );
                        }

                        // Add Collisions component to track collision state
                        ent.insert(Collisions::default());
                    }
                }

                ent.insert(Groups::from(&node));

                // Add all components registered by plugins
                component_registry.add_to_entity(&mut ent, &mut node_handle);

                let ent = ent.id();
                ent_mapping.insert(instance_id, (ent, None));
                node_index.insert(instance_id, ent);

                // Try to add any registered bundles for this node type
                super::autosync::try_add_bundles_for_node(commands, ent, &mut node_handle);

                // Add GodotChildOf relationship to mirror Godot's scene tree hierarchy
                if node.instance_id() != scene_root.instance_id()
                    && let Some(parent) = node.get_parent()
                {
                    let parent_id = parent.instance_id();
                    if let Some((parent_entity, _)) = ent_mapping.get(&parent_id) {
                        commands
                            .entity(ent)
                            .insert(super::relationship::GodotChildOf(*parent_entity));
                    } else {
                        warn!(target: "godot_scene_tree_messages",
                            "Parent entity with ID {} not found in ent_mapping. This might indicate a missing or incorrect mapping.",
                            parent_id);
                    }
                }
            }
            SceneTreeMessageType::NodeRemoved => {
                if let Some((ent, prot_opt)) = ent {
                    // Check if node is being reparented vs truly removed
                    // During reparenting, the node is temporarily removed from old parent
                    // but still exists in the scene tree (has a parent)
                    // We need to try_get because the node handle might be invalid if freed
                    let is_reparenting = node_handle
                        .try_get::<Node>()
                        .map(|godot_node| godot_node.get_parent().is_some())
                        .unwrap_or(false);

                    if is_reparenting {
                        // Node is being reparented - don't despawn entity, it will be re-added
                        trace!(target: "godot_scene_tree_events",
                            "Node is being reparented, preserving entity");
                        // Don't remove from ent_mapping - entity still valid
                    } else {
                        // Node is truly being removed (freed or despawned)
                        let protected = prot_opt.is_some();
                        if !protected {
                            commands.entity(ent).despawn();
                        } else {
                            _strip_godot_components(commands, ent, &mut node_handle);
                        }
                        ent_mapping.remove(&instance_id);
                        node_index.remove(instance_id);
                    }
                } else {
                    // Entity was already despawned (common when using queue_free)
                    trace!(target: "godot_scene_tree_messages", "Entity for removed node was already despawned");
                }
            }
            SceneTreeMessageType::NodeRenamed => {
                if let Some((ent, _)) = ent {
                    commands
                        .entity(ent)
                        .insert(Name::from(
                            node_handle.get::<Node>().get_name().to_string(),
                        ));
                } else {
                    trace!(target: "godot_scene_tree_messages", "Entity for renamed node was already despawned");
                }
            }
        }
    }
}

fn _strip_godot_components(commands: &mut Commands, ent: Entity, node: &mut GodotNodeHandle) {
    let mut entity_commands = commands.entity(ent);

    entity_commands.remove::<GodotNodeHandle>();
    entity_commands.remove::<GodotScene>();
    entity_commands.remove::<Name>();
    entity_commands.remove::<Groups>();

    remove_comprehensive_node_type_markers(&mut entity_commands, node);
}

#[main_thread_system]
#[allow(clippy::too_many_arguments)]
fn read_scene_tree_messages(
    mut commands: Commands,
    mut scene_tree: SceneTreeRef,
    mut message_reader: MessageReader<SceneTreeMessage>,
    mut entities: Query<(&mut GodotNodeHandle, Entity, Option<&ProtectedNodeEntity>)>,
    component_registry: Res<SceneTreeComponentRegistry>,
    mut node_index: ResMut<NodeEntityIndex>,
) {
    create_scene_tree_entity(
        &mut commands,
        message_reader.read().cloned(),
        &mut scene_tree,
        &mut entities,
        &component_registry,
        &mut node_index,
    );
}
