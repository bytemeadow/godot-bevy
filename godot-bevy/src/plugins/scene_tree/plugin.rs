use super::node_type_checking::{
    add_node_type_markers_from_string, remove_comprehensive_node_type_markers,
};
use crate::plugins::core::SceneTreeComponentRegistry;
use crate::prelude::GodotScene;
use crate::watchers::scene_tree_watcher::is_excluded_from_mirror;
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
    lifecycle::HookContext,
    message::{Message, MessageReader, MessageWriter, message_update_system},
    prelude::{Name, ReflectComponent, ReflectResource, Resource},
    query::Has,
    schedule::IntoScheduleConfigs,
    system::{Commands, NonSendMut, Query, Res, ResMut, SystemParam},
    world::DeferredWorld,
};
use bevy_reflect::Reflect;
use bevy_time::{Time, TimeSystems, Virtual};
use godot::classes::ClassDb;
use godot::tools::try_get_autoload_by_name;
use godot::{
    builtin::{GString, StringName},
    classes::{Area2D, Area3D, Engine, Node, RigidBody2D, RigidBody3D, SceneTree},
    meta::ToGodot,
    obj::{Gd, Inherits, InstanceId, Singleton},
    prelude::GodotConvert,
};
use parking_lot::Mutex;
use std::cell::RefCell;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::rc::Rc;
use tracing::{debug, trace, warn};

/// A resource that maintains an O(1) lookup from Godot `InstanceId` to Bevy `Entity`.
///
/// Kept complete for every entity with a `GodotNodeHandle` — scene-tree,
/// packed-scene, or user-spawned — via component hooks. Use it to find the Bevy
/// entity for a Godot node (collision handling, signal routing, etc.).
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
        super::autosync::register_all_required_components(app);

        app.init_non_send::<SceneTreeRefImpl>()
            .init_resource::<NodeEntityIndex>()
            .init_resource::<PauseBridge>()
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
                    mirror_tree_pause_to_virtual.before(TimeSystems),
                ),
            );

        // Hooks keep NodeEntityIndex complete in O(1) per change, so message
        // processing resolves entities through it instead of an O(world) scan.
        app.world_mut()
            .register_component_hooks::<GodotNodeHandle>()
            .on_insert(on_godot_node_handle_insert)
            // 0.19 renamed the replace hook to `on_discard` (fires when the component
            // is about to be dropped via replace/remove); same semantics as before.
            .on_discard(on_godot_node_handle_replace);
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

/// Edge state for `mirror_tree_pause_to_virtual`: the last-seen `SceneTree.paused` (so it
/// acts only on an edge) and whether the mirror -- not the user -- applied the current
/// virtual pause (so a falling edge resumes only the mirror's own pause).
#[derive(Resource, Default)]
struct PauseBridge {
    last_tree_paused: bool,
    paused_by_mirror: bool,
}

/// Mirror `SceneTree.paused` onto `Time<Virtual>`, one-way and edge-triggered: a rising edge
/// pauses virtual time and a falling edge unpauses it -- but only the pause the mirror itself
/// applied. A user's own `Time<Virtual>::pause()` (e.g. a cutscene) is never clobbered, even
/// across a tree pause/unpause cycle. Runs in `First` before `TimeSystems` (and before the
/// fixed driver reads `is_paused()`) so the pause takes effect the same frame. `SceneTreeRef`
/// pins it to the main thread and hands back the cached `Gd<SceneTree>`.
fn mirror_tree_pause_to_virtual(
    mut scene_tree: SceneTreeRef,
    virt: Option<ResMut<Time<Virtual>>>,
    mut bridge: ResMut<PauseBridge>,
) {
    // Standalone/benchmark apps compose this plugin without TimePlugin; with no
    // Time<Virtual> there is nothing to mirror.
    let Some(mut virt) = virt else {
        return;
    };
    let tree_paused = scene_tree.get().is_paused();
    if tree_paused == bridge.last_tree_paused {
        return;
    }
    if tree_paused {
        // Don't stack onto a user's existing pause -- only claim the pause we apply.
        if !virt.is_paused() {
            virt.pause();
            bridge.paused_by_mirror = true;
        }
    } else if bridge.paused_by_mirror {
        virt.unpause();
        bridge.paused_by_mirror = false;
    }
    bridge.last_tree_paused = tree_paused;
}

fn initialize_scene_tree(
    mut commands: Commands,
    mut scene_tree: SceneTreeRef,
    mut entities: Query<(
        &GodotNodeHandle,
        Entity,
        Option<&ProtectedNodeEntity>,
        Has<SceneTreeDecorated>,
    )>,
    component_registry: Res<SceneTreeComponentRegistry>,
    mut node_index: ResMut<NodeEntityIndex>,
    message_reader: Res<SceneTreeMessageReader>,
    mut godot: GodotAccess,
) {
    let root = scene_tree.get().get_root().unwrap();

    // Check if we have the optimized GDScript watcher for type pre-analysis
    let optimized_watcher = get_bevy_app_child("OptimizedSceneTreeWatcher");

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

    // The snapshot above created and decorated an entity for every node currently in the
    // tree. Anything the watcher queued between connecting (the addon's _ready, during
    // do_initialize) and now is either a node the snapshot also captured or one no longer
    // present, so it carries nothing new. Discard the backlog so First doesn't re-walk it;
    // events that arrive after this drain land in the channel normally.
    let _ = message_reader.0.lock().try_iter().count();
}

fn traverse_fallback(node: Gd<Node>) -> Vec<SceneTreeMessage> {
    fn traverse_recursive(node: Gd<Node>, messages: &mut Vec<SceneTreeMessage>) {
        // Excluded subtree: skip this node and (recursion is below) all descendants.
        if node.has_meta("_bevy_exclude") {
            return;
        }
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

const BEVY_APP_AUTOLOAD_NAME: &str = "BevyAppSingleton";

/// Gets a child node of the BevyAppSingleton autoload by name.
/// Falls back to tree search if the autoload isn't registered.
fn get_bevy_app_child(child_name: &str) -> Option<Gd<Node>> {
    // Autoload lookup is cached after first call
    if let Ok(bevy_app) = try_get_autoload_by_name::<Node>(BEVY_APP_AUTOLOAD_NAME) {
        return bevy_app.try_get_node_as::<Node>(child_name);
    }

    let scene_tree = Engine::singleton()
        .get_main_loop()
        .and_then(|ml| ml.try_cast::<SceneTree>().ok())?;
    let root = scene_tree.get_root()?;
    find_node_by_name(&root.upcast(), &StringName::from(child_name))
}

fn connect_scene_tree(mut scene_tree: SceneTreeRef) {
    let mut scene_tree_gd = scene_tree.get();

    let watcher = get_bevy_app_child("SceneTreeWatcher")
        .unwrap_or_else(|| {
            panic!("SceneTreeWatcher not found as child of BevyAppSingleton autoload or anywhere in the scene tree.");
        });

    // Check if we have the optimized GDScript watcher
    let optimized_watcher = get_bevy_app_child("OptimizedSceneTreeWatcher");

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

/// Index a `GodotNodeHandle` when it is added or changed, from any source. A hook
/// keeps `NodeEntityIndex` complete in O(1) per change — an `Added`-filtered
/// system would instead scan the whole archetype every frame.
fn on_godot_node_handle_insert(mut world: DeferredWorld, ctx: HookContext) {
    let Some(handle) = world.get::<GodotNodeHandle>(ctx.entity).copied() else {
        return;
    };
    world
        .resource_mut::<NodeEntityIndex>()
        .insert(handle.instance_id(), ctx.entity);
}

/// Fires before a `GodotNodeHandle` is overwritten or removed, while the old
/// handle is still readable, so its stale index entry can be evicted.
fn on_godot_node_handle_replace(mut world: DeferredWorld, ctx: HookContext) {
    let Some(handle) = world.get::<GodotNodeHandle>(ctx.entity).copied() else {
        return;
    };
    world
        .resource_mut::<NodeEntityIndex>()
        .remove(handle.instance_id());
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

/// Inserted once when the scene-tree plugin fully decorates an entity. A later
/// `NodeAdded` for the same node (a reparent, or a startup-backlog duplicate) refreshes
/// Name/GodotChildOf but must not re-run the registry, autosync, markers, Groups, or
/// collision connects -- that would reset authored ECS state.
#[derive(Component)]
struct SceneTreeDecorated;

fn create_scene_tree_entity(
    commands: &mut Commands,
    messages: impl IntoIterator<Item = SceneTreeMessage>,
    scene_tree: &mut SceneTreeRef,
    entities: &mut Query<(
        &GodotNodeHandle,
        Entity,
        Option<&ProtectedNodeEntity>,
        Has<SceneTreeDecorated>,
    )>,
    component_registry: &SceneTreeComponentRegistry,
    node_index: &mut NodeEntityIndex,
    godot: &mut GodotAccess,
) {
    // Resolve entities via the complete NodeEntityIndex (in-loop inserts below
    // plus the GodotNodeHandle hooks), avoiding an O(world) scan per batch.
    let scene_root = scene_tree.get().get_root().unwrap();

    // CollisionWatcher is optional - only required if GodotCollisionsPlugin is added
    let collision_watcher = get_bevy_app_child("CollisionWatcher");

    // Collect collision bodies for batched signal connection.
    let mut pending_collision_bodies: Vec<(Gd<Node>, u8, ColliderKind)> = Vec::new();

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
        let existing_entity = node_index.get(instance_id);

        match message_type {
            SceneTreeMessageType::NodeAdded => {
                // Skip nodes that have been freed before we process them (can happen in tests)
                if !instance_id.lookup_validity() {
                    continue;
                }

                // A prior batch already decorated this entity, so it is queryable now;
                // skip re-decorating (see SceneTreeDecorated).
                let already_decorated = existing_entity
                    .and_then(|ent| entities.get(ent).ok())
                    .map(|(_, _, _, decorated)| decorated)
                    .unwrap_or(false);

                let mut new_entity_commands = if let Some(ent) = existing_entity {
                    commands.entity(ent)
                } else {
                    commands.spawn_empty()
                };

                let mut node_accessor = godot.node(node_handle);
                let mut node = node_accessor.get::<Node>();

                let node_name = node_name.unwrap_or_else(|| node.get_name().to_string());

                let new_entity = if already_decorated {
                    new_entity_commands.insert((node_id, Name::from(node_name)));
                    new_entity_commands.id()
                } else {
                    // Compute the class hierarchy once; reused for markers and autosync.
                    let class_hierarchy = get_inheritance_hierarchy(
                        node_type
                            // Fall back to getting node-type from node if not provided by message
                            .unwrap_or_else(|| node.get_class().to_string())
                            .as_str(),
                    );
                    // The first matching arm inserts the whole ancestor-marker chain in one
                    // move, so stop -- continuing would redundantly re-insert those markers. An
                    // unknown leaf (e.g. a GDExtension class) returns false and falls through to
                    // its first native ancestor.
                    for class_name in class_hierarchy.iter() {
                        if add_node_type_markers_from_string(
                            &mut new_entity_commands,
                            class_name.as_str(),
                        ) {
                            break;
                        }
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
                        let is_collision_body =
                            collision_mask_has(mask, COLLISION_MASK_BODY_ENTERED)
                                || collision_mask_has(mask, COLLISION_MASK_AREA_ENTERED);

                        if is_collision_body {
                            debug!(target: "godot_scene_tree_collisions",
                                   node_id = instance_id.to_string(),
                                   "is collision body");

                            let kind = if class_hierarchy.iter().any(|c| c.as_str() == "Area2D") {
                                ColliderKind::Area2D
                            } else if class_hierarchy.iter().any(|c| c.as_str() == "Area3D") {
                                ColliderKind::Area3D
                            } else {
                                ColliderKind::Other
                            };
                            pending_collision_bodies.push((node.clone(), mask, kind));
                        }
                    }
                    // Groups come pre-analyzed from the GDScript watcher, or fall back to FFI.
                    let groups = match groups {
                        Some(groups_vec) => Groups::from(groups_vec),
                        None => Groups::from(&node),
                    };
                    new_entity_commands.insert((
                        node_id,
                        Name::from(node_name),
                        groups,
                        SceneTreeDecorated,
                    ));

                    // Add all components registered by plugins
                    component_registry.add_to_entity(&mut new_entity_commands, &mut node_accessor);

                    let new_entity = new_entity_commands.id();
                    node_index.insert(instance_id, new_entity);

                    // Try to add any registered bundles for this node type
                    super::autosync::try_add_bundles_for_node(
                        commands,
                        new_entity,
                        godot,
                        node_handle,
                        class_hierarchy.as_slice(),
                    );

                    new_entity
                };

                // Reconcile GodotChildOf with the node's current parent (the point of a reparent
                // NodeAdded): link to a mirrored parent, else drop any stale edge.
                let parent_id = parent_id_from_gdscript
                    .or_else(|| node.get_parent().map(|parent| parent.instance_id()))
                    .filter(|parent_id| *parent_id != scene_root.instance_id());
                match parent_id.and_then(|parent_id| node_index.get(parent_id)) {
                    Some(parent_entity) => {
                        commands
                            .entity(new_entity)
                            .insert(super::relationship::GodotChildOf(parent_entity));
                    }
                    None => {
                        commands
                            .entity(new_entity)
                            .remove::<super::relationship::GodotChildOf>();
                        if let Some(parent_id) = parent_id {
                            warn!(target: "godot_scene_tree_messages",
                                "Parent entity with ID {} not found in NodeEntityIndex. This might indicate a missing or incorrect mapping. Path={}",
                                parent_id, node.get_path());
                        }
                    }
                }
            }
            SceneTreeMessageType::NodeRemoved => {
                if let Some(ent) = existing_entity {
                    // Check if node is being reparented vs truly removed
                    // During reparenting, the node is temporarily removed from old parent
                    // but still exists in the scene tree (has a parent)
                    // We need to try_get because the node handle might be invalid if freed
                    let is_reparenting = godot
                        .try_get::<Node>(node_handle)
                        .map(|godot_node| godot_node.get_parent().is_some())
                        .unwrap_or(false);

                    if is_reparenting {
                        // A node reparented under an excluded subtree has its NodeAdded dropped
                        // by the watcher, so preserving the entity here would strand it with a
                        // stale parent and keep it syncing. Reconcile against exclusion: tear it
                        // down instead. Otherwise it moved within the mirrored tree -- preserve it.
                        let into_excluded = godot
                            .try_get::<Node>(node_handle)
                            .map(|n| is_excluded_from_mirror(&n))
                            .unwrap_or(false);
                        if into_excluded {
                            commands.entity(ent).despawn();
                            node_index.remove(instance_id);
                        } else {
                            trace!(target: "godot_scene_tree_events",
                                "Node is being reparented, preserving entity");
                        }
                    } else {
                        // Truly removed. Read protected from the world; same-batch
                        // spawns aren't queryable yet but are never protected.
                        let protected = entities
                            .get(ent)
                            .map(|(_, _, prot, _)| prot.is_some())
                            .unwrap_or(false);
                        if !protected {
                            commands.entity(ent).despawn();
                        } else {
                            _strip_godot_components(commands, ent);
                        }
                        node_index.remove(instance_id);
                    }
                } else {
                    // Entity was already despawned (common when using queue_free)
                    trace!(target: "godot_scene_tree_messages", "Entity for removed node was already despawned");
                }
            }
            SceneTreeMessageType::NodeRenamed => {
                if let Some(ent) = existing_entity {
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
        batch_connect_collision_signals(collision_watcher, &pending_collision_bodies);
    }
}

/// Inheritance chain for a Godot class (class then ancestors). Memoized per
/// class name: the chain is static and scenes have few distinct classes, so we
/// walk `ClassDb` once per class, not per node; `Rc` avoids reallocating on hits.
/// Main-thread only (holds `GodotAccess`), so the thread-local cache is lock-free.
fn get_inheritance_hierarchy(class_name: &str) -> Rc<Vec<String>> {
    thread_local! {
        static CACHE: RefCell<HashMap<String, Rc<Vec<String>>>> = RefCell::new(HashMap::new());
    }

    CACHE.with(|cache| {
        if let Some(hierarchy) = cache.borrow().get(class_name) {
            return hierarchy.clone();
        }

        let class_db = ClassDb::singleton();
        let mut hierarchy = Vec::new();
        let mut current_class = StringName::from(class_name);
        while !current_class.is_empty() {
            hierarchy.push(current_class.to_string());
            current_class = class_db.get_parent_class(&current_class);
        }

        let hierarchy = Rc::new(hierarchy);
        cache
            .borrow_mut()
            .insert(class_name.to_string(), hierarchy.clone());
        hierarchy
    })
}

/// Which collider a pending body is, derived once from the class hierarchy at decoration
/// so the seed dispatches without re-probing the type via casts. Only Areas are seeded.
#[derive(Clone, Copy)]
enum ColliderKind {
    Area2D,
    Area3D,
    Other,
}

/// Batch connect collision signals using GDScript bulk operations.
/// Falls back to individual connections if bulk operations node is not available.
fn batch_connect_collision_signals(
    collision_watcher: &Gd<Node>,
    pending_bodies: &[(Gd<Node>, u8, ColliderKind)],
) {
    use godot::builtin::PackedInt64Array;

    let bulk_ops = get_bevy_app_child("OptimizedBulkOperations")
        .filter(|node| node.has_method("bulk_connect_collision_signals"));

    if let Some(mut bulk_ops) = bulk_ops {
        // Use batched GDScript call
        let instance_ids: Vec<i64> = pending_bodies
            .iter()
            .map(|(node, _, _)| node.instance_id().to_i64())
            .collect();
        let collision_masks: Vec<i64> = pending_bodies
            .iter()
            .map(|(_, mask, _)| i64::from(*mask))
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
        for (node, mask, _) in pending_bodies {
            if !node.is_instance_valid() {
                continue;
            }
            let mut node = node.clone();
            let node_variant = node.to_variant();

            // Only two distinct bound callables -- Started (enter) and Ended (exit) --
            // reused across the body/area signal pair.
            let started = collision_watcher.callable("collision_event").bind(&[
                node_variant.clone(),
                CollisionMessageType::Started.to_variant(),
            ]);
            let ended = collision_watcher
                .callable("collision_event")
                .bind(&[node_variant, CollisionMessageType::Ended.to_variant()]);

            // A duplicate connect (the same node pushed twice in one batch) makes Godot
            // print an error. One is_connected on the first enter signal proves the node
            // is already wired -- skip all four rather than eat four error prints.
            let enter_sig = if collision_mask_has(*mask, COLLISION_MASK_BODY_ENTERED) {
                BODY_ENTERED
            } else {
                AREA_ENTERED
            };
            if node.is_connected(enter_sig, &started) {
                continue;
            }

            if collision_mask_has(*mask, COLLISION_MASK_BODY_ENTERED) {
                node.connect(BODY_ENTERED, &started);
            }
            if collision_mask_has(*mask, COLLISION_MASK_BODY_EXITED) {
                node.connect(BODY_EXITED, &ended);
            }
            if collision_mask_has(*mask, COLLISION_MASK_AREA_ENTERED) {
                node.connect(AREA_ENTERED, &started);
            }
            if collision_mask_has(*mask, COLLISION_MASK_AREA_EXITED) {
                node.connect(AREA_EXITED, &ended);
            }
        }

        debug!(target: "godot_scene_tree_collisions",
               count = pending_bodies.len(),
               "Individually connected collision signals (bulk ops not available)");
    }

    // Seed pre-existing overlaps (Area only) and warn on contact-monitor-less
    // RigidBodies. Runs after connect for both the bulk and fallback paths, one-time
    // per new collider at scene-tree add.
    for (node, _mask, kind) in pending_bodies {
        if !node.is_instance_valid() {
            continue;
        }

        // try_cast still gates, so a stale kind fails safe to "no seed".
        match kind {
            ColliderKind::Area2D => {
                if let Ok(area) = node.clone().try_cast::<Area2D>() {
                    seed_overlaps_2d(&area, collision_watcher);
                }
            }
            ColliderKind::Area3D => {
                if let Ok(area) = node.clone().try_cast::<Area3D>() {
                    seed_overlaps_3d(&area, collision_watcher);
                }
            }
            ColliderKind::Other => {
                if is_rigid_body_without_contact_monitor(node) {
                    warn_dead_contact_monitor(node);
                }
            }
        }
    }
}

/// Route each of an Area2D's current overlaps through the collision watcher as a
/// synthetic `Started`, recovering a spawn-into-overlap whose real enter signal fired
/// (to zero connections) before this connect. `get_overlapping_*` reads the area's
/// `body_map`, populated at `flush_queries` regardless of signal connection.
/// `add_collision` dedups, so a real enter that also fired produces no duplicate.
fn seed_overlaps_2d(area: &Gd<Area2D>, watcher: &Gd<Node>) {
    let mut watcher = watcher.clone();
    // has_* is a cheap bool; skip the Array-marshalling get_* when there's nothing to seed.
    if area.has_overlapping_bodies() {
        for body in area.get_overlapping_bodies().iter_shared() {
            watcher.call(
                "collision_event",
                &[
                    body.to_variant(),
                    area.to_variant(),
                    CollisionMessageType::Started.to_variant(),
                ],
            );
        }
    }
    if area.has_overlapping_areas() {
        for other in area.get_overlapping_areas().iter_shared() {
            watcher.call(
                "collision_event",
                &[
                    other.to_variant(),
                    area.to_variant(),
                    CollisionMessageType::Started.to_variant(),
                ],
            );
        }
    }
}

fn seed_overlaps_3d(area: &Gd<Area3D>, watcher: &Gd<Node>) {
    let mut watcher = watcher.clone();
    if area.has_overlapping_bodies() {
        for body in area.get_overlapping_bodies().iter_shared() {
            watcher.call(
                "collision_event",
                &[
                    body.to_variant(),
                    area.to_variant(),
                    CollisionMessageType::Started.to_variant(),
                ],
            );
        }
    }
    if area.has_overlapping_areas() {
        for other in area.get_overlapping_areas().iter_shared() {
            watcher.call(
                "collision_event",
                &[
                    other.to_variant(),
                    area.to_variant(),
                    CollisionMessageType::Started.to_variant(),
                ],
            );
        }
    }
}

/// A RigidBody with `contact_monitor` disabled never fires the `body_entered/exited`
/// signals we connect (Godot's `_body_inout` returns early on a null contact monitor).
fn is_rigid_body_without_contact_monitor(node: &Gd<Node>) -> bool {
    if let Ok(body) = node.clone().try_cast::<RigidBody2D>() {
        !body.is_contact_monitor_enabled()
    } else if let Ok(body) = node.clone().try_cast::<RigidBody3D>() {
        !body.is_contact_monitor_enabled()
    } else {
        false
    }
}

fn warn_dead_contact_monitor(node: &Gd<Node>) {
    warn!(target: "godot_scene_tree_collisions",
        node = %node.get_path(),
        "connected RigidBody collision signals but contact_monitor is disabled; \
         Godot will never fire them. Set contact_monitor = true and \
         max_contacts_reported > 0 on this node to receive collision events.");
}

fn _strip_godot_components(commands: &mut Commands, ent: Entity) {
    let mut entity_commands = commands.entity(ent);

    entity_commands.remove::<GodotNodeHandle>();
    entity_commands.remove::<GodotScene>();
    entity_commands.remove::<Name>();
    entity_commands.remove::<Groups>();
    entity_commands.remove::<SceneTreeDecorated>();

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
    mut entities: Query<(
        &GodotNodeHandle,
        Entity,
        Option<&ProtectedNodeEntity>,
        Has<SceneTreeDecorated>,
    )>,
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
