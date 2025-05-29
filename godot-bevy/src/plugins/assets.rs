use bevy::app::{App, Plugin};
use bevy::asset::AssetPlugin;
use bevy::prelude::*;
use godot::classes::ProjectSettings;
use std::path::PathBuf;

/// Plugin that configures Bevy's asset server to use Godot's project directory as the asset root.
/// This ensures that asset paths are consistent whether running via Cargo or the Godot Editor.
pub struct GodotAssetsPlugin;

impl Plugin for GodotAssetsPlugin {
    fn build(&self, app: &mut App) {
        // Get the Godot project path
        let godot_project_path = get_godot_project_path();

        info!(
            "Setting Bevy asset root to Godot project path: {:?}",
            godot_project_path
        );

        // Configure the asset plugin with the Godot project path
        // This needs to be added early in the plugin chain
        let asset_plugin = AssetPlugin {
            file_path: godot_project_path.to_string_lossy().to_string(),
            ..Default::default()
        };

        app.add_plugins(asset_plugin);
    }
}

/// Gets the path to the Godot project directory.
/// This works whether running from Cargo or from the Godot Editor.
fn get_godot_project_path() -> PathBuf {
    // Try to get the project path from Godot's ProjectSettings
    let project_settings = ProjectSettings::singleton();

    // Get the project file path (e.g., "res://project.godot")
    let project_file_path = project_settings.globalize_path("res://");
    let project_path = PathBuf::from(project_file_path.to_string());

    // Convert to absolute path
    if project_path.is_absolute() {
        project_path
    } else {
        // Fallback: try to find the project directory relative to current working directory
        let current_dir = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
        current_dir
            .as_path()
            .ancestors()
            .find(|path| path.join("project.godot").exists())
            .unwrap_or_else(|| {
                warn!("Could not find Godot project directory, using current directory");
                current_dir.as_path()
            })
            .to_path_buf()
    }
}
