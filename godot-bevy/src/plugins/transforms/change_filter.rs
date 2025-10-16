use bevy::ecs::component::{Component, Tick};
use bevy::prelude::ReflectComponent;
use bevy::reflect::Reflect;

/// Metadata component to track transform sync state for change detection
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct TransformSyncMetadata {
    #[reflect(ignore)]
    pub last_sync_tick: Option<Tick>,
}
