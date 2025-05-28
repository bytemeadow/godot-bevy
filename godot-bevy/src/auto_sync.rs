use crate::bridge::GodotNodeHandle;
use bevy::prelude::*;

/// Trait for components that can automatically sync from Godot nodes
pub trait AutoSyncComponent: Component {
    type GodotType;

    fn auto_sync(&mut self, godot_node: &mut GodotNodeHandle);
}

/// Bundle that spawns a Godot scene and automatically syncs a component
#[derive(Bundle)]
pub struct GodotSceneWithComponent<T: Component + Default> {
    pub scene: crate::plugins::packed_scene::GodotScene,
    pub component: T,
}

impl<T: Component + Default> GodotSceneWithComponent<T> {
    pub fn new(path: &str) -> Self {
        Self {
            scene: crate::plugins::packed_scene::GodotScene::from_path(path),
            component: T::default(),
        }
    }

    pub fn from_resource(resource: crate::bridge::GodotResourceHandle) -> Self {
        Self {
            scene: crate::plugins::packed_scene::GodotScene::from_resource(resource),
            component: T::default(),
        }
    }
}
