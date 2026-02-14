use super::node_type_checking::{
    add_node_type_markers_from_string, remove_comprehensive_node_type_markers,
};
use crate::plugins::core::SceneTreeComponentRegistry;
use crate::prelude::GodotScene;
use crate::{
    interop::{GodotAccess, GodotNodeHandle},
    plugins::collisions::{
        AREA_ENTERED, AREA_EXITED, BODY_ENTERED, BODY_EXITED, CollisionMessageType,
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
use godot::classes::ClassDb;
use godot::{
    builtin::{GString, StringName},
    classes::{Engine, Node, SceneTree},
    meta::ToGodot,
    obj::{Gd, Inherits, InstanceId, Singleton},
    prelude::GodotConvert,
};
use parking_lot::Mutex;
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

    /// Look up the Bevy `Entity` for a Godot `GodotNodeHandle`.
    #[inline]
    pub fn get_handle(&self, handle: GodotNodeHandle) -> Option<Entity> {
        self.get(handle.instance_id())
    }

    /// Check if an entity exists for the given `GodotNodeHandle`.
    #[inline]
    pub fn contains_handle(&self, handle: GodotNodeHandle) -> bool {
        self.contains(handle.instance_id())
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

fn initialize_scene_tree(
    mut commands: Commands,
    mut scene_tree: SceneTreeRef,
    mut entities: Query<(&GodotNodeHandle, Entity, Option<&ProtectedNodeEntity>)>,
    component_registry: Res<SceneTreeComponentRegistry>,
    mut node_index: ResMut<NodeEntityIndex>,
    mut godot: GodotAccess,
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
        let node_names = result_dict
            .get("node_names")
            .map(|value| value.to::<godot::builtin::PackedStringArray>());
        let parent_ids = result_dict
            .get("parent_ids")
            .map(|value| value.to::<godot::builtin::PackedInt64Array>());
        let collision_masks = result_dict
            .get("collision_masks")
            .map(|value| value.to::<godot::builtin::PackedInt64Array>());
        // Groups is optional - only present in v2+ of the addon
        let groups_array = result_dict
            .get("groups")
            .map(|value| value.to::<godot::builtin::VarArray>());

        let mut messages = Vec::new();
        let len = instance_ids.len().min(node_types.len());
        for i in 0..len {
            if let (Some(id), Some(type_gstring)) = (instance_ids.get(i), node_types.get(i)) {
                let type_str = type_gstring.to_string();
                let node_name = node_names
                    .as_ref()
                    .and_then(|names| names.get(i))
                    .map(|name| name.to_string());
                let parent_id =
                    parent_ids
                        .as_ref()
                        .and_then(|ids| ids.get(i))
                        .and_then(|parent_id| {
                            if parent_id > 0 {
                                Some(InstanceId::from_i64(parent_id))
                            } else {
                                None
                            }
                        });
                let collision_mask = collision_masks
                    .as_ref()
                    .and_then(|masks| masks.get(i))
                    .and_then(|mask| u8::try_from(mask).ok());
                // Parse groups if available (v2+ addon)
                let groups = groups_array.as_ref().and_then(|arr| {
                    arr.get(i).map(|variant| {
                        let packed = variant.to::<godot::builtin::PackedStringArray>();
                        packed
                            .as_slice()
                            .iter()
                            .map(|s| s.to_string())
                            .collect::<Vec<_>>()
                    })
                });

                messages.push(SceneTreeMessage {
                    node_id: GodotNodeHandle::from(godot::prelude::InstanceId::from_i64(id)),
                    message_type: SceneTreeMessageType::NodeAdded,
                    node_type: Some(type_str),
                    node_name,
                    parent_id,
                    collision_mask,
                    groups,
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
        &mut godot,
    );
}

fn traverse_fallback(node: Gd<Node>) -> Vec<SceneTreeMessage> {
    fn traverse_recursive(node: Gd<Node>, messages: &mut Vec<SceneTreeMessage>) {
        messages.push(SceneTreeMessage {
            node_id: GodotNodeHandle::from(node.instance_id()),
            message_type: SceneTreeMessageType::NodeAdded,
            node_type: None, // No type optimization available
            node_name: None,
            parent_id: None,
            collision_mask: None,
            groups: None, // No groups optimization available
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
    pub node_id: GodotNodeHandle,
    pub message_type: SceneTreeMessageType,
    pub node_type: Option<String>, // Pre-analyzed node type from GDScript watcher
    pub node_name: Option<String>,
    pub parent_id: Option<InstanceId>,
    pub collision_mask: Option<u8>,
    pub groups: Option<Vec<String>>, // Pre-analyzed groups from GDScript watcher (v2+)
}

#[derive(Copy, Clone, Debug, GodotConvert)]
#[godot(via = GString)]
pub enum SceneTreeMessageType {
    NodeAdded,
    NodeRemoved,
    NodeRenamed,
}

const COLLISION_MASK_BODY_ENTERED: u8 = 1 << 0;
const COLLISION_MASK_BODY_EXITED: u8 = 1 << 1;
const COLLISION_MASK_AREA_ENTERED: u8 = 1 << 2;
const COLLISION_MASK_AREA_EXITED: u8 = 1 << 3;

fn collision_mask_from_node(node: &mut Node) -> u8 {
    let mut mask = 0;
    if node.has_signal(BODY_ENTERED) {
        mask |= COLLISION_MASK_BODY_ENTERED;
    }
    if node.has_signal(BODY_EXITED) {
        mask |= COLLISION_MASK_BODY_EXITED;
    }
    if node.has_signal(AREA_ENTERED) {
        mask |= COLLISION_MASK_AREA_ENTERED;
    }
    if node.has_signal(AREA_EXITED) {
        mask |= COLLISION_MASK_AREA_EXITED;
    }
    mask
}

fn collision_mask_has(mask: u8, flag: u8) -> bool {
    mask & flag != 0
}

/// Helper function to recursively search for a node by name
fn find_node_by_name(parent: &Gd<Node>, name: &StringName) -> Option<Gd<Node>> {
    // Check if this node matches - compare StringName directly to avoid allocation
    if &parent.get_name() == name {
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
            find_node_by_name(&root.clone().upcast(), &StringName::from("SceneTreeWatcher"))
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
            find_node_by_name(
                &root.clone().upcast(),
                &StringName::from("OptimizedSceneTreeWatcher"),
            )
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

impl From<Vec<String>> for Groups {
    fn from(groups: Vec<String>) -> Self {
        Groups { groups }
    }
}

/// Resource for receiving scene tree messages from Godot.
/// Wrapped in Mutex to be Send+Sync, allowing it to be a regular Bevy Resource.
#[derive(Resource)]
pub struct SceneTreeMessageReader(pub Mutex<crossbeam_channel::Receiver<SceneTreeMessage>>);

impl SceneTreeMessageReader {
    pub fn new(receiver: crossbeam_channel::Receiver<SceneTreeMessage>) -> Self {
        Self(Mutex::new(receiver))
    }
}

fn write_scene_tree_messages(
    message_reader: Res<SceneTreeMessageReader>,
    mut message_writer: MessageWriter<SceneTreeMessage>,
) {
    let receiver = message_reader.0.lock();
    let messages: Vec<_> = receiver.try_iter().collect();
    message_writer.write_batch(messages);
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
    entities: &mut Query<(&GodotNodeHandle, Entity, Option<&ProtectedNodeEntity>)>,
    component_registry: &SceneTreeComponentRegistry,
    node_index: &mut NodeEntityIndex,
    godot: &mut GodotAccess,
) {
    // Make InstanceId to entity mapping for efficient random access (only needed during this function)
    let mut godot_entity_map = entities
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
            find_node_by_name(
                &scene_root.clone().upcast(),
                &StringName::from("CollisionWatcher"),
            )
        });

    // Collect collision bodies for batched signal connection
    // Tuple: (instance_id as i64, collision_mask as u8)
    let mut pending_collision_bodies: Vec<(i64, u8)> = Vec::new();

    for message in messages.into_iter() {
        trace!(target: "godot_scene_tree_messages", message = ?message);

        let SceneTreeMessage {
            node_id,
            message_type,
            node_type,
            node_name,
            parent_id: parent_id_from_gdscript,
            collision_mask,
            groups,
        } = message;
        let instance_id = node_id.instance_id();
        let node_handle = node_id;
        let entity_info = godot_entity_map.get(&instance_id).cloned();

        match message_type {
            SceneTreeMessageType::NodeAdded => {
                // Skip nodes that have been freed before we process them (can happen in tests)
                if !instance_id.lookup_validity() {
                    continue;
                }

                let mut new_entity_commands = if let Some((ent, _)) = entity_info {
                    commands.entity(ent)
                } else {
                    commands.spawn_empty()
                };

                let mut node_accessor = godot.node(node_handle);
                let mut node = node_accessor.get::<Node>();

                let node_name = node_name.unwrap_or_else(|| node.get_name().to_string());
                new_entity_commands
                    .insert(node_id)
                    .insert(Name::from(node_name));

                // Add node type marker components
                for class_name in get_inheritance_hierarchy(
                    node_type
                        // Fall back to getting node-type from node if not provided by message
                        .unwrap_or_else(|| node.get_class().to_string())
                        .as_str(),
                ) {
                    add_node_type_markers_from_string(
                        &mut new_entity_commands,
                        class_name.as_str(),
                    );
                }

                // Check if the node is a collision body (Area2D, Area3D, RigidBody2D, RigidBody3D, etc.)
                // These nodes typically have collision detection capabilities
                // Only connect if CollisionWatcher exists (i.e., GodotCollisionsPlugin was added)
                let collision_mask = collision_mask.or_else(|| {
                    collision_watcher
                        .as_ref()
                        .map(|_| collision_mask_from_node(&mut node))
                });

                // Check if the node is a collision body and collect for batched signal connection
                if collision_watcher.is_some()
                    && let Some(mask) = collision_mask
                {
                    let is_collision_body = collision_mask_has(mask, COLLISION_MASK_BODY_ENTERED)
                        || collision_mask_has(mask, COLLISION_MASK_AREA_ENTERED);

                    if is_collision_body {
                        debug!(target: "godot_scene_tree_collisions",
                               node_id = instance_id.to_string(),
                               "is collision body");

                        // Collect for batched connection
                        pending_collision_bodies.push((instance_id.to_i64(), mask));
                    }
                }
                // Use pre-analyzed groups from GDScript watcher if available, otherwise fallback to FFI
                if let Some(groups_vec) = groups {
                    new_entity_commands.insert(Groups::from(groups_vec));
                } else {
                    new_entity_commands.insert(Groups::from(&node));
                }

                // Add all components registered by plugins
                component_registry.add_to_entity(&mut new_entity_commands, &mut node_accessor);

                let new_entity = new_entity_commands.id();
                godot_entity_map.insert(
                    instance_id,
                    (new_entity, entity_info.and_then(|(_, protected)| protected)),
                );
                node_index.insert(instance_id, new_entity);

                // Try to add any registered bundles for this node type
                super::autosync::try_add_bundles_for_node(commands, new_entity, godot, node_handle);

                // Add GodotChildOf relationship to mirror Godot's scene tree hierarchy
                let parent_id = parent_id_from_gdscript
                    .or_else(|| node.get_parent().map(|parent| parent.instance_id()));
                if let Some(parent_id) = parent_id
                    && parent_id != scene_root.instance_id()
                {
                    if let Some((parent_entity, _)) = godot_entity_map.get(&parent_id) {
                        commands
                            .entity(new_entity)
                            .insert(super::relationship::GodotChildOf(*parent_entity));
                    } else {
                        warn!(target: "godot_scene_tree_messages",
                            "Parent entity with ID {} not found in godot_entity_map. This might indicate a missing or incorrect mapping. Path={}",
                            parent_id, node.get_path());
                    }
                }
            }
            SceneTreeMessageType::NodeRemoved => {
                if let Some((ent, prot_opt)) = entity_info {
                    // Check if node is being reparented vs truly removed
                    // During reparenting, the node is temporarily removed from old parent
                    // but still exists in the scene tree (has a parent)
                    // We need to try_get because the node handle might be invalid if freed
                    let is_reparenting = godot
                        .try_get::<Node>(node_handle)
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
                            _strip_godot_components(commands, ent);
                        }
                        godot_entity_map.remove(&instance_id);
                        node_index.remove(instance_id);
                    }
                } else {
                    // Entity was already despawned (common when using queue_free)
                    trace!(target: "godot_scene_tree_messages", "Entity for removed node was already despawned");
                }
            }
            SceneTreeMessageType::NodeRenamed => {
                if let Some((ent, _)) = entity_info {
                    let name = node_name
                        .unwrap_or_else(|| godot.get::<Node>(node_handle).get_name().to_string());
                    commands.entity(ent).insert(Name::from(name));
                } else {
                    trace!(target: "godot_scene_tree_messages", "Entity for renamed node was already despawned");
                }
            }
        }
    }

    // Batch connect collision signals if there are any pending
    if !pending_collision_bodies.is_empty()
        && let Some(ref collision_watcher) = collision_watcher
    {
        batch_connect_collision_signals(&scene_root, collision_watcher, &pending_collision_bodies);
    }
}

fn get_inheritance_hierarchy(class_name: &str) -> Vec<String> {
    let class_db = ClassDb::singleton();
    let mut hierarchy = Vec::new();

    // Initialize a local mutable variable to track the "current" class name
    let mut current_class = StringName::from(class_name);

    while !current_class.is_empty() {
        // Convert to String for the return vector
        hierarchy.push(current_class.to_string());

        // Update current_class to its parent
        current_class = class_db.get_parent_class(&current_class);
    }

    hierarchy
}

/// Batch connect collision signals using GDScript bulk operations.
/// Falls back to individual connections if bulk operations node is not available.
fn batch_connect_collision_signals(
    scene_root: &Gd<godot::classes::Window>,
    collision_watcher: &Gd<Node>,
    pending_bodies: &[(i64, u8)],
) {
    use godot::builtin::PackedInt64Array;

    // Try to find OptimizedBulkOperations node with the required method
    let bulk_ops = scene_root
        .get_node_or_null("BevyAppSingleton/OptimizedBulkOperations")
        .or_else(|| scene_root.get_node_or_null("/root/BevyAppSingleton/OptimizedBulkOperations"))
        .filter(|node| node.has_method("bulk_connect_collision_signals"));

    if let Some(mut bulk_ops) = bulk_ops {
        // Use batched GDScript call
        let instance_ids: Vec<i64> = pending_bodies.iter().map(|(id, _)| *id).collect();
        let collision_masks: Vec<i64> = pending_bodies
            .iter()
            .map(|(_, mask)| i64::from(*mask))
            .collect();

        let ids_packed = PackedInt64Array::from(instance_ids.as_slice());
        let masks_packed = PackedInt64Array::from(collision_masks.as_slice());

        bulk_ops.call(
            "bulk_connect_collision_signals",
            &[
                ids_packed.to_variant(),
                masks_packed.to_variant(),
                collision_watcher.to_variant(),
            ],
        );
    } else {
        // Fallback: connect signals individually
        for (instance_id, mask) in pending_bodies {
            let instance_id = InstanceId::from_i64(*instance_id);
            if !instance_id.lookup_validity() {
                continue;
            }

            // Get the node from instance ID
            let Some(mut node) = Gd::<Node>::try_from_instance_id(instance_id).ok() else {
                continue;
            };

            let node_clone = node.clone();

            if collision_mask_has(*mask, COLLISION_MASK_BODY_ENTERED) {
                node.connect(
                    BODY_ENTERED,
                    &collision_watcher.callable("collision_event").bind(&[
                        node_clone.to_variant(),
                        CollisionMessageType::Started.to_variant(),
                    ]),
                );
            }

            if collision_mask_has(*mask, COLLISION_MASK_BODY_EXITED) {
                node.connect(
                    BODY_EXITED,
                    &collision_watcher.callable("collision_event").bind(&[
                        node_clone.to_variant(),
                        CollisionMessageType::Ended.to_variant(),
                    ]),
                );
            }

            if collision_mask_has(*mask, COLLISION_MASK_AREA_ENTERED) {
                node.connect(
                    AREA_ENTERED,
                    &collision_watcher.callable("collision_event").bind(&[
                        node_clone.to_variant(),
                        CollisionMessageType::Started.to_variant(),
                    ]),
                );
            }

            if collision_mask_has(*mask, COLLISION_MASK_AREA_EXITED) {
                node.connect(
                    AREA_EXITED,
                    &collision_watcher.callable("collision_event").bind(&[
                        node_clone.to_variant(),
                        CollisionMessageType::Ended.to_variant(),
                    ]),
                );
            }
        }

        debug!(target: "godot_scene_tree_collisions",
               count = pending_bodies.len(),
               "Individually connected collision signals (bulk ops not available)");
    }
}

fn _strip_godot_components(commands: &mut Commands, ent: Entity) {
    let mut entity_commands = commands.entity(ent);

    entity_commands.remove::<GodotNodeHandle>();
    entity_commands.remove::<GodotScene>();
    entity_commands.remove::<Name>();
    entity_commands.remove::<Groups>();

    remove_comprehensive_node_type_markers(&mut entity_commands);
}

fn try_process_node_renamed_messages_fast_path(
    commands: &mut Commands,
    messages: &[SceneTreeMessage],
    node_index: &NodeEntityIndex,
    godot: &mut GodotAccess,
) -> bool {
    if !messages
        .iter()
        .all(|message| matches!(message.message_type, SceneTreeMessageType::NodeRenamed))
    {
        return false;
    }

    for message in messages {
        let node_handle = message.node_id;
        let Some(entity) = node_index.get(node_handle.instance_id()) else {
            trace!(target: "godot_scene_tree_messages", "Entity for renamed node was already despawned");
            continue;
        };

        let name = message
            .node_name
            .clone()
            .unwrap_or_else(|| godot.get::<Node>(node_handle).get_name().to_string());
        commands.entity(entity).insert(Name::from(name));
    }

    true
}

#[allow(clippy::too_many_arguments)]
fn read_scene_tree_messages(
    mut commands: Commands,
    mut scene_tree: SceneTreeRef,
    mut message_reader: MessageReader<SceneTreeMessage>,
    mut entities: Query<(&GodotNodeHandle, Entity, Option<&ProtectedNodeEntity>)>,
    component_registry: Res<SceneTreeComponentRegistry>,
    mut node_index: ResMut<NodeEntityIndex>,
    mut godot: GodotAccess,
) {
    let messages: Vec<_> = message_reader.read().cloned().collect();
    if messages.is_empty() {
        return;
    }

    if try_process_node_renamed_messages_fast_path(
        &mut commands,
        &messages,
        &node_index,
        &mut godot,
    ) {
        return;
    }

    create_scene_tree_entity(
        &mut commands,
        messages,
        &mut scene_tree,
        &mut entities,
        &component_registry,
        &mut node_index,
        &mut godot,
    );
}
