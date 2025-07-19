pub mod change_filter;
pub mod conversions;
pub mod custom_sync;
pub mod math;
pub mod plugin;
pub mod sync_systems;

// Re-export main components and types
pub use change_filter::TransformSyncMetadata;
pub use conversions::{IntoBevyTransform, IntoGodotTransform, IntoGodotTransform2D};
pub use custom_sync::{
    GodotTransformSyncPluginExt, add_transform_sync_systems_2d, add_transform_sync_systems_3d,
};
pub use plugin::GodotTransformSyncPlugin;

// Re-export math utilities for advanced users
pub use math::*;
