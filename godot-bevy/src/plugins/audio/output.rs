//! Audio output management and sound tracking

use crate::bridge::GodotNodeHandle;
use crate::plugins::audio::ChannelId;
use bevy::prelude::*;
use godot::classes::{AudioStreamPlayer, AudioStreamPlayer2D, AudioStreamPlayer3D};
use std::collections::HashMap;

/// Handle to a playing sound instance
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SoundId(pub(crate) u32);

/// Individual channel output handling
#[derive(Resource, Default)]
pub struct AudioOutput {
    pub(crate) playing_sounds: HashMap<SoundId, GodotNodeHandle>,
    // Track which sounds belong to which channels
    pub(crate) sound_to_channel: HashMap<SoundId, ChannelId>,
    pub(crate) next_sound_id: u32,
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
        if let Some(handle) = self.playing_sounds.get_mut(&sound_id) {
            set_audio_player_volume(handle, volume.clamp(0.0, 1.0));
            trace!("Set volume to {} for sound: {:?}", volume, sound_id);
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
