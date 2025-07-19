use bevy::ecs::component::{Component, Tick};

/// Metadata component to track transform sync state for change detection
#[derive(Component, Default)]
pub struct TransformSyncMetadata {
    pub last_sync_tick: Option<Tick>,
}
