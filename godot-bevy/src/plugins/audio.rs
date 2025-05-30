use bevy::app::{App, Plugin, Update};
use bevy::asset::{Assets, Handle};
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
/// Focuses purely on audio playback - asset loading is handled by Bevy's AssetServer.
pub struct GodotAudioPlugin;

impl Plugin for GodotAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<AudioManager>().add_systems(
            Update,
            (process_sound_queue, cleanup_finished_sounds).chain(),
        );
    }
}

/// Main audio manager for playing sounds and music.
#[derive(Resource, Default)]
pub struct AudioManager {
    playing_sounds: HashMap<SoundId, GodotNodeHandle>,
    next_id: u32,
    sound_queue: Vec<QueuedSound>,
}

/// Handle to a playing sound instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SoundId(u32);

/// Internal struct for queued sounds
#[derive(Debug)]
struct QueuedSound {
    id: SoundId,
    handle: Handle<GodotResource>,
    settings: SoundSettings,
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
    /// Play an audio asset from a Handle<GodotResource>
    pub fn play(&mut self, handle: Handle<GodotResource>) -> SoundId {
        self.play_with_settings(handle, SoundSettings::default())
    }

    /// Play an audio asset with custom settings
    pub fn play_with_settings(
        &mut self,
        handle: Handle<GodotResource>,
        settings: SoundSettings,
    ) -> SoundId {
        let id = SoundId(self.next_id);
        self.next_id += 1;

        self.sound_queue.push(QueuedSound {
            id,
            handle,
            settings,
        });

        id
    }

    /// Stop a specific sound
    pub fn stop(&mut self, id: SoundId) -> Result<(), AudioError> {
        if let Some(mut handle) = self.playing_sounds.remove(&id) {
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
        for (_, mut handle) in self.playing_sounds.drain() {
            if let Some(mut player) = handle.try_get::<AudioStreamPlayer>() {
                player.stop();
            }
        }
    }

    /// Check if a sound is still playing
    pub fn is_playing(&mut self, id: SoundId) -> bool {
        if let Some(handle) = self.playing_sounds.get_mut(&id) {
            if let Some(player) = handle.try_get::<AudioStreamPlayer>() {
                return player.is_playing();
            }
        }
        false
    }

    /// Get the number of playing sounds
    pub fn playing_count(&self) -> usize {
        self.playing_sounds.len()
    }

    /// Get stats about playing sounds
    pub fn stats(&self) -> (usize, usize) {
        (self.sound_queue.len(), self.playing_sounds.len())
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
        let audio_stream = if let Some(asset) = assets.get_mut(&queued.handle) {
            asset.try_cast::<AudioStream>()
        } else {
            // Asset not ready yet, re-queue for next frame
            audio_manager.sound_queue.push(queued);
            continue;
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
                        "Audio stream type doesn't support runtime loop configuration for asset: {:?}",
                        queued.handle
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
            audio_manager.playing_sounds.insert(queued.id, handle);

            trace!("Started playing audio: {:?}", queued.id);
        } else {
            warn!(
                "Failed to get audio stream for queued sound: {:?}",
                queued.handle
            );
        }
    }
}

/// System that cleans up finished sounds
fn cleanup_finished_sounds(mut audio_manager: ResMut<AudioManager>) {
    let mut finished_sounds = Vec::new();

    for (&sound_id, handle) in audio_manager.playing_sounds.iter_mut() {
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
        audio_manager.playing_sounds.remove(&sound_id);
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
    /// Play a sound from a handle
    fn play_sound(&mut self, handle: Handle<GodotResource>) -> SoundId;

    /// Play a sound with volume
    fn play_sound_with_volume(&mut self, handle: Handle<GodotResource>, volume: f32) -> SoundId;

    /// Play a looping sound
    fn play_looping_sound(&mut self, handle: Handle<GodotResource>) -> SoundId;
}

impl AudioManagerExt for AudioManager {
    fn play_sound(&mut self, handle: Handle<GodotResource>) -> SoundId {
        self.play(handle)
    }

    fn play_sound_with_volume(&mut self, handle: Handle<GodotResource>, volume: f32) -> SoundId {
        self.play_with_settings(handle, SoundSettings::new().volume(volume))
    }

    fn play_looping_sound(&mut self, handle: Handle<GodotResource>) -> SoundId {
        self.play_with_settings(handle, SoundSettings::new().looped())
    }
}

// Re-export for backward compatibility
pub use AudioManager as GodotAudio;
