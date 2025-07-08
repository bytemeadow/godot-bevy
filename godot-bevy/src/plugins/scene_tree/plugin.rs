use crate::interop::node_markers::*;
use crate::plugins::transforms::{Transform2D, Transform3D};
use crate::prelude::main_thread_system;
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
    log::{debug, trace},
    prelude::Resource,
};
use godot::{
    builtin::GString,
    classes::{
        AnimatedSprite2D, AnimatedSprite3D, AnimationPlayer, AnimationTree, Area2D, Area3D,
        AudioStreamPlayer, AudioStreamPlayer2D, AudioStreamPlayer3D, Button, Camera2D, Camera3D,
        CanvasItem, CharacterBody2D, CharacterBody3D, CollisionPolygon2D, CollisionPolygon3D,
        CollisionShape2D, CollisionShape3D, Control, DirectionalLight3D, Engine, Label, LineEdit,
        MeshInstance2D, MeshInstance3D, Node, Node2D, Node3D, Panel, Path2D, Path3D, PathFollow2D,
        PathFollow3D, RigidBody2D, RigidBody3D, SceneTree, SpotLight3D, Sprite2D, Sprite3D,
        StaticBody2D, StaticBody3D, TextEdit, Timer,
    },
    meta::ToGodot,
    obj::{Gd, Inherits},
    prelude::GodotConvert,
};
use std::collections::HashMap;
use std::marker::PhantomData;

/// Unified scene tree plugin that provides:
/// - SceneTreeRef for accessing the Godot scene tree
/// - Scene tree events (NodeAdded, NodeRemoved, NodeRenamed)
/// - Automatic entity creation and mirroring for scene tree nodes
///
/// This plugin is always included in the core plugins and provides
/// complete scene tree integration out of the box.
pub struct GodotSceneTreePlugin {
    /// Whether to automatically add Transform components to entities
    pub add_transforms: bool,
}

impl Default for GodotSceneTreePlugin {
    fn default() -> Self {
        Self {
            add_transforms: true,
        }
    }
}

/// Configuration resource for scene tree behavior
#[derive(Resource)]
pub(crate) struct SceneTreeConfig {
    add_transforms: bool,
}

impl Plugin for GodotSceneTreePlugin {
    fn build(&self, app: &mut App) {
        // Auto-register all discovered AutoSyncBundle plugins
        super::autosync::register_all_autosync_bundles(app);

        app.init_non_send_resource::<SceneTreeRefImpl>()
            .insert_resource(SceneTreeConfig {
                add_transforms: self.add_transforms,
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
    mut entities: Query<(&mut GodotNodeHandle, Entity)>,
    config: Res<SceneTreeConfig>,
) {
    fn traverse(node: Gd<Node>, events: &mut Vec<SceneTreeEvent>) {
        events.push(SceneTreeEvent {
            node: GodotNodeHandle::from_instance_id(node.instance_id()),
            event_type: SceneTreeEventType::NodeAdded,
        });

        for child in node.get_children().iter_shared() {
            traverse(child, events);
        }
    }

    let root = scene_tree.get().get_root().unwrap();
    let mut events = vec![];
    traverse(root.upcast(), &mut events);

    create_scene_tree_entity(
        &mut commands,
        events,
        &mut scene_tree,
        &mut entities,
        &config,
    );
}

#[derive(Debug, Clone, Event)]
pub struct SceneTreeEvent {
    pub node: GodotNodeHandle,
    pub event_type: SceneTreeEventType,
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

    let watcher = scene_tree_gd
        .get_root()
        .unwrap()
        .get_node_as::<Node>("/root/BevyAppSingleton/SceneTreeWatcher");

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

/// Adds appropriate marker components to an entity based on the Godot node type
fn add_node_type_markers(
    entity_commands: &mut bevy::ecs::system::EntityCommands,
    node: &mut GodotNodeHandle,
) {
    // Try each node type and add the corresponding marker component
    // We check more specific types first, then fall back to more general ones

    // Visual nodes
    if node.try_get::<Sprite2D>().is_some() {
        entity_commands.insert(Sprite2DMarker);
    }
    if node.try_get::<Sprite3D>().is_some() {
        entity_commands.insert(Sprite3DMarker);
    }
    if node.try_get::<AnimatedSprite2D>().is_some() {
        entity_commands.insert(AnimatedSprite2DMarker);
    }
    if node.try_get::<AnimatedSprite3D>().is_some() {
        entity_commands.insert(AnimatedSprite3DMarker);
    }
    if node.try_get::<MeshInstance2D>().is_some() {
        entity_commands.insert(MeshInstance2DMarker);
    }
    if node.try_get::<MeshInstance3D>().is_some() {
        entity_commands.insert(MeshInstance3DMarker);
    }

    // Physics bodies
    if node.try_get::<CharacterBody2D>().is_some() {
        entity_commands.insert(CharacterBody2DMarker);
    }
    if node.try_get::<CharacterBody3D>().is_some() {
        entity_commands.insert(CharacterBody3DMarker);
    }
    if node.try_get::<RigidBody2D>().is_some() {
        entity_commands.insert(RigidBody2DMarker);
    }
    if node.try_get::<RigidBody3D>().is_some() {
        entity_commands.insert(RigidBody3DMarker);
    }
    if node.try_get::<StaticBody2D>().is_some() {
        entity_commands.insert(StaticBody2DMarker);
    }
    if node.try_get::<StaticBody3D>().is_some() {
        entity_commands.insert(StaticBody3DMarker);
    }

    // Areas
    if node.try_get::<Area2D>().is_some() {
        entity_commands.insert(Area2DMarker);
    }
    if node.try_get::<Area3D>().is_some() {
        entity_commands.insert(Area3DMarker);
    }

    // Collision shapes
    if node.try_get::<CollisionShape2D>().is_some() {
        entity_commands.insert(CollisionShape2DMarker);
    }
    if node.try_get::<CollisionShape3D>().is_some() {
        entity_commands.insert(CollisionShape3DMarker);
    }
    if node.try_get::<CollisionPolygon2D>().is_some() {
        entity_commands.insert(CollisionPolygon2DMarker);
    }
    if node.try_get::<CollisionPolygon3D>().is_some() {
        entity_commands.insert(CollisionPolygon3DMarker);
    }

    // Audio nodes
    if node.try_get::<AudioStreamPlayer>().is_some() {
        entity_commands.insert(AudioStreamPlayerMarker);
    }
    if node.try_get::<AudioStreamPlayer2D>().is_some() {
        entity_commands.insert(AudioStreamPlayer2DMarker);
    }
    if node.try_get::<AudioStreamPlayer3D>().is_some() {
        entity_commands.insert(AudioStreamPlayer3DMarker);
    }

    // UI nodes
    if node.try_get::<Label>().is_some() {
        entity_commands.insert(LabelMarker);
    }
    if node.try_get::<Button>().is_some() {
        entity_commands.insert(ButtonMarker);
    }
    if node.try_get::<LineEdit>().is_some() {
        entity_commands.insert(LineEditMarker);
    }
    if node.try_get::<TextEdit>().is_some() {
        entity_commands.insert(TextEditMarker);
    }
    if node.try_get::<Panel>().is_some() {
        entity_commands.insert(PanelMarker);
    }

    // Camera nodes
    if node.try_get::<Camera2D>().is_some() {
        entity_commands.insert(Camera2DMarker);
    }
    if node.try_get::<Camera3D>().is_some() {
        entity_commands.insert(Camera3DMarker);
    }

    // Light nodes
    if node.try_get::<DirectionalLight3D>().is_some() {
        entity_commands.insert(DirectionalLight3DMarker);
    }
    if node.try_get::<SpotLight3D>().is_some() {
        entity_commands.insert(SpotLight3DMarker);
    }

    // Animation nodes
    if node.try_get::<AnimationPlayer>().is_some() {
        entity_commands.insert(AnimationPlayerMarker);
    }
    if node.try_get::<AnimationTree>().is_some() {
        entity_commands.insert(AnimationTreeMarker);
    }

    // Timer
    if node.try_get::<Timer>().is_some() {
        entity_commands.insert(TimerMarker);
    }

    // Path nodes
    if node.try_get::<Path2D>().is_some() {
        entity_commands.insert(Path2DMarker);
    }
    if node.try_get::<Path3D>().is_some() {
        entity_commands.insert(Path3DMarker);
    }
    if node.try_get::<PathFollow2D>().is_some() {
        entity_commands.insert(PathFollow2DMarker);
    }
    if node.try_get::<PathFollow3D>().is_some() {
        entity_commands.insert(PathFollow3DMarker);
    }

    // Base node types (checked last to ensure more specific types take precedence)
    if node.try_get::<Control>().is_some() {
        entity_commands.insert(ControlMarker);
    }
    if node.try_get::<CanvasItem>().is_some() {
        entity_commands.insert(CanvasItemMarker);
    }
    if node.try_get::<Node3D>().is_some() {
        entity_commands.insert(Node3DMarker);
    }
    if node.try_get::<Node2D>().is_some() {
        entity_commands.insert(Node2DMarker);
    }

    // All nodes inherit from Node, so add this last
    entity_commands.insert(NodeMarker);
}

#[doc(hidden)]
pub struct SceneTreeEventReader(pub std::sync::mpsc::Receiver<SceneTreeEvent>);

fn write_scene_tree_events(
    event_reader: NonSendMut<SceneTreeEventReader>,
    mut event_writer: EventWriter<SceneTreeEvent>,
) {
    event_writer.write_batch(event_reader.0.try_iter());
}

fn create_scene_tree_entity(
    commands: &mut Commands,
    events: impl IntoIterator<Item = SceneTreeEvent>,
    scene_tree: &mut SceneTreeRef,
    entities: &mut Query<(&mut GodotNodeHandle, Entity)>,
    config: &SceneTreeConfig,
) {
    let mut ent_mapping = entities
        .iter()
        .map(|(reference, ent)| (reference.instance_id(), ent))
        .collect::<HashMap<_, _>>();
    let scene_root = scene_tree.get().get_root().unwrap();
    let collision_watcher = scene_tree
        .get()
        .get_root()
        .unwrap()
        .get_node_as::<Node>("/root/BevyAppSingleton/CollisionWatcher");

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

                let mut ent = if let Some(ent) = ent {
                    commands.entity(ent)
                } else {
                    commands.spawn_empty()
                };

                ent.insert(GodotNodeHandle::clone(&node))
                    .insert(Name::from(node.get::<Node>().get_name().to_string()));

                // Add node type marker components
                add_node_type_markers(&mut ent, &mut node);

                // Add transform components if configured to do so
                if config.add_transforms {
                    if let Some(node3d) = node.try_get::<Node3D>() {
                        ent.insert(Transform3D::from(node3d.get_transform()));
                    }

                    if let Some(node2d) = node.try_get::<Node2D>() {
                        let transform = node2d.get_transform();
                        ent.insert(Transform2D::from(transform));
                    }
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

                let ent = ent.id();
                ent_mapping.insert(node.instance_id(), ent);

                // Try to add any registered bundles for this node type
                super::autosync::try_add_bundles_for_node(commands, ent, &event.node);

                if node.instance_id() != scene_root.instance_id() {
                    if let Some(parent) = node.get_parent() {
                        let parent_id = parent.instance_id();
                        if let Some(&parent_entity) = ent_mapping.get(&parent_id) {
                            commands.entity(parent_entity).add_children(&[ent]);
                        } else {
                            bevy::log::warn!(target: "godot_scene_tree_events",
                                "Parent entity with ID {} not found in ent_mapping. This might indicate a missing or incorrect mapping.",
                                parent_id);
                        }
                    }
                }
            }
            SceneTreeEventType::NodeRemoved => {
                if let Some(ent) = ent {
                    commands.entity(ent).despawn();
                } else {
                    // Entity was already despawned (common when using queue_free)
                    trace!(target: "godot_scene_tree_events", "Entity for removed node was already despawned");
                }
            }
            SceneTreeEventType::NodeRenamed => {
                if let Some(ent) = ent {
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

#[main_thread_system]
fn read_scene_tree_events(
    mut commands: Commands,
    mut scene_tree: SceneTreeRef,
    mut event_reader: EventReader<SceneTreeEvent>,
    mut entities: Query<(&mut GodotNodeHandle, Entity)>,
    config: Res<SceneTreeConfig>,
) {
    create_scene_tree_entity(
        &mut commands,
        event_reader.read().cloned(),
        &mut scene_tree,
        &mut entities,
        &config,
    );
}
