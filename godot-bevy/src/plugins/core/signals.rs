use bevy::{
    app::{App, First, Plugin},
    ecs::{
        event::{Event, EventWriter, event_update_system},
        schedule::IntoScheduleConfigs,
        system::NonSendMut,
    },
};
use godot::{classes::Node, meta::ToGodot, obj::InstanceId};

use crate::bridge::GodotNodeHandle;

use super::SceneTreeRef;

pub struct GodotSignalsPlugin;

impl Plugin for GodotSignalsPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(First, write_godot_signal_events.before(event_update_system))
            .add_event::<GodotSignal>();
    }
}

#[derive(Debug, Event)]
pub struct GodotSignal {
    pub name: String,
    pub origin: InstanceId,
    pub target: InstanceId,
}

#[doc(hidden)]
pub struct GodotSignalReader(pub std::sync::mpsc::Receiver<GodotSignal>);

fn write_godot_signal_events(
    events: NonSendMut<GodotSignalReader>,
    mut event_writer: EventWriter<GodotSignal>,
) {
    event_writer.write_batch(events.0.try_iter());
}

pub fn connect_godot_signal(
    node: &mut GodotNodeHandle,
    signal_name: &str,
    scene_tree: &mut SceneTreeRef,
) {
    let mut node = node.get::<Node>();
    let signal_watcher = scene_tree
        .get()
        .get_root()
        .unwrap()
        .get_node_as::<Node>("/root/BevyAppSingleton/SignalWatcher");

    let node_clone = node.clone();

    node.connect(
        signal_name,
        &signal_watcher.callable("event").bind(&[
            signal_watcher.to_variant(),
            node_clone.to_variant(),
            signal_name.to_variant(),
        ]),
    );
}
