use bevy_app::{App, Plugin};
use bevy_asset::{
    Asset, AssetApp, AssetLoader, AssetMetaCheck, AssetPlugin, LoadContext,
    io::{
        AssetReader, AssetReaderError, AssetSourceBuilder, AssetSourceId, PathStream, Reader,
        ReaderNotSeekableError, SeekableReader, VecReader,
    },
};
use bevy_reflect::TypePath;
use futures_lite::io::AsyncRead;
use futures_lite::stream;
use godot::classes::FileAccess;
use godot::classes::ResourceLoader;
use godot::classes::file_access::ModeFlags;
#[cfg(feature = "experimental-threads")]
use godot::classes::resource_loader::ThreadLoadStatus;
use godot::obj::{Gd, Singleton};
use godot::prelude::Resource as GodotBaseResource;
use std::io;
use std::path::Path;
use std::pin::Pin;
use std::task::{Context, Poll};
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
/// ```ignore
/// use bevy::prelude::*;
/// use bevy_asset::{AssetServer, Assets, Handle};
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
        // IMPORTANT: Register custom AssetReader BEFORE setting up AssetPlugin.
        // Each source reconstructs the Godot VFS path with its own scheme.
        app.register_asset_source(
            AssetSourceId::Default,
            AssetSourceBuilder::new(|| Box::new(GodotAssetReader::new("res://"))),
        );
        app.register_asset_source(
            AssetSourceId::from("res"),
            AssetSourceBuilder::new(|| Box::new(GodotAssetReader::new("res://"))),
        );
        app.register_asset_source(
            AssetSourceId::from("user"),
            AssetSourceBuilder::new(|| Box::new(GodotAssetReader::new("user://"))),
        );
        app.register_asset_source(
            AssetSourceId::from("uid"),
            AssetSourceBuilder::new(|| Box::new(GodotAssetReader::new("uid://"))),
        );

        // Configure AssetPlugin to bypass path verification for Godot resources
        app.add_plugins(AssetPlugin {
            meta_check: AssetMetaCheck::Never,
            ..Default::default()
        });

        app.init_asset::<GodotResource>()
            .init_asset_loader::<GodotResourceAssetLoader>();
    }
}

/// Reconstructs the Godot VFS path for its source and hands back a lazy,
/// `FileAccess`-backed reader. Private: only `GodotAssetsPlugin` constructs it.
struct GodotAssetReader {
    scheme: &'static str, // "res://" | "user://" | "uid://"
}

impl GodotAssetReader {
    fn new(scheme: &'static str) -> Self {
        Self { scheme }
    }

    fn godot_path(&self, path: &Path) -> String {
        format!("{}{}", self.scheme, path.to_string_lossy())
    }
}

impl AssetReader for GodotAssetReader {
    async fn read<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        // Never do IO here and never return NotFound on file existence: returning Ok
        // keeps the already-selected loader running (GodotResourceAssetLoader ignores
        // the reader and resolves imported/remap/uid assets via ResourceLoader, which
        // FileAccess can't see in exports). Byte loaders that actually read get real
        // bytes on first access, or a clean io::Error if the file is missing.
        Ok(GodotFileReader::pending(self.godot_path(path)))
    }

    async fn read_meta<'a>(&'a self, path: &'a Path) -> Result<impl Reader + 'a, AssetReaderError> {
        // Godot has no bevy `.meta` sidecar; NotFound is the signal bevy handles
        // gracefully (falls back to loader-by-extension with default meta).
        Err::<VecReader, _>(AssetReaderError::NotFound(path.to_path_buf()))
    }

    async fn read_directory<'a>(
        &'a self,
        _path: &'a Path,
    ) -> Result<Box<PathStream>, AssetReaderError> {
        // Directory support is descoped: a real listing of a Godot dir yields
        // `.import`/`.uid`/`.gd` sidecars with no bevy loader, which hard-fails
        // load_folder. Keep the empty stream so load_folder is a no-op, not a failure.
        let empty_iter = std::iter::empty::<std::path::PathBuf>();
        let stream = stream::iter(empty_iter);
        Ok(Box::new(stream) as Box<PathStream>)
    }

    async fn is_directory<'a>(&'a self, _path: &'a Path) -> Result<bool, AssetReaderError> {
        Ok(false)
    }
}

/// Lazy, `FileAccess`-backed [`Reader`]. Holds a Godot VFS path until first access,
/// then materializes the whole file into a `VecReader`. Never stores a `Gd` across a
/// call, so it stays `Send + Sync + Unpin`.
enum GodotFileReader {
    Pending(String),  // godot VFS path; no Gd stored
    Ready(VecReader), // materialized bytes
}

impl GodotFileReader {
    fn pending(godot_path: String) -> Self {
        GodotFileReader::Pending(godot_path)
    }

    /// Read the file on first access; idempotent once `Ready`.
    fn ensure_ready(&mut self) -> io::Result<&mut VecReader> {
        if let GodotFileReader::Pending(path) = self {
            let bytes = read_godot_file(path)?;
            *self = GodotFileReader::Ready(VecReader::new(bytes));
        }
        match self {
            GodotFileReader::Ready(reader) => Ok(reader),
            GodotFileReader::Pending(_) => unreachable!(),
        }
    }
}

impl AsyncRead for GodotFileReader {
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut [u8],
    ) -> Poll<io::Result<usize>> {
        let this = self.get_mut(); // GodotFileReader: Unpin (String / VecReader are Unpin)
        match this.ensure_ready() {
            Ok(reader) => Pin::new(reader).poll_read(cx, buf),
            Err(e) => Poll::Ready(Err(e)),
        }
    }
}

impl Reader for GodotFileReader {
    fn seekable(&mut self) -> Result<&mut dyn SeekableReader, ReaderNotSeekableError> {
        // Materialize so seeking loaders get a real seekable VecReader. On open
        // failure report not-seekable; the loader's fallback reads via read_to_end,
        // which surfaces the same open error.
        match self.ensure_ready() {
            Ok(reader) => reader.seekable(),
            Err(_) => Err(ReaderNotSeekableError),
        }
    }
}

/// Read a whole Godot file into a byte vec via `FileAccess`. Open failure is the
/// `None` branch (mapped to `NotFound`), distinct from an empty file (`Some`, length 0).
fn read_godot_file(godot_path: &str) -> io::Result<Vec<u8>> {
    match FileAccess::open(godot_path, ModeFlags::READ) {
        Some(mut fa) => {
            let len = fa.get_length() as i64;
            Ok(fa.get_buffer(len).to_vec())
        }
        None => Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("Godot FileAccess could not open '{godot_path}'"),
        )),
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

/// Universal AssetLoader for all Godot resources using async loading
#[derive(Default, TypePath)]
pub struct GodotResourceAssetLoader;

impl AssetLoader for GodotResourceAssetLoader {
    type Asset = GodotResource;
    type Settings = ();
    type Error = GodotAssetLoaderError;

    #[cfg(feature = "experimental-threads")]
    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let godot_path = load_context.path().to_string();

        {
            let mut resource_loader = ResourceLoader::singleton();
            let path_gstring = godot::builtin::GString::from(&godot_path);
            resource_loader.load_threaded_request(&path_gstring);
        }

        loop {
            let status = {
                let mut resource_loader = ResourceLoader::singleton();
                let path_gstring = godot::builtin::GString::from(&godot_path);
                resource_loader.load_threaded_get_status(&path_gstring)
            };

            match status {
                ThreadLoadStatus::LOADED => {
                    let resource = {
                        let mut resource_loader = ResourceLoader::singleton();
                        let path_gstring = godot::builtin::GString::from(&godot_path);
                        resource_loader.load_threaded_get(&path_gstring)
                    };

                    match resource {
                        Some(resource) => {
                            let handle = GodotResourceHandle::new(resource);
                            return Ok(GodotResource { handle });
                        }
                        None => {
                            return Err(GodotAssetLoaderError::ResourceLoadFailed(format!(
                                "Failed to get loaded Godot resource: {godot_path}"
                            )));
                        }
                    }
                }
                ThreadLoadStatus::FAILED => {
                    return Err(GodotAssetLoaderError::ResourceLoadFailed(format!(
                        "Godot ResourceLoader failed to load: {godot_path}"
                    )));
                }
                ThreadLoadStatus::INVALID_RESOURCE => {
                    return Err(GodotAssetLoaderError::ResourceLoadFailed(format!(
                        "Invalid resource path or corrupted resource: {godot_path}"
                    )));
                }
                _ => {
                    futures_lite::future::yield_now().await;
                }
            }
        }
    }

    /// Synchronous loading fallback when threaded loading is not available.
    /// Used for web/WASM builds and when experimental-threads is not enabled.
    #[cfg(not(feature = "experimental-threads"))]
    async fn load(
        &self,
        _reader: &mut dyn Reader,
        _settings: &(),
        load_context: &mut LoadContext<'_>,
    ) -> Result<Self::Asset, Self::Error> {
        let godot_path = load_context.path().to_string();
        let path_gstring = godot::builtin::GString::from(&godot_path);

        let mut resource_loader = ResourceLoader::singleton();
        let resource = resource_loader.load(&path_gstring);

        match resource {
            Some(resource) => {
                let handle = GodotResourceHandle::new(resource);
                Ok(GodotResource { handle })
            }
            None => Err(GodotAssetLoaderError::ResourceLoadFailed(format!(
                "Failed to load Godot resource: {godot_path}"
            ))),
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
