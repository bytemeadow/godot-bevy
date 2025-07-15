pub mod components;
pub mod conversions;
pub mod macros;
pub mod math;
pub mod plugin;
pub mod tests;

// Re-export main components and types
pub use components::{Transform2D, Transform2DMutGuard, Transform3D, TransformMutGuard};
pub use conversions::{IntoBevyTransform, IntoGodotTransform, IntoGodotTransform2D};
pub use macros::{add_transform_sync_systems, transform_sync_systems};
pub use plugin::GodotTransformSyncPlugin;

// Re-export math utilities for advanced users
pub use math::*;
