pub mod autosync;
pub mod node_type_checking_generated;
pub mod plugin;

// Re-export main components
pub use autosync::{
    AutoSyncBundleRegistry, BundleCreatorFn, register_all_autosync_bundles,
    try_add_bundles_for_node,
};
pub use plugin::{
    GodotSceneTreePlugin, Groups, SceneTreeConfig, SceneTreeEvent, SceneTreeEventReader,
    SceneTreeEventType, SceneTreeRef,
};
