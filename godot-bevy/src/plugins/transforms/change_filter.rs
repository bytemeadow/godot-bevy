use bevy_ecs::component::{Component, Tick};
use bevy_ecs::prelude::ReflectComponent;
use bevy_reflect::Reflect;

/// Metadata component to track transform sync state for change detection
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct TransformSyncMetadata {
    #[reflect(ignore)]
    pub last_sync_tick: Option<Tick>,
}
