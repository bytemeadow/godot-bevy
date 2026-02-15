use crate::interop::{GodotAccess, GodotNodeHandle};
use crate::plugins::core::PhysicsUpdate;
use bevy_app::{App, Plugin};
use bevy_ecs::component::Component;
use bevy_ecs::message::{Message, MessageWriter};
use bevy_ecs::prelude::Query;
use bevy_ecs::query::With;
use bevy_ecs::schedule::{IntoScheduleConfigs, SystemSet};
use godot::classes::Node;
use std::marker::PhantomData;

/// Marker set for Godot mailbox drain systems.
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum GodotMailboxSet {
    /// Reads script-side mailbox state from Godot nodes and emits Bevy messages.
    Drain,
}

/// Trait implemented by typed mailbox messages that can be drained from a Godot node.
///
/// A message type decides how to read and clear its own script-side mailbox fields.
/// This allows bridging legacy script writes into typed ECS messages without coupling
/// games to stringly-typed ad-hoc systems.
pub trait GodotMailboxMessage: Message + Send + Sync + Sized + 'static {
    /// Reads this message from a Godot node (if pending), and clears pending data as needed.
    fn drain_from_node(node: &mut Node, source: GodotNodeHandle) -> Option<Self>;
}

/// Generic plugin that drains mailbox messages from Godot nodes with marker `Marker`.
///
/// This plugin runs in [`PhysicsUpdate`] and writes messages of type `T`.
pub struct GodotMailboxPlugin<T, Marker>
where
    T: GodotMailboxMessage,
    Marker: Component,
{
    _phantom: PhantomData<(T, Marker)>,
}

impl<T, Marker> Default for GodotMailboxPlugin<T, Marker>
where
    T: GodotMailboxMessage,
    Marker: Component,
{
    fn default() -> Self {
        Self {
            _phantom: PhantomData,
        }
    }
}

impl<T, Marker> Plugin for GodotMailboxPlugin<T, Marker>
where
    T: GodotMailboxMessage,
    Marker: Component,
{
    fn build(&self, app: &mut App) {
        app.add_message::<T>().add_systems(
            PhysicsUpdate,
            drain_mailbox_messages::<T, Marker>.in_set(GodotMailboxSet::Drain),
        );
    }
}

fn drain_mailbox_messages<T, Marker>(
    nodes: Query<&GodotNodeHandle, With<Marker>>,
    mut writer: MessageWriter<T>,
    mut godot: GodotAccess,
) where
    T: GodotMailboxMessage,
    Marker: Component,
{
    for handle in nodes.iter() {
        let Some(mut node) = godot.try_get::<Node>(*handle) else {
            continue;
        };

        if let Some(message) = T::drain_from_node(&mut node, *handle) {
            writer.write(message);
        }
    }
}
