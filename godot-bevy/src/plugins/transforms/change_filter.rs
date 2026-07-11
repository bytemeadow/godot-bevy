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

/// Opt an entity out of Godot->Bevy transform reads. The Bevy->Godot write path is
/// unaffected, so the entity becomes Bevy-authoritative (one-way). Skipping the read
/// leaves the shadow stale, so Godot-side moves are ignored -- that is the intended
/// one-way ownership. Attach it directly, or add the node to the
/// [`NO_TRANSFORM_READ_GROUP`] Godot group to author the opt-out in-editor.
#[derive(Component, Default, Debug, Clone, Copy, Reflect)]
#[reflect(Component)]
pub struct DisableGodotTransformRead;

/// Godot group whose members are decorated with [`DisableGodotTransformRead`] at spawn
/// while `GodotTransformSyncPlugin` is active.
pub const NO_TRANSFORM_READ_GROUP: &str = "godot_bevy_no_transform_read";
