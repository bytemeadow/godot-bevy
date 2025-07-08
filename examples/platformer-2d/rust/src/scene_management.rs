//! Simplified Scene Management System
//!
//! This module provides a simpler approach to reduce SceneTreeRef conflicts
//! while maintaining the parallelization benefits.

use bevy::prelude::*;
use godot_bevy::prelude::*;

/// Simple events for requesting scene tree operations
#[derive(Event, Debug)]
pub enum SceneOperationEvent {
    /// Reload the current scene
    ReloadCurrent,

    /// Change to a specific scene file
    ChangeToFile { path: String },

    /// Change to a loaded PackedScene
    ChangeToPacked { scene: Handle<GodotResource> },
}

/// Plugin that provides simplified scene management
pub struct SceneManagementPlugin;

impl Plugin for SceneManagementPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SceneOperationEvent>()
            .add_systems(Update, process_scene_operations);
    }
}

/// Central system that processes scene tree operations
///
/// This system reduces SceneTreeRef conflicts by handling scene operations centrally.
#[main_thread_system]
fn process_scene_operations(
    mut scene_tree: SceneTreeRef,
    mut operation_events: EventReader<SceneOperationEvent>,
    mut assets: ResMut<Assets<GodotResource>>,
) {
    for event in operation_events.read() {
        match event {
            SceneOperationEvent::ReloadCurrent => {
                info!("SceneManager: Reloading current scene");
                scene_tree.get().reload_current_scene();
            }

            SceneOperationEvent::ChangeToFile { path } => {
                info!("SceneManager: Changing to scene file: {}", path);
                scene_tree.get().change_scene_to_file(path);
            }

            SceneOperationEvent::ChangeToPacked { scene } => {
                if let Some(godot_resource) = assets.get_mut(scene) {
                    if let Some(packed_scene) =
                        godot_resource.try_cast::<godot::classes::PackedScene>()
                    {
                        info!("SceneManager: Changing to packed scene");
                        scene_tree.get().change_scene_to_packed(&packed_scene);
                    } else {
                        warn!("SceneManager: Resource is not a PackedScene");
                    }
                } else {
                    warn!("SceneManager: PackedScene asset not found or not loaded");
                }
            }
        }
    }
}

/// Convenience functions for common scene operations
impl SceneOperationEvent {
    /// Create a reload current scene event
    pub fn reload() -> Self {
        Self::ReloadCurrent
    }

    /// Create a change to file event
    pub fn change_to_file(path: impl Into<String>) -> Self {
        Self::ChangeToFile { path: path.into() }
    }

    /// Create a change to packed scene event
    pub fn change_to_packed(scene: Handle<GodotResource>) -> Self {
        Self::ChangeToPacked { scene }
    }
}
