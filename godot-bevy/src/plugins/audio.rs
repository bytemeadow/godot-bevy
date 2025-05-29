use bevy::app::{App, Plugin, Update};
use bevy::ecs::system::{Res, ResMut};
use bevy::prelude::*;
use godot::classes::{AudioStream, AudioStreamPlayer};
use godot::obj::NewAlloc;
use std::collections::HashMap;

use super::assets::GodotResourceLoader;
use super::core::SceneTreeRef;
use crate::bridge::{GodotNodeHandle, GodotResourceHandle};

/// Plugin that provides a Kira-like audio API using Godot's audio system.
pub struct GodotAudioPlugin;

impl Plugin for GodotAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioManager>().add_systems(
            Update,
            (process_sound_queue, cleanup_finished_sounds).chain(),
        );
    }
}

/// Main audio manager - similar to Kira's AudioManager
#[derive(Resource, Default)]
pub struct AudioManager {
    one_shot_sounds: HashMap<SoundId, GodotNodeHandle>,
    next_id: u32,
    sound_queue: Vec<QueuedSound>,
    /// Cache of preloaded audio assets using thread-safe handles
    cached_assets: HashMap<String, GodotResourceHandle>,
}

/// Handle to a preloaded audio asset - similar to Kira's Handle<AudioSource>
#[derive(Debug, Clone)]
pub struct AudioHandle {
    path: String,
}

/// Handle to a playing sound instance - similar to Kira's Handle<AudioInstance>
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

/// Settings for playing a sound - similar to Kira's SoundSettings
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
    /// Load and cache an audio asset for efficient reuse - similar to Kira's asset loading
    /// Returns a handle that can be used for playing instances
    pub fn load(
        &mut self,
        path: &str,
        godot_loader: &GodotResourceLoader,
    ) -> Result<AudioHandle, AudioError> {
        let path = path.to_string();

        // Check if already cached
        if self.cached_assets.contains_key(&path) {
            return Ok(AudioHandle { path });
        }

        // Load and cache the asset
        if let Some(audio_resource) = godot_loader.load(&path) {
            let handle = GodotResourceHandle::new(audio_resource);
            self.cached_assets.insert(path.clone(), handle);
            info!("Loaded and cached audio: {}", path);
            Ok(AudioHandle { path })
        } else {
            Err(AudioError::LoadError(path))
        }
    }

    /// Play a preloaded audio handle - similar to Kira's audio.play()
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

    /// Play a sound file directly (loads every time) - convenient but less efficient for repeated sounds
    pub fn play(&mut self, path: &str) -> Result<SoundId, AudioError> {
        self.play_with_settings(path, SoundSettings::default())
    }

    /// Play a sound file with custom settings (loads every time)
    pub fn play_with_settings(
        &mut self,
        path: &str,
        settings: SoundSettings,
    ) -> Result<SoundId, AudioError> {
        let id = SoundId(self.next_id);
        self.next_id += 1;

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

    /// Clear the audio cache (useful for memory management)
    pub fn clear_cache(&mut self) {
        self.cached_assets.clear();
        info!("Audio cache cleared");
    }

    /// Get cache statistics
    pub fn cache_stats(&self) -> (usize, usize) {
        (self.cached_assets.len(), self.one_shot_sounds.len())
    }
}

/// System that processes queued sounds
fn process_sound_queue(
    mut audio_manager: ResMut<AudioManager>,
    godot_loader: Res<GodotResourceLoader>,
    mut scene_tree: SceneTreeRef,
) {
    // Take all queued sounds to process
    let queued_sounds = std::mem::take(&mut audio_manager.sound_queue);

    for queued in queued_sounds {
        let audio_stream = match &queued.source {
            SoundSource::Path(path) => {
                // Load directly from path (not cached)
                godot_loader.load_as::<AudioStream>(path)
            }
            SoundSource::Handle(handle) => {
                // Get from cache
                if let Some(cached_handle) = audio_manager.cached_assets.get_mut(&handle.path) {
                    cached_handle.get().try_cast::<AudioStream>().ok()
                } else {
                    warn!("Audio handle not found in cache: {}", handle.path);
                    continue;
                }
            }
        };

        if let Some(mut audio_stream) = audio_stream {
            // Configure looping on the stream itself if requested
            if queued.settings.looping {
                // Try to enable looping on the stream - this works for AudioStreamOggVorbis and similar
                // Note: Not all stream types support runtime loop changes
                if let Some(mut ogg_stream) = audio_stream.clone().try_cast::<godot::classes::AudioStreamOggVorbis>().ok() {
                    ogg_stream.set_loop(true);
                    audio_stream = ogg_stream.upcast();
                } else if let Some(mut wav_stream) = audio_stream.clone().try_cast::<godot::classes::AudioStreamWav>().ok() {
                    wav_stream.set_loop_mode(godot::classes::audio_stream_wav::LoopMode::FORWARD);
                    audio_stream = wav_stream.upcast();
                } else {
                    warn!("Audio stream type doesn't support runtime loop configuration: {}", 
                          match &queued.source {
                              SoundSource::Path(path) => path,
                              SoundSource::Handle(handle) => &handle.path,
                          });
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

            let source_info = match &queued.source {
                SoundSource::Path(path) => format!("path: {}", path),
                SoundSource::Handle(handle) => format!("cached: {}", handle.path),
            };
            info!(
                "Started playing sound: {} (ID: {:?})",
                source_info, queued.id
            );
        } else {
            let source_info = match &queued.source {
                SoundSource::Path(path) => path.clone(),
                SoundSource::Handle(handle) => handle.path.clone(),
            };
            warn!("Failed to load audio: {}", source_info);
        }
    }
}

/// System that cleans up finished sounds
fn cleanup_finished_sounds(mut audio_manager: ResMut<AudioManager>) {
    let before_count = audio_manager.one_shot_sounds.len();

    audio_manager.one_shot_sounds.retain(|id, handle| {
        if let Some(player) = handle.try_get::<AudioStreamPlayer>() {
            let is_playing = player.is_playing();
            if !is_playing {
                debug!("Sound finished: {:?}", id);
            }
            is_playing
        } else {
            false // Remove invalid handles
        }
    });

    let after_count = audio_manager.one_shot_sounds.len();
    if before_count != after_count {
        debug!("Cleaned up {} finished sounds", before_count - after_count);
    }
}

/// Convert linear volume (0.0-1.0) to decibels.
fn volume_to_db(volume: f32) -> f32 {
    if volume <= 0.0 {
        -80.0 // Effectively muted
    } else {
        20.0 * volume.log10()
    }
}

/// Error type for audio operations
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("Failed to load audio file: {0}")]
    LoadError(String),
    #[error("Sound not found: {0:?}")]
    SoundNotFound(SoundId),
}

// Re-export for backward compatibility and convenience
pub use AudioManager as GodotAudio;

/// Helper extension trait to make audio playing even more convenient
pub trait AudioManagerExt {
    /// Play a sound with just a path - most convenient method
    fn play_sound(&mut self, path: &str) -> Result<SoundId, AudioError>;

    /// Play a sound with volume
    fn play_sound_with_volume(&mut self, path: &str, volume: f32) -> Result<SoundId, AudioError>;

    /// Play a looping sound
    fn play_looping_sound(&mut self, path: &str) -> Result<SoundId, AudioError>;

    /// Load an audio asset for reuse (requires GodotResourceLoader)
    fn load_audio(
        &mut self,
        path: &str,
        loader: &GodotResourceLoader,
    ) -> Result<AudioHandle, AudioError>;
}

impl AudioManagerExt for AudioManager {
    fn play_sound(&mut self, path: &str) -> Result<SoundId, AudioError> {
        self.play(path)
    }

    fn play_sound_with_volume(&mut self, path: &str, volume: f32) -> Result<SoundId, AudioError> {
        self.play_with_settings(path, SoundSettings::new().volume(volume))
    }

    fn play_looping_sound(&mut self, path: &str) -> Result<SoundId, AudioError> {
        self.play_with_settings(path, SoundSettings::new().looped())
    }

    fn load_audio(
        &mut self,
        path: &str,
        loader: &GodotResourceLoader,
    ) -> Result<AudioHandle, AudioError> {
        self.load(path, loader)
    }
}
