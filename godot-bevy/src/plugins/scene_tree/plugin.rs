use super::node_type_checking_generated::{
    add_comprehensive_node_type_markers, add_node_type_markers_from_string,
    remove_comprehensive_node_type_markers,
};
use crate::plugins::core::SceneTreeComponentRegistry;
use crate::prelude::{GodotScene, main_thread_system};
use crate::{
    interop::GodotNodeHandle,
    plugins::collisions::{
        AREA_ENTERED, AREA_EXITED, BODY_ENTERED, BODY_EXITED, COLLISION_START_SIGNALS,
        CollisionEventType, Collisions,
    },
};
use bevy::{
    app::{App, First, Plugin, PreStartup},
    ecs::{
        component::Component,
        entity::Entity,
        event::{Event, EventReader, EventWriter, event_update_system},
        name::Name,
        schedule::IntoScheduleConfigs,
        system::{Commands, NonSendMut, Query, Res, SystemParam},
    },
    prelude::Resource,
};
use godot::{
    builtin::GString,
    classes::{Engine, Node, SceneTree},
    meta::ToGodot,
    obj::{Gd, Inherits},
    prelude::GodotConvert,
};
use std::collections::HashMap;
use std::marker::PhantomData;
use tracing::{debug, trace, warn};

/// Unified scene tree plugin that provides:
/// - SceneTreeRef for accessing the Godot scene tree
/// - Scene tree events (NodeAdded, NodeRemoved, NodeRenamed)
/// - Automatic entity creation and mirroring for scene tree nodes
///
/// This plugin is always included in the core plugins and provides
/// complete scene tree integration out of the box.
pub struct GodotSceneTreePlugin {
    /// When true, adds a parent child entity relationship in ECS
    /// that mimics Godot's parent child node relationship.
    /// NOTE: You should disable this if you want to use Avian Physics,
    /// as it is incompatible, i.e., Avian Physics has its own notions
    /// for what parent/child entity relatonships mean
    pub add_child_relationship: bool,
}

impl Default for GodotSceneTreePlugin {
    fn default() -> Self {
        Self {
            add_child_relationship: true,
        }
    }
}

/// Configuration resource for scene tree behavior
#[derive(Resource)]
pub struct SceneTreeConfig {
    /// When true, adds a parent child entity relationship in ECS
    /// that mimics Godot's parent child node relationship.
    /// NOTE: You should disable this if you want to use Avian Physics,
    /// as it is incompatible, i.e., Avian Physics has its own notions
    /// for what parent/child entity relatonships mean
    pub add_child_relationship: bool,
}

impl Plugin for GodotSceneTreePlugin {
    fn build(&self, app: &mut App) {
        // Auto-register all discovered AutoSyncBundle plugins
        super::autosync::register_all_autosync_bundles(app);

        app.init_non_send_resource::<SceneTreeRefImpl>()
            .insert_resource(SceneTreeConfig {
                add_child_relationship: self.add_child_relationship,
            })
            .add_event::<SceneTreeEvent>()
            .add_systems(
                PreStartup,
                (connect_scene_tree, initialize_scene_tree).chain(),
            )
            .add_systems(
                First,
                (
                    write_scene_tree_events.before(event_update_system),
                    read_scene_tree_events.before(event_update_system),
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
    config: Res<SceneTreeConfig>,
    component_registry: Res<SceneTreeComponentRegistry>,
) {
    let root = scene_tree.get().get_root().unwrap();

    // Check if we have the optimized GDScript watcher for type pre-analysis
    let optimized_watcher = root
        .try_get_node_as::<Node>("/root/BevyAppSingleton/OptimizedSceneTreeWatcher")
        .or_else(|| root.try_get_node_as::<Node>("BevyAppSingleton/OptimizedSceneTreeWatcher"));

    let events = if let Some(mut watcher) = optimized_watcher {
        // Use optimized GDScript watcher to analyze the initial tree with type information
        tracing::info!("Using optimized initial tree analysis with type pre-analysis");

        let analysis_result = watcher.call("analyze_initial_tree", &[]);
        let result_dict = analysis_result.to::<godot::builtin::Dictionary>();
        let instance_ids = result_dict
            .get("instance_ids")
            .unwrap()
            .to::<godot::builtin::PackedInt64Array>();
        let node_types = result_dict
            .get("node_types")
            .unwrap()
            .to::<godot::builtin::PackedStringArray>();

        let mut events = Vec::new();
        let len = instance_ids.len().min(node_types.len());
        for i in 0..len {
            if let (Some(id), Some(type_gstring)) = (instance_ids.get(i), node_types.get(i)) {
                let type_str = type_gstring.to_string();

                events.push(SceneTreeEvent {
                    node: GodotNodeHandle::from_instance_id(godot::prelude::InstanceId::from_i64(
                        id,
                    )),
                    event_type: SceneTreeEventType::NodeAdded,
                    node_type: Some(type_str),
                });
            }
        }

        events
    } else {
        // Use fallback traversal without type optimization
        tracing::info!("Using fallback initial tree analysis (no type optimization)");
        traverse_fallback(root.upcast())
    };

    create_scene_tree_entity(
        &mut commands,
        events,
        &mut scene_tree,
        &mut entities,
        &config,
        &component_registry,
    );
}

fn traverse_fallback(node: Gd<Node>) -> Vec<SceneTreeEvent> {
    fn traverse_recursive(node: Gd<Node>, events: &mut Vec<SceneTreeEvent>) {
        events.push(SceneTreeEvent {
            node: GodotNodeHandle::from_instance_id(node.instance_id()),
            event_type: SceneTreeEventType::NodeAdded,
            node_type: None, // No type optimization available
        });

        for child in node.get_children().iter_shared() {
            traverse_recursive(child, events);
        }
    }

    let mut events = Vec::new();
    traverse_recursive(node, &mut events);
    events
}

#[derive(Debug, Clone, Event)]
pub struct SceneTreeEvent {
    pub node: GodotNodeHandle,
    pub event_type: SceneTreeEventType,
    pub node_type: Option<String>, // Pre-analyzed node type from GDScript watcher
}

#[derive(Copy, Clone, Debug, GodotConvert)]
#[godot(via = GString)]
pub enum SceneTreeEventType {
    NodeAdded,
    NodeRemoved,
    NodeRenamed,
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
        .unwrap_or_else(|| {
            panic!("SceneTreeWatcher not found at expected paths. Make sure it exists at /root/BevyAppSingleton/SceneTreeWatcher or BevyAppSingleton/SceneTreeWatcher");
        });

    // Check if we have the optimized GDScript watcher
    let optimized_watcher = root
        .try_get_node_as::<Node>("/root/BevyAppSingleton/OptimizedSceneTreeWatcher")
        .or_else(|| root.try_get_node_as::<Node>("BevyAppSingleton/OptimizedSceneTreeWatcher"));

    if optimized_watcher.is_some() {
        // The optimized GDScript watcher handles scene tree connections and forwards
        // pre-analyzed events to the Rust watcher (which has the MPSC sender)
        // No need to connect here - it connects automatically in its _ready()
        tracing::info!("Using optimized GDScript scene tree watcher with type pre-analysis");
    } else {
        // Fallback to direct connection without type optimization
        tracing::info!("Using fallback scene tree connection (no type optimization)");

        scene_tree_gd.connect(
            "node_added",
            &watcher
                .callable("scene_tree_event")
                .bind(&[SceneTreeEventType::NodeAdded.to_variant()]),
        );

        scene_tree_gd.connect(
            "node_removed",
            &watcher
                .callable("scene_tree_event")
                .bind(&[SceneTreeEventType::NodeRemoved.to_variant()]),
        );

        scene_tree_gd.connect(
            "node_renamed",
            &watcher
                .callable("scene_tree_event")
                .bind(&[SceneTreeEventType::NodeRenamed.to_variant()]),
        );
    }
}

#[derive(Component, Debug)]
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
pub struct SceneTreeEventReader(pub std::sync::mpsc::Receiver<SceneTreeEvent>);

fn write_scene_tree_events(
    event_reader: NonSendMut<SceneTreeEventReader>,
    mut event_writer: EventWriter<SceneTreeEvent>,
) {
    event_writer.write_batch(event_reader.0.try_iter());
}

/// Marks an entity so it is not despawned when its corresponding Godot Node is freed, breaking
/// the usual 1-to-1 lifetime between them. This allows game logic to keep running on entities
/// that have no Node, such as simulating off-screen factory machines or NPCs in inactive scenes.
/// A Godot Node can be re-associated later by adding a `GodotScene` component to the **entity.**
#[derive(Component)]
pub struct ProtectedNodeEntity;

fn create_scene_tree_entity(
    commands: &mut Commands,
    events: impl IntoIterator<Item = SceneTreeEvent>,
    scene_tree: &mut SceneTreeRef,
    entities: &mut Query<(&mut GodotNodeHandle, Entity, Option<&ProtectedNodeEntity>)>,
    config: &SceneTreeConfig,
    component_registry: &SceneTreeComponentRegistry,
) {
    let mut ent_mapping = entities
        .iter()
        .map(|(reference, ent, protected)| (reference.instance_id(), (ent, protected)))
        .collect::<HashMap<_, _>>();
    let scene_root = scene_tree.get().get_root().unwrap();
    let collision_watcher = scene_root
        .try_get_node_as::<Node>("/root/BevyAppSingleton/CollisionWatcher")
        .or_else(|| {
            // Try without the full path for test environments
            scene_root.try_get_node_as::<Node>("BevyAppSingleton/CollisionWatcher")
        })
        .unwrap_or_else(|| {
            panic!("CollisionWatcher not found at expected paths. Make sure it exists at /root/BevyAppSingleton/CollisionWatcher or BevyAppSingleton/CollisionWatcher");
        });

    for event in events.into_iter() {
        trace!(target: "godot_scene_tree_events", event = ?event);

        let mut node = event.node.clone();
        let ent = ent_mapping.get(&node.instance_id()).cloned();

        match event.event_type {
            SceneTreeEventType::NodeAdded => {
                // Skip nodes that have been freed before we process them (can happen in tests)
                if !node.instance_id().lookup_validity() {
                    continue;
                }

                let mut ent = if let Some((ent, _)) = ent {
                    commands.entity(ent)
                } else {
                    commands.spawn_empty()
                };

                ent.insert(GodotNodeHandle::clone(&node))
                    .insert(Name::from(node.get::<Node>().get_name().to_string()));

                // Add node type marker components - use optimized version if available
                if let Some(ref node_type_str) = event.node_type {
                    // Use pre-analyzed type from GDScript watcher (much faster)
                    add_node_type_markers_from_string(&mut ent, node_type_str);
                } else {
                    // Fallback to comprehensive analysis with FFI calls
                    add_comprehensive_node_type_markers(&mut ent, &mut node);
                }

                let mut node = node.get::<Node>();

                // Check if the node is a collision body (Area2D, Area3D, RigidBody2D, RigidBody3D, etc.)
                // These nodes typically have collision detection capabilities
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
                                CollisionEventType::Started.to_variant(),
                            ]),
                        );
                    }

                    if node.has_signal(BODY_EXITED) {
                        node.connect(
                            BODY_EXITED,
                            &collision_watcher.callable("collision_event").bind(&[
                                node_clone.to_variant(),
                                CollisionEventType::Ended.to_variant(),
                            ]),
                        );
                    }

                    if node.has_signal(AREA_ENTERED) {
                        node.connect(
                            AREA_ENTERED,
                            &collision_watcher.callable("collision_event").bind(&[
                                node_clone.to_variant(),
                                CollisionEventType::Started.to_variant(),
                            ]),
                        );
                    }

                    if node.has_signal(AREA_EXITED) {
                        node.connect(
                            AREA_EXITED,
                            &collision_watcher.callable("collision_event").bind(&[
                                node_clone.to_variant(),
                                CollisionEventType::Ended.to_variant(),
                            ]),
                        );
                    }

                    // Add Collisions component to track collision state
                    ent.insert(Collisions::default());
                }

                ent.insert(Groups::from(&node));

                // Add all components registered by plugins
                component_registry.add_to_entity(&mut ent, &event.node);

                let ent = ent.id();
                ent_mapping.insert(node.instance_id(), (ent, None));

                // Try to add any registered bundles for this node type
                super::autosync::try_add_bundles_for_node(commands, ent, &event.node);

                if config.add_child_relationship
                    && node.instance_id() != scene_root.instance_id()
                    && let Some(parent) = node.get_parent()
                {
                    let parent_id = parent.instance_id();
                    if let Some((parent_entity, _)) = ent_mapping.get(&parent_id) {
                        commands.entity(*parent_entity).add_children(&[ent]);
                    } else {
                        warn!(target: "godot_scene_tree_events",
                            "Parent entity with ID {} not found in ent_mapping. This might indicate a missing or incorrect mapping.",
                            parent_id);
                    }
                }
            }
            SceneTreeEventType::NodeRemoved => {
                if let Some((ent, prot_opt)) = ent {
                    let protected = prot_opt.is_some();
                    if !protected {
                        commands.entity(ent).despawn();
                    } else {
                        _strip_godot_components(commands, ent);
                    }
                    ent_mapping.remove(&node.instance_id());
                } else {
                    // Entity was already despawned (common when using queue_free)
                    trace!(target: "godot_scene_tree_events", "Entity for removed node was already despawned");
                }
            }
            SceneTreeEventType::NodeRenamed => {
                if let Some((ent, _)) = ent {
                    commands
                        .entity(ent)
                        .insert(Name::from(node.get::<Node>().get_name().to_string()));
                } else {
                    trace!(target: "godot_scene_tree_events", "Entity for renamed node was already despawned");
                }
            }
        }
    }
}

fn _strip_godot_components(commands: &mut Commands, ent: Entity) {
    let mut entity_commands = commands.entity(ent);
    // Remove GodotNodeHandle components
    entity_commands.remove::<GodotNodeHandle>();

    // Remove all GodotScene components
    entity_commands.remove::<GodotScene>();

    // Remove automatic markers
    entity_commands.remove::<Name>();
    entity_commands.remove::<Groups>();
    // Create a dummy handle since we're removing components anyway
    let mut dummy_handle =
        GodotNodeHandle::from_instance_id(godot::prelude::InstanceId::from_i64(0));
    remove_comprehensive_node_type_markers(&mut entity_commands, &mut dummy_handle);
}

#[main_thread_system]
fn read_scene_tree_events(
    mut commands: Commands,
    mut scene_tree: SceneTreeRef,
    mut event_reader: EventReader<SceneTreeEvent>,
    mut entities: Query<(&mut GodotNodeHandle, Entity, Option<&ProtectedNodeEntity>)>,
    config: Res<SceneTreeConfig>,
    component_registry: Res<SceneTreeComponentRegistry>,
) {
    create_scene_tree_entity(
        &mut commands,
        event_reader.read().cloned(),
        &mut scene_tree,
        &mut entities,
        &config,
        &component_registry,
    );
}
