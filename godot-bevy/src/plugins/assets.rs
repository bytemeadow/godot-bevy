use bevy::app::{App, Plugin};
use bevy::asset::{AssetApp, AssetLoader, LoadContext, io::Reader};
use bevy::prelude::*;
use godot::classes::{AudioStream, PackedScene, ResourceLoader, Texture2D};
use godot::obj::Gd;
use godot::prelude::Resource as GodotResource;
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
            .init_asset::<GodotPackedScene>()
            .init_asset::<GodotAudioStream>()
            .init_asset::<GodotTexture2D>()
            .init_asset_loader::<GodotPackedSceneLoader>()
            .init_asset_loader::<GodotAudioStreamLoader>()
            .init_asset_loader::<GodotTexture2DLoader>();
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
    pub fn load(&self, path: &str) -> Option<Gd<GodotResource>> {
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
        T: godot::obj::GodotClass + godot::obj::Inherits<GodotResource>,
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
    /// Resource exists but is not the expected type
    #[error("Resource is not the expected type: {0}")]
    InvalidResourceType(String),
}

/// Wrapper for Godot PackedScene resources in Bevy's asset system
#[derive(Asset, TypePath, Debug, Clone)]
pub struct GodotPackedScene {
    handle: GodotResourceHandle,
}

impl GodotPackedScene {
    pub fn get(&mut self) -> Gd<PackedScene> {
        self.handle.get().try_cast::<PackedScene>().unwrap()
    }

    pub fn handle(&self) -> &GodotResourceHandle {
        &self.handle
    }
}

/// Wrapper for Godot AudioStream resources in Bevy's asset system
#[derive(Asset, TypePath, Debug, Clone)]
pub struct GodotAudioStream {
    handle: GodotResourceHandle,
}

impl GodotAudioStream {
    pub fn get(&mut self) -> Gd<AudioStream> {
        self.handle.get().try_cast::<AudioStream>().unwrap()
    }

    pub fn handle(&self) -> &GodotResourceHandle {
        &self.handle
    }
}

/// Wrapper for Godot Texture2D resources in Bevy's asset system
#[derive(Asset, TypePath, Debug, Clone)]
pub struct GodotTexture2D {
    handle: GodotResourceHandle,
}

impl GodotTexture2D {
    pub fn get(&mut self) -> Gd<Texture2D> {
        self.handle.get().try_cast::<Texture2D>().unwrap()
    }

    pub fn handle(&self) -> &GodotResourceHandle {
        &self.handle
    }
}

/// AssetLoader for Godot PackedScene files (.tscn, .scn)
#[derive(Default)]
pub struct GodotPackedSceneLoader;

impl AssetLoader for GodotPackedSceneLoader {
    type Asset = GodotPackedScene;
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

        info!("Loading Godot PackedScene: {}", godot_path);

        let mut resource_loader = ResourceLoader::singleton();
        let path_gstring = godot::builtin::GString::from(godot_path.clone());

        match resource_loader.load(&path_gstring) {
            Some(resource) => {
                // Verify it's a PackedScene by trying to cast a clone
                let resource_clone = resource.clone();
                match resource_clone.try_cast::<PackedScene>() {
                    Ok(_) => {
                        let handle = GodotResourceHandle::new(resource);
                        Ok(GodotPackedScene { handle })
                    }
                    Err(_) => Err(GodotAssetLoaderError::InvalidResourceType(format!(
                        "Resource is not a PackedScene: {}",
                        godot_path
                    ))),
                }
            }
            None => Err(GodotAssetLoaderError::ResourceLoadFailed(format!(
                "Failed to load PackedScene: {}",
                godot_path
            ))),
        }
    }

    fn extensions(&self) -> &[&str] {
        &["tscn", "scn"]
    }
}

/// AssetLoader for Godot AudioStream files (.ogg, .wav, .mp3, etc.)
#[derive(Default)]
pub struct GodotAudioStreamLoader;

impl AssetLoader for GodotAudioStreamLoader {
    type Asset = GodotAudioStream;
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

        info!("Loading Godot AudioStream: {}", godot_path);

        let mut resource_loader = ResourceLoader::singleton();
        let path_gstring = godot::builtin::GString::from(godot_path.clone());

        match resource_loader.load(&path_gstring) {
            Some(resource) => {
                // Verify it's an AudioStream by trying to cast a clone
                let resource_clone = resource.clone();
                match resource_clone.try_cast::<AudioStream>() {
                    Ok(_) => {
                        let handle = GodotResourceHandle::new(resource);
                        Ok(GodotAudioStream { handle })
                    }
                    Err(_) => Err(GodotAssetLoaderError::InvalidResourceType(format!(
                        "Resource is not an AudioStream: {}",
                        godot_path
                    ))),
                }
            }
            None => Err(GodotAssetLoaderError::ResourceLoadFailed(format!(
                "Failed to load AudioStream: {}",
                godot_path
            ))),
        }
    }

    fn extensions(&self) -> &[&str] {
        &["ogg", "wav", "mp3", "aac"]
    }
}

/// AssetLoader for Godot Texture2D files (.png, .jpg, etc.)
#[derive(Default)]
pub struct GodotTexture2DLoader;

impl AssetLoader for GodotTexture2DLoader {
    type Asset = GodotTexture2D;
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

        info!("Loading Godot Texture2D: {}", godot_path);

        let mut resource_loader = ResourceLoader::singleton();
        let path_gstring = godot::builtin::GString::from(godot_path.clone());

        match resource_loader.load(&path_gstring) {
            Some(resource) => {
                // Verify it's a Texture2D by trying to cast a clone
                let resource_clone = resource.clone();
                match resource_clone.try_cast::<Texture2D>() {
                    Ok(_) => {
                        let handle = GodotResourceHandle::new(resource);
                        Ok(GodotTexture2D { handle })
                    }
                    Err(_) => Err(GodotAssetLoaderError::InvalidResourceType(format!(
                        "Resource is not a Texture2D: {}",
                        godot_path
                    ))),
                }
            }
            None => Err(GodotAssetLoaderError::ResourceLoadFailed(format!(
                "Failed to load Texture2D: {}",
                godot_path
            ))),
        }
    }

    fn extensions(&self) -> &[&str] {
        &[
            "png", "jpg", "jpeg", "webp", "svg", "bmp", "tga", "exr", "hdr",
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
