use bevy::prelude::*;

/// Transform synchronization modes
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TransformSyncMode {
    /// No transform syncing - use direct Godot physics (move_and_slide, etc.)
    /// Best for: Platformers, physics-heavy games
    Disabled,
    /// One-way sync: ECS → Godot only
    /// Best for: Pure ECS games, simple movement
    #[default]
    OneWay,
    /// Two-way sync: ECS ↔ Godot
    /// Best for: Hybrid apps migrating from GDScript to ECS
    TwoWay,
}

/// Configuration resource for transform syncing behavior
#[derive(Default, Resource, Debug, Clone)]
pub struct GodotTransformConfig {
    pub sync_mode: TransformSyncMode,
}

impl GodotTransformConfig {
    /// Disable all transform syncing - use direct Godot physics instead
    pub fn disabled() -> Self {
        Self {
            sync_mode: TransformSyncMode::Disabled,
        }
    }

    /// Enable one-way sync (ECS → Godot) - default behavior
    pub fn one_way() -> Self {
        Self {
            sync_mode: TransformSyncMode::OneWay,
        }
    }

    /// Enable two-way sync (ECS ↔ Godot) for hybrid apps
    pub fn two_way() -> Self {
        Self {
            sync_mode: TransformSyncMode::TwoWay,
        }
    }
}
