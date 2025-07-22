pub mod change_filter;
pub mod config;
pub mod conversions;
pub mod custom_sync;
pub mod math;
pub mod plugin;
pub mod sync_systems;

// Re-export main components and types
pub use change_filter::TransformSyncMetadata;
pub use config::{GodotTransformConfig, TransformSyncMode};
pub use conversions::{IntoBevyTransform, IntoGodotTransform, IntoGodotTransform2D};
pub use custom_sync::{GodotTransformSyncPluginExt, add_transform_sync_systems};
pub use plugin::GodotTransformSyncPlugin;

// Re-export math utilities for advanced users
pub use math::*;
