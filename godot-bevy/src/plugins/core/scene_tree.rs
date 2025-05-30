use std::{collections::HashMap, marker::PhantomData};

use bevy::{
    app::{App, First, Plugin, PreStartup, Startup},
    ecs::{
        component::Component,
        entity::Entity,
        event::{Event, EventReader, EventWriter, event_update_system},
        name::Name,
        schedule::IntoScheduleConfigs,
        system::{Commands, NonSendMut, Query, SystemParam},
    },
    log::{debug, trace},
};
use godot::{
    builtin::GString,
    classes::{Engine, Node, Node2D, Node3D, SceneTree},
    meta::ToGodot,
    obj::{Gd, Inherits},
    prelude::GodotConvert,
};

use crate::{
    bridge::GodotNodeHandle,
    prelude::{Collisions, Transform2D, Transform3D},
};

use super::collisions::ALL_COLLISION_SIGNALS;

pub struct GodotSceneTreePlugin;

impl Plugin for GodotSceneTreePlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(PreStartup, initialize_scene_tree)
            .add_systems(Startup, connect_scene_tree)
            .add_systems(First, write_scene_tree_events.before(event_update_system))
            .add_systems(First, read_scene_tree_events.before(event_update_system))
            .add_event::<SceneTreeEvent>()
            .init_non_send_resource::<SceneTreeRefImpl>();
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
    mut entities: Query<(&mut GodotNodeHandle, Entity)>,
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

    create_scene_tree_entity(&mut commands, events, &mut scene_tree, &mut entities);
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
) {
    let mut ent_mapping = entities
        .iter()
        .map(|(reference, ent)| (reference.instance_id(), ent))
        .collect::<HashMap<_, _>>();
    let scene_root = scene_tree.get().get_root().unwrap();

    for event in events.into_iter() {
        trace!(target: "godot_scene_tree_events", event = ?event);

        let mut node = event.node.clone();
        let ent = ent_mapping.get(&node.instance_id()).cloned();

        match event.event_type {
            SceneTreeEventType::NodeAdded => {
                let mut ent = if let Some(ent) = ent {
                    commands.entity(ent)
                } else {
                    commands.spawn_empty()
                };

                ent.insert(GodotNodeHandle::clone(&node))
                    .insert(Name::from(node.get::<Node>().get_name().to_string()));

                if let Some(node3d) = node.try_get::<Node3D>() {
                    ent.insert(Transform3D::from(node3d.get_transform()));
                }

                if let Some(node2d) = node.try_get::<Node2D>() {
                    // TODO: validate this is as expected
                    let transform = node2d.get_transform();
                    ent.insert(Transform2D::from(transform));
                }

                let mut node = node.get::<Node>();

                // Check for any collision-related signals and connect them
                let has_collision_signals = ALL_COLLISION_SIGNALS
                    .iter()
                    .any(|&signal| node.has_signal(signal));

                if has_collision_signals {
                    debug!(target: "godot_scene_tree_collisions", 
                           node_id = node.instance_id().to_string(), 
                           "has collision signals");

                    let signal_watcher = scene_tree
                        .get()
                        .get_root()
                        .unwrap()
                        .get_node_as::<Node>("/root/BevyAppSingleton/SignalWatcher");

                    let node_clone = node.clone();

                    // Connect all available collision signals
                    for &signal_name in ALL_COLLISION_SIGNALS {
                        if node.has_signal(signal_name) {
                            node.connect(
                                signal_name,
                                &signal_watcher
                                    .callable("collision_event")
                                    .bind(&[node_clone.to_variant(), signal_name.to_variant()]),
                            );
                        }
                    }

                    ent.insert(Collisions::default());
                }

                ent.insert(Groups::from(&node));

                let ent = ent.id();
                ent_mapping.insert(node.instance_id(), ent);

                if node.instance_id() != scene_root.instance_id() {
                    let parent = node.get_parent().unwrap().instance_id();
                    commands
                        .entity(*ent_mapping.get(&parent).unwrap())
                        .add_children(&[ent]);
                }
            }
            SceneTreeEventType::NodeRemoved => {
                commands.entity(ent.unwrap()).despawn();
            }
            SceneTreeEventType::NodeRenamed => {
                commands
                    .entity(ent.unwrap())
                    .insert(Name::from(node.get::<Node>().get_name().to_string()));
            }
        }
    }
}

fn read_scene_tree_events(
    mut commands: Commands,
    mut scene_tree: SceneTreeRef,
    mut event_reader: EventReader<SceneTreeEvent>,
    mut entities: Query<(&mut GodotNodeHandle, Entity)>,
) {
    create_scene_tree_entity(
        &mut commands,
        event_reader.read().cloned(),
        &mut scene_tree,
        &mut entities,
    );
}
