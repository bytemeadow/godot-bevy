use bevy::app::{App, Plugin, Update};
use bevy::asset::{AssetServer, Assets, Handle};
use bevy::ecs::system::ResMut;
use bevy::prelude::*;
use godot::classes::{AudioStream, AudioStreamPlayer};
use godot::obj::NewAlloc;
use std::collections::HashMap;
use thiserror::Error;

use super::assets::GodotResource;
use super::core::SceneTreeRef;
use crate::bridge::GodotNodeHandle;

/// Plugin that provides a convenient audio API using Godot's audio system.
/// Now integrated with Bevy's asset system for async loading and better performance.
pub struct GodotAudioPlugin;

impl Plugin for GodotAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioManager>().add_systems(
            Update,
            (process_sound_queue, cleanup_finished_sounds).chain(),
        );
    }
}

/// Main audio manager for playing sounds and music
#[derive(Resource, Default)]
pub struct AudioManager {
    one_shot_sounds: HashMap<SoundId, GodotNodeHandle>,
    next_id: u32,
    sound_queue: Vec<QueuedSound>,
    /// Cache of preloaded audio assets using Bevy handles
    cached_assets: HashMap<String, Handle<GodotResource>>,
}

/// Handle to a preloaded audio asset - now using Bevy's asset system
#[derive(Debug, Clone)]
pub struct AudioHandle {
    path: String,
    handle: Handle<GodotResource>,
}

/// Handle to a playing sound instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SoundId(u32);

/// Internal struct for queued sounds
#[derive(Debug)]
struct QueuedSound {
    id: SoundId,
    source: SoundSource,
    settings: SoundSettings,
}

/// Source of audio - either cached or needs loading
#[derive(Debug)]
enum SoundSource {
    Path(String),
    Handle(AudioHandle),
}

/// Settings for playing a sound
#[derive(Debug, Clone)]
pub struct SoundSettings {
    pub volume: f32,
    pub pitch: f32,
    pub looping: bool,
}

impl Default for SoundSettings {
    fn default() -> Self {
        Self {
            volume: 1.0,
            pitch: 1.0,
            looping: false,
        }
    }
}

impl SoundSettings {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn volume(mut self, volume: f32) -> Self {
        self.volume = volume.clamp(0.0, 1.0);
        self
    }

    pub fn pitch(mut self, pitch: f32) -> Self {
        self.pitch = pitch.clamp(0.1, 4.0);
        self
    }

    pub fn looped(mut self) -> Self {
        self.looping = true;
        self
    }
}

impl AudioManager {
    /// Load and cache an audio asset for efficient reuse using Bevy's asset system
    /// Returns a handle that can be used for playing instances
    pub fn load(&mut self, path: &str, asset_server: &AssetServer) -> AudioHandle {
        let path = path.to_string();

        // Check if already cached
        if let Some(handle) = self.cached_assets.get(&path) {
            return AudioHandle {
                path,
                handle: handle.clone(),
            };
        }

        // Load through Bevy's asset system (async)
        let handle: Handle<GodotResource> = asset_server.load(&path);
        self.cached_assets.insert(path.clone(), handle.clone());
        info!("Loading audio asset (async): {}", path);

        AudioHandle { path, handle }
    }

    /// Load an audio asset from an existing Handle<GodotResource>
    pub fn load_from_handle(&mut self, path: &str, handle: Handle<GodotResource>) -> AudioHandle {
        let path = path.to_string();
        self.cached_assets.insert(path.clone(), handle.clone());
        AudioHandle { path, handle }
    }

    /// Play a preloaded audio handle
    pub fn play_handle(&mut self, handle: &AudioHandle) -> Result<SoundId, AudioError> {
        self.play_handle_with_settings(handle, SoundSettings::default())
    }

    /// Play a preloaded audio handle with settings
    pub fn play_handle_with_settings(
        &mut self,
        handle: &AudioHandle,
        settings: SoundSettings,
    ) -> Result<SoundId, AudioError> {
        let id = SoundId(self.next_id);
        self.next_id += 1;

        self.sound_queue.push(QueuedSound {
            id,
            source: SoundSource::Handle(handle.clone()),
            settings,
        });

        Ok(id)
    }

    /// Play a sound file directly using asset server (loads async) - convenient but less efficient for repeated sounds
    pub fn play(&mut self, path: &str, asset_server: &AssetServer) -> Result<SoundId, AudioError> {
        self.play_with_settings(path, SoundSettings::default(), asset_server)
    }

    /// Play a sound file with custom settings using asset server (loads async)
    pub fn play_with_settings(
        &mut self,
        path: &str,
        settings: SoundSettings,
        asset_server: &AssetServer,
    ) -> Result<SoundId, AudioError> {
        let id = SoundId(self.next_id);
        self.next_id += 1;

        // If not cached, load it now
        if !self.cached_assets.contains_key(path) {
            let handle: Handle<GodotResource> = asset_server.load(path);
            self.cached_assets.insert(path.to_string(), handle);
        }

        self.sound_queue.push(QueuedSound {
            id,
            source: SoundSource::Path(path.to_string()),
            settings,
        });

        Ok(id)
    }

    /// Stop a specific sound
    pub fn stop(&mut self, id: SoundId) -> Result<(), AudioError> {
        if let Some(mut handle) = self.one_shot_sounds.remove(&id) {
            if let Some(mut player) = handle.try_get::<AudioStreamPlayer>() {
                player.stop();
            }
            Ok(())
        } else {
            Err(AudioError::SoundNotFound(id))
        }
    }

    /// Stop all playing sounds
    pub fn stop_all(&mut self) {
        for (_, mut handle) in self.one_shot_sounds.drain() {
            if let Some(mut player) = handle.try_get::<AudioStreamPlayer>() {
                player.stop();
            }
        }
    }

    /// Check if a sound is still playing
    pub fn is_playing(&mut self, id: SoundId) -> bool {
        if let Some(handle) = self.one_shot_sounds.get_mut(&id) {
            if let Some(player) = handle.try_get::<AudioStreamPlayer>() {
                return player.is_playing();
            }
        }
        false
    }

    /// Get the number of playing sounds
    pub fn playing_count(&self) -> usize {
        self.one_shot_sounds.len()
    }

    /// Clear the internal cache - useful for memory management in long-running games
    pub fn clear_cache(&mut self) {
        self.cached_assets.clear();
    }

    /// Get stats about cached assets and playing sounds
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.cached_assets.len(), self.one_shot_sounds.len())
    }
}

/// System that processes queued sounds using Bevy's asset system
fn process_sound_queue(
    mut audio_manager: ResMut<AudioManager>,
    mut assets: ResMut<Assets<GodotResource>>,
    mut scene_tree: SceneTreeRef,
) {
    // Take all queued sounds to process
    let queued_sounds = std::mem::take(&mut audio_manager.sound_queue);

    for queued in queued_sounds {
        let audio_stream = match &queued.source {
            SoundSource::Path(path) => {
                // Get from cache (should be loaded by now via asset server)
                if let Some(handle) = audio_manager.cached_assets.get(path) {
                    if let Some(asset) = assets.get_mut(handle) {
                        asset.try_cast::<AudioStream>()
                    } else {
                        // Asset not ready yet, re-queue for next frame
                        audio_manager.sound_queue.push(queued);
                        continue;
                    }
                } else {
                    warn!("Audio path not found in cache: {}", path);
                    continue;
                }
            }
            SoundSource::Handle(audio_handle) => {
                // Get from Bevy asset system
                if let Some(asset) = assets.get_mut(&audio_handle.handle) {
                    asset.try_cast::<AudioStream>()
                } else {
                    // Asset not ready yet, re-queue for next frame
                    audio_manager.sound_queue.push(queued);
                    continue;
                }
            }
        };

        if let Some(mut audio_stream) = audio_stream {
            // Configure looping on the stream itself if requested
            if queued.settings.looping {
                // Try to enable looping on the stream - this works for AudioStreamOggVorbis and similar
                // Note: Not all stream types support runtime loop changes
                if let Ok(mut ogg_stream) = audio_stream
                    .clone()
                    .try_cast::<godot::classes::AudioStreamOggVorbis>()
                {
                    ogg_stream.set_loop(true);
                    audio_stream = ogg_stream.upcast();
                } else if let Ok(mut wav_stream) = audio_stream
                    .clone()
                    .try_cast::<godot::classes::AudioStreamWav>()
                {
                    wav_stream.set_loop_mode(godot::classes::audio_stream_wav::LoopMode::FORWARD);
                    audio_stream = wav_stream.upcast();
                } else {
                    warn!(
                        "Audio stream type doesn't support runtime loop configuration: {}",
                        match &queued.source {
                            SoundSource::Path(path) => path,
                            SoundSource::Handle(handle) => &handle.path,
                        }
                    );
                }
            }

            // Create Godot AudioStreamPlayer
            let mut player = AudioStreamPlayer::new_alloc();
            player.set_stream(&audio_stream);
            player.set_volume_db(volume_to_db(queued.settings.volume));
            player.set_pitch_scale(queued.settings.pitch);

            // Add to scene tree
            if let Some(mut root) = scene_tree.get().get_root() {
                root.add_child(&player);
            }

            // Configure and play
            player.play();

            // Store the handle for tracking
            let handle = GodotNodeHandle::new(player);
            audio_manager.one_shot_sounds.insert(queued.id, handle);

            trace!("Started playing audio: {:?}", queued.id);
        } else {
            warn!("Failed to get audio stream for queued sound");
        }
    }
}

/// System that cleans up finished sounds
fn cleanup_finished_sounds(mut audio_manager: ResMut<AudioManager>) {
    let mut finished_sounds = Vec::new();

    for (&sound_id, handle) in audio_manager.one_shot_sounds.iter_mut() {
        if let Some(player) = handle.try_get::<AudioStreamPlayer>() {
            if !player.is_playing() {
                finished_sounds.push(sound_id);
            }
        } else {
            // Player was freed, consider it finished
            finished_sounds.push(sound_id);
        }
    }

    for sound_id in finished_sounds {
        audio_manager.one_shot_sounds.remove(&sound_id);
        trace!("Cleaned up finished sound: {:?}", sound_id);
    }
}

/// Convert linear volume (0.0-1.0) to decibels for Godot
fn volume_to_db(volume: f32) -> f32 {
    if volume <= 0.0 {
        -80.0 // Silence
    } else {
        20.0 * volume.log10()
    }
}

/// Possible errors that can be produced by the audio manager
#[derive(Debug, Error)]
pub enum AudioError {
    #[error("Sound not found: {0:?}")]
    SoundNotFound(SoundId),
}

/// Helper extension trait to make audio playing more convenient
pub trait AudioManagerExt {
    /// Play a sound with just a path - most convenient method
    fn play_sound(&mut self, path: &str, asset_server: &AssetServer) -> Result<SoundId, AudioError>;

    /// Play a sound with volume
    fn play_sound_with_volume(
        &mut self,
        path: &str,
        volume: f32,
        asset_server: &AssetServer,
    ) -> Result<SoundId, AudioError>;

    /// Play a looping sound
    fn play_looping_sound(
        &mut self,
        path: &str,
        asset_server: &AssetServer,
    ) -> Result<SoundId, AudioError>;

    /// Load an audio asset for reuse using AssetServer
    fn load_audio(&mut self, path: &str, asset_server: &AssetServer) -> AudioHandle;
}

impl AudioManagerExt for AudioManager {
    fn play_sound(&mut self, path: &str, asset_server: &AssetServer) -> Result<SoundId, AudioError> {
        self.play(path, asset_server)
    }

    fn play_sound_with_volume(
        &mut self,
        path: &str,
        volume: f32,
        asset_server: &AssetServer,
    ) -> Result<SoundId, AudioError> {
        self.play_with_settings(path, SoundSettings::new().volume(volume), asset_server)
    }

    fn play_looping_sound(
        &mut self,
        path: &str,
        asset_server: &AssetServer,
    ) -> Result<SoundId, AudioError> {
        self.play_with_settings(path, SoundSettings::new().looped(), asset_server)
    }

    fn load_audio(&mut self, path: &str, asset_server: &AssetServer) -> AudioHandle {
        self.load(path, asset_server)
    }
}

// Re-export for backward compatibility
pub use AudioManager as GodotAudio;
