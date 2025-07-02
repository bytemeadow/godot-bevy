use bevy::app::{App, Plugin};
use bevy::asset::{
    AssetApp, AssetLoader, LoadContext,
    io::{AssetReader, AssetReaderError, PathStream, Reader, VecReader},
};
use bevy::prelude::*;
use futures_lite::stream;
use godot::classes::ResourceLoader;
use godot::classes::resource_loader::ThreadLoadStatus;
use godot::obj::Gd;
use godot::prelude::Resource as GodotBaseResource;
use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex};
use thiserror::Error;

use crate::interop::GodotResourceHandle;

/// Plugin that provides Bevy AssetLoader implementations for Godot resources.
/// This enables loading Godot resources through standard Bevy APIs while maintaining
/// compatibility with both development and exported builds.
///
/// **Note**: Path verification bypass is handled automatically by `GodotCorePlugin`,
/// so Bevy's `AssetServer` can load Godot resources from .pck files and other virtual paths
/// without additional configuration. The `GodotResourceAssetLoader` ignores Bevy's file reader
/// and uses Godot's `ResourceLoader` directly for maximum compatibility.
///
/// ## Unified Asset Loading
/// ```rust
/// use bevy::prelude::*;
/// use bevy::asset::{AssetServer, Assets, Handle};
/// use godot::classes::PackedScene;
/// use godot_bevy::prelude::*;
///
/// fn load_assets(asset_server: Res<AssetServer>) {
///     // Load any Godot resource through Bevy's asset system (async, non-blocking)
///     let scene: Handle<GodotResource> = asset_server.load("scenes/player.tscn");
///     let audio: Handle<GodotResource> = asset_server.load("audio/music.ogg");
///     let texture: Handle<GodotResource> = asset_server.load("art/player.png");
/// }
///
/// #[derive(Resource)]
/// struct MyAssets {
///     scene: Handle<GodotResource>,
/// }
///
/// fn use_loaded_assets(
///     mut assets: ResMut<Assets<GodotResource>>,
///     my_assets: Res<MyAssets>, // Your loaded handles
/// ) {
///     if let Some(asset) = assets.get_mut(&my_assets.scene) {
///         if let Some(scene) = asset.try_cast::<PackedScene>() {
///             // Use the scene...
///         }
///     }
/// }
/// ```
///
/// **Benefits:**
/// - Non-blocking: Won't freeze your game during loading
/// - Integrates with Bevy's asset system (loading states, hot reloading, etc.)
/// - Better for large assets and batch loading
/// - Works seamlessly with `bevy_asset_loader`
/// - Unified system for all Godot resource types
///
/// This works identically in development and exported builds, including with .pck files.
#[derive(Default)]
pub struct GodotAssetsPlugin;

impl Plugin for GodotAssetsPlugin {
    fn build(&self, app: &mut App) {
        app.init_asset::<GodotResource>()
            .init_asset_loader::<GodotResourceAssetLoader>();
    }
}

/// Custom AssetReader that bypasses all filesystem verification.
/// This allows Godot's ResourceLoader to handle virtual paths from .pck files
/// without Bevy's asset system rejecting them for not existing on disk.
pub struct GodotAssetReader;

impl Default for GodotAssetReader {
    fn default() -> Self {
        Self::new()
    }
}

impl GodotAssetReader {
    pub fn new() -> Self {
        Self
    }
}

impl AssetReader for GodotAssetReader {
    async fn read<'a>(&'a self, _path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        // Return a dummy reader - our GodotResourceAssetLoader ignores this anyway
        Ok(VecReader::new(Vec::<u8>::new()))
    }

    async fn read_meta<'a>(
        &'a self,
        _path: &'a Path,
    ) -> Result<impl Reader + 'a, AssetReaderError> {
        // Return empty metadata
        Ok(VecReader::new(Vec::<u8>::new()))
    }

    async fn read_directory<'a>(
        &'a self,
        _path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        // Return empty directory listing
        let empty_iter = std::iter::empty::<std::path::PathBuf>();
        let stream = stream::iter(empty_iter);
        Ok(Box::new(stream) as Box<PathStream>)
    }

    async fn is_directory<'a>(&'a self, _path: &'a Path) -> Result<bool, AssetReaderError> {
        // Always report as not a directory
        Ok(false)
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

/// Tracks loading state for async Godot resource loading
#[derive(Debug)]
enum LoadingState {
    Requested,
    Loading,
    Ready,
    Failed,
}

/// Global state for tracking async loads
static LOADING_TRACKER: once_cell::sync::Lazy<Arc<Mutex<HashMap<String, LoadingState>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(HashMap::new())));

/// Universal AssetLoader for all Godot resources using async loading
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
        let godot_path = load_context.asset_path().to_string();

        {
            let mut resource_loader = ResourceLoader::singleton();
            let path_gstring = godot::builtin::GString::from(godot_path.clone());
            resource_loader.load_threaded_request(&path_gstring);
        }

        {
            let mut tracker = LOADING_TRACKER.lock().unwrap();
            tracker.insert(godot_path.clone(), LoadingState::Requested);
        }

        loop {
            let status = {
                let mut resource_loader = ResourceLoader::singleton();
                let path_gstring = godot::builtin::GString::from(godot_path.clone());
                resource_loader.load_threaded_get_status(&path_gstring)
            };

            match status {
                ThreadLoadStatus::LOADED => {
                    let resource = {
                        let mut resource_loader = ResourceLoader::singleton();
                        let path_gstring = godot::builtin::GString::from(godot_path.clone());
                        resource_loader.load_threaded_get(&path_gstring)
                    };

                    match resource {
                        Some(resource) => {
                            {
                                let mut tracker = LOADING_TRACKER.lock().unwrap();
                                tracker.insert(godot_path.clone(), LoadingState::Ready);
                            }

                            let handle = GodotResourceHandle::new(resource);
                            return Ok(GodotResource { handle });
                        }
                        None => {
                            // Update tracker
                            {
                                let mut tracker = LOADING_TRACKER.lock().unwrap();
                                tracker.insert(godot_path.clone(), LoadingState::Failed);
                            }

                            return Err(GodotAssetLoaderError::ResourceLoadFailed(format!(
                                "Failed to get loaded Godot resource: {godot_path}"
                            )));
                        }
                    }
                }
                ThreadLoadStatus::FAILED => {
                    {
                        let mut tracker = LOADING_TRACKER.lock().unwrap();
                        tracker.insert(godot_path.clone(), LoadingState::Failed);
                    }

                    return Err(GodotAssetLoaderError::ResourceLoadFailed(format!(
                        "Godot ResourceLoader failed to load: {godot_path}"
                    )));
                }
                ThreadLoadStatus::INVALID_RESOURCE => {
                    {
                        let mut tracker = LOADING_TRACKER.lock().unwrap();
                        tracker.insert(godot_path.clone(), LoadingState::Failed);
                    }

                    return Err(GodotAssetLoaderError::ResourceLoadFailed(format!(
                        "Invalid resource path or corrupted resource: {godot_path}"
                    )));
                }
                _ => {
                    {
                        let mut tracker = LOADING_TRACKER.lock().unwrap();
                        tracker.insert(godot_path.clone(), LoadingState::Loading);
                    }

                    futures_lite::future::yield_now().await;
                }
            }
        }
    }

    fn extensions(&self) -> &[&str] {
        &[
            "tscn", "scn", // Scenes
            "res", "tres", // Resources
            "jpg", "jpeg", "png", // Images
            "wav", "mp3", "ogg", "aac", // Audio
        ]
    }
}
