use bevy::app::{App, Plugin};
use bevy::asset::{AssetApp, AssetLoader, LoadContext, io::Reader};
use bevy::prelude::*;
use godot::classes::ResourceLoader;
use godot::obj::Gd;
use godot::prelude::Resource as GodotBaseResource;
use std::path::Path;
use thiserror::Error;

use crate::bridge::GodotResourceHandle;

/// Plugin that provides Bevy AssetLoader implementations for Godot resources.
/// This enables loading Godot resources through standard Bevy APIs while maintaining
/// compatibility with both development and exported builds.
pub struct GodotAssetsPlugin;

impl Plugin for GodotAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GodotResourceLoader>()
            .init_asset::<GodotResource>()
            .init_asset_loader::<GodotResourceAssetLoader>();
    }
}

/// Resource that provides a unified interface for loading Godot resources.
/// This works in both development and exported contexts.
#[derive(Resource, Default)]
pub struct GodotResourceLoader;

impl GodotResourceLoader {
    /// Load a Godot resource from the given path.
    /// Paths should be relative to the Godot project (e.g., "audio/sound.ogg").
    /// This will automatically add the "res://" prefix if not present.
    pub fn load(&self, path: &str) -> Option<Gd<GodotBaseResource>> {
        let godot_path = if path.starts_with("res://") || path.starts_with("user://") {
            path.to_string()
        } else {
            format!("res://{}", path)
        };

        let path_gstring = godot::builtin::GString::from(godot_path);
        ResourceLoader::singleton().load(&path_gstring)
    }

    /// Load a Godot resource and cast it to the specified type.
    /// Returns None if the resource doesn't exist or can't be cast to the target type.
    pub fn load_as<T>(&self, path: &str) -> Option<Gd<T>>
    where
        T: godot::obj::GodotClass + godot::obj::Inherits<GodotBaseResource>,
    {
        self.load(path)?.try_cast().ok()
    }

    /// Check if a resource exists at the given path.
    pub fn exists(&self, path: &str) -> bool {
        self.load(path).is_some()
    }
}

/// Possible errors that can be produced by Godot asset loaders
#[non_exhaustive]
#[derive(Debug, Error)]
pub enum GodotAssetLoaderError {
    /// Failed to load resource through Godot's ResourceLoader
    #[error("Failed to load Godot resource: {0}")]
    ResourceLoadFailed(String),
}

/// Universal wrapper for any Godot resource in Bevy's asset system
#[derive(Asset, TypePath, Debug, Clone)]
pub struct GodotResource {
    handle: GodotResourceHandle,
}

impl GodotResource {
    /// Get the raw Godot resource - you'll need to cast it to the specific type you need
    pub fn get(&mut self) -> Gd<GodotBaseResource> {
        self.handle.get()
    }

    /// Get the resource handle
    pub fn handle(&self) -> &GodotResourceHandle {
        &self.handle
    }

    /// Try to cast to a specific Godot resource type
    pub fn try_cast<T>(&mut self) -> Option<Gd<T>>
    where
        T: godot::obj::GodotClass + godot::obj::Inherits<GodotBaseResource>,
    {
        self.get().try_cast().ok()
    }
}

/// Universal AssetLoader for all Godot resources
#[derive(Default)]
pub struct GodotResourceAssetLoader;

impl AssetLoader for GodotResourceAssetLoader {
    type Asset = GodotResource;
    type Settings = ();
    type Error = GodotAssetLoaderError;

    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let path = load_context.path();
        let godot_path = ensure_godot_path(path);

        info!("Loading Godot resource: {}", godot_path);

        let mut resource_loader = ResourceLoader::singleton();
        let path_gstring = godot::builtin::GString::from(godot_path.clone());

        match resource_loader.load(&path_gstring) {
            Some(resource) => {
                let handle = GodotResourceHandle::new(resource);
                Ok(GodotResource { handle })
            }
            None => Err(GodotAssetLoaderError::ResourceLoadFailed(format!(
                "Failed to load Godot resource: {}",
                godot_path
            ))),
        }
    }

    fn extensions(&self) -> &[&str] {
        &[
            // Scenes
            "tscn", "scn", // Resources
            "res", "tres", // Images
            "jpg", "jpeg", "png", "webp", "svg", "bmp", "tga", "exr", "hdr", // Audio
            "wav", "mp3", "ogg", "aac", // Other common Godot assets
            "material", "mesh", "font", "theme", "shader",
        ]
    }
}

/// Ensures a path has the proper Godot resource prefix.
fn ensure_godot_path(path: &Path) -> String {
    let path_str = path.to_string_lossy();
    if path_str.starts_with("res://") || path_str.starts_with("user://") {
        path_str.to_string()
    } else {
        format!("res://{}", path_str)
    }
}
