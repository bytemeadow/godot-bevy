//! Audio output management and sound tracking

use crate::bridge::GodotNodeHandle;
use crate::plugins::audio::{AudioTween, ChannelId};
use bevy::prelude::*;
use godot::classes::{AudioStreamPlayer, AudioStreamPlayer2D, AudioStreamPlayer3D};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::Duration;

/// Unique identifier for a sound instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SoundId(pub(crate) u32);

impl SoundId {
    pub(crate) fn next() -> Self {
        static NEXT_ID: AtomicU32 = AtomicU32::new(0);
        Self(NEXT_ID.fetch_add(1, Ordering::Relaxed))
    }
}

/// Manages audio output and tracks playing sounds
#[derive(Resource, Default)]
pub struct AudioOutput {
    pub(crate) playing_sounds: HashMap<SoundId, GodotNodeHandle>,
    pub(crate) sound_to_channel: HashMap<SoundId, ChannelId>,
    /// Track current volume for each sound for accurate fade-outs
    pub(crate) current_volumes: HashMap<SoundId, f32>,
    pub(crate) active_tweens: HashMap<SoundId, ActiveTween>,
}

/// Tracks an active tween for a specific sound
#[derive(Debug, Clone)]
pub struct ActiveTween {
    pub tween_type: TweenType,
    pub start_value: f32,
    pub target_value: f32,
    pub duration: Duration,
    pub elapsed: Duration,
    pub easing: super::AudioEasing,
}

/// Type of tween being applied
#[derive(Debug, Clone)]
pub enum TweenType {
    Volume,
    Pitch,
    FadeOut, // Special case for fade-out to remove sound when complete
}

impl AudioOutput {
    /// Get the number of currently playing sounds
    pub fn playing_count(&self) -> usize {
        self.playing_sounds.len()
    }

    /// Check if a specific sound is still playing
    pub fn is_playing(&self, sound_id: SoundId) -> bool {
        self.playing_sounds.contains_key(&sound_id)
    }

    /// Get the channel that a sound belongs to
    pub fn sound_channel(&self, sound_id: SoundId) -> Option<ChannelId> {
        self.sound_to_channel.get(&sound_id).copied()
    }

    // ===== DIRECT INDIVIDUAL SOUND CONTROL =====

    /// Set volume for a specific sound (direct execution)
    pub fn set_sound_volume(&mut self, sound_id: SoundId, volume: f32) {
        let clamped_volume = volume.clamp(0.0, 1.0);
        if let Some(handle) = self.playing_sounds.get_mut(&sound_id) {
            set_audio_player_volume(handle, clamped_volume);
            // Track the current volume for accurate fade-outs
            self.current_volumes.insert(sound_id, clamped_volume);
            trace!("Set volume to {} for sound: {:?}", clamped_volume, sound_id);
        }
    }

    /// Set pitch for a specific sound (direct execution)
    pub fn set_sound_pitch(&mut self, sound_id: SoundId, pitch: f32) {
        if let Some(handle) = self.playing_sounds.get_mut(&sound_id) {
            set_audio_player_pitch(handle, pitch.clamp(0.1, 4.0));
            trace!("Set pitch to {} for sound: {:?}", pitch, sound_id);
        }
    }

    /// Pause a specific sound (direct execution)
    pub fn pause_sound(&mut self, sound_id: SoundId) {
        if let Some(handle) = self.playing_sounds.get_mut(&sound_id) {
            pause_audio_player(handle);
            trace!("Paused sound: {:?}", sound_id);
        }
    }

    /// Resume a specific sound (direct execution)
    pub fn resume_sound(&mut self, sound_id: SoundId) {
        if let Some(handle) = self.playing_sounds.get_mut(&sound_id) {
            resume_audio_player(handle);
            trace!("Resumed sound: {:?}", sound_id);
        }
    }

    /// Stop a specific sound (direct execution)
    pub fn stop_sound(&mut self, sound_id: SoundId) {
        if let Some(mut handle) = self.playing_sounds.remove(&sound_id) {
            stop_audio_player(&mut handle);
            self.sound_to_channel.remove(&sound_id);
            self.current_volumes.remove(&sound_id); // Clean up volume tracking
            trace!("Stopped sound: {:?}", sound_id);
        }
    }
}

// ===== HELPER FUNCTIONS FOR DIRECT AUDIO CONTROL =====

/// Convert linear volume (0.0-1.0) to decibels for Godot
fn volume_to_db(volume: f32) -> f32 {
    if volume <= 0.0 {
        -80.0 // Silence
    } else {
        20.0 * volume.log10()
    }
}

fn set_audio_player_volume(handle: &mut GodotNodeHandle, volume: f32) {
    let volume_db = volume_to_db(volume);
    if let Some(mut player) = handle.try_get::<AudioStreamPlayer>() {
        player.set_volume_db(volume_db);
    } else if let Some(mut player) = handle.try_get::<AudioStreamPlayer2D>() {
        player.set_volume_db(volume_db);
    } else if let Some(mut player) = handle.try_get::<AudioStreamPlayer3D>() {
        player.set_volume_db(volume_db);
    }
}

fn set_audio_player_pitch(handle: &mut GodotNodeHandle, pitch: f32) {
    if let Some(mut player) = handle.try_get::<AudioStreamPlayer>() {
        player.set_pitch_scale(pitch);
    } else if let Some(mut player) = handle.try_get::<AudioStreamPlayer2D>() {
        player.set_pitch_scale(pitch);
    } else if let Some(mut player) = handle.try_get::<AudioStreamPlayer3D>() {
        player.set_pitch_scale(pitch);
    }
}

fn pause_audio_player(handle: &mut GodotNodeHandle) {
    if let Some(mut player) = handle.try_get::<AudioStreamPlayer>() {
        player.set_stream_paused(true);
    } else if let Some(mut player) = handle.try_get::<AudioStreamPlayer2D>() {
        player.set_stream_paused(true);
    } else if let Some(mut player) = handle.try_get::<AudioStreamPlayer3D>() {
        player.set_stream_paused(true);
    }
}

fn resume_audio_player(handle: &mut GodotNodeHandle) {
    if let Some(mut player) = handle.try_get::<AudioStreamPlayer>() {
        player.set_stream_paused(false);
    } else if let Some(mut player) = handle.try_get::<AudioStreamPlayer2D>() {
        player.set_stream_paused(false);
    } else if let Some(mut player) = handle.try_get::<AudioStreamPlayer3D>() {
        player.set_stream_paused(false);
    }
}

fn stop_audio_player(handle: &mut GodotNodeHandle) {
    // Try each player type
    if let Some(mut player) = handle.try_get::<AudioStreamPlayer>() {
        player.stop();
    } else if let Some(mut player) = handle.try_get::<AudioStreamPlayer2D>() {
        player.stop();
    } else if let Some(mut player) = handle.try_get::<AudioStreamPlayer3D>() {
        player.stop();
    }
}

impl ActiveTween {
    pub fn new_fade_in(target_volume: f32, tween: AudioTween) -> Self {
        Self {
            tween_type: TweenType::Volume,
            start_value: 0.0,
            target_value: target_volume,
            duration: tween.duration,
            elapsed: Duration::ZERO,
            easing: tween.easing,
        }
    }

    pub fn new_fade_out(current_volume: f32, tween: AudioTween) -> Self {
        Self {
            tween_type: TweenType::FadeOut,
            start_value: current_volume,
            target_value: 0.0,
            duration: tween.duration,
            elapsed: Duration::ZERO,
            easing: tween.easing,
        }
    }

    pub fn new_volume(start: f32, target: f32, tween: AudioTween) -> Self {
        Self {
            tween_type: TweenType::Volume,
            start_value: start,
            target_value: target,
            duration: tween.duration,
            elapsed: Duration::ZERO,
            easing: tween.easing,
        }
    }

    pub fn new_pitch(start: f32, target: f32, tween: AudioTween) -> Self {
        Self {
            tween_type: TweenType::Pitch,
            start_value: start,
            target_value: target,
            duration: tween.duration,
            elapsed: Duration::ZERO,
            easing: tween.easing,
        }
    }

    /// Update the tween and return the current interpolated value
    pub fn update(&mut self, delta: Duration) -> f32 {
        self.elapsed += delta;

        // Handle zero duration case - return target value immediately
        if self.duration.as_secs_f32() == 0.0 {
            return self.target_value;
        }

        let progress = (self.elapsed.as_secs_f32() / self.duration.as_secs_f32()).clamp(0.0, 1.0);

        // Apply easing
        let eased_progress = match self.easing {
            super::AudioEasing::Linear => progress,
            super::AudioEasing::EaseIn => progress * progress,
            super::AudioEasing::EaseOut => 1.0 - (1.0 - progress) * (1.0 - progress),
            super::AudioEasing::EaseInOut => {
                if progress < 0.5 {
                    2.0 * progress * progress
                } else {
                    1.0 - 2.0 * (1.0 - progress) * (1.0 - progress)
                }
            }
        };

        // Interpolate between start and target
        self.start_value + (self.target_value - self.start_value) * eased_progress
    }

    /// Check if the tween is complete
    pub fn is_complete(&self) -> bool {
        self.elapsed >= self.duration
    }
}
