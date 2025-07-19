use bevy::ecs::component::Tick;
use bevy::prelude::Component;

/// Component that stores the tick when we last synced from Godot
/// This allows us to detect changes that happened AFTER our sync
#[derive(Component)]
pub struct LastGodotSyncTick {
    pub tick: Tick,
}
