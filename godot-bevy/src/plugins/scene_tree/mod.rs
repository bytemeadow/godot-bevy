pub mod autosync;
pub mod node_type_checking;
pub mod plugin;
pub mod relationship;

// Re-export main components
pub use autosync::{AutoSyncBundleRegistry, BundleCreatorFn, register_all_autosync_bundles};
pub use plugin::{
    GodotSceneTreePlugin, Groups, NodeEntityIndex, ProtectedNodeEntity, SceneTreeConfig,
    SceneTreeMessage, SceneTreeMessageReader, SceneTreeMessageType, SceneTreeRef,
};
pub use relationship::{GodotChildOf, GodotChildren};
