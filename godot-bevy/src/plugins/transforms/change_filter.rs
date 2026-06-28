use bevy_ecs::component::Component;
use bevy_ecs::prelude::ReflectComponent;
use bevy_reflect::Reflect;
use bevy_transform::components::Transform as BevyTransform;

/// Per-node transform sync state. The `shadow` is the last value exchanged with
/// Godot (seeded from the node at registration, then updated by both the read and
/// the write) -- it's the echo guard, comparing values rather than ticks so it
/// works read-before-write. `written_once` is set only by the write and gates the
/// first-write physics-interpolation reset.
#[derive(Component, Default, Reflect)]
#[reflect(Component)]
pub struct TransformSyncMetadata {
    #[reflect(ignore)]
    pub shadow: BevyTransform,
    pub written_once: bool,
}
