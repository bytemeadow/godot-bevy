//! Audio channel management and typed channels

use crate::plugins::assets::GodotResource;
use crate::plugins::audio::{
    AudioCommand, AudioPlayerType, AudioSettings, AudioTween, PlayCommand, SoundId,
};
use bevy::asset::Handle;
use bevy::prelude::*;
use parking_lot::RwLock;
use std::collections::VecDeque;
use std::marker::PhantomData;

/// Channel identifier for tracking which sounds belong to which channels
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ChannelId(pub &'static str);

#[derive(Debug, Clone)]
pub(crate) struct ChannelState {
    #[allow(dead_code)]
    pub volume: f32,
    #[allow(dead_code)]
    pub pitch: f32,
    #[allow(dead_code)]
    pub paused: bool,
    #[allow(dead_code)]
    pub panning: f32, // For 2D/non-positional audio
}

impl Default for ChannelState {
    fn default() -> Self {
        Self {
            volume: 1.0,
            pitch: 1.0,
            paused: false,
            panning: 0.0,
        }
    }
}

/// Trait that audio channel marker types must implement
pub trait AudioChannelMarker: Resource {
    const CHANNEL_NAME: &'static str;
}

/// Typed audio channel resource - each channel type gets its own instance
#[derive(Resource)]
pub struct AudioChannel<T: AudioChannelMarker> {
    pub(crate) channel_id: ChannelId,
    pub(crate) commands: RwLock<VecDeque<AudioCommand>>,
    _marker: PhantomData<T>,
}

impl<T: AudioChannelMarker> AudioChannel<T> {
    pub fn new(channel_id: ChannelId) -> Self {
        Self {
            channel_id,
            commands: RwLock::new(VecDeque::new()),
            _marker: PhantomData,
        }
    }

    /// Get the channel ID
    pub fn id(&self) -> &ChannelId {
        &self.channel_id
    }

    /// Queue a command for this channel (internal method)
    fn queue_command(&self, command: AudioCommand) {
        self.commands.write().push_back(command);
    }

    /// Play audio with configurable settings - returns a fluent builder
    pub fn play(&self, handle: Handle<GodotResource>) -> PlayAudioCommand<T> {
        PlayAudioCommand::new(
            self.channel_id,
            handle,
            AudioPlayerType::NonPositional,
            self,
        )
    }

    /// Play 2D positional audio
    pub fn play_2d(&self, handle: Handle<GodotResource>, position: Vec2) -> PlayAudioCommand<T> {
        PlayAudioCommand::new(
            self.channel_id,
            handle,
            AudioPlayerType::Spatial2D { position },
            self,
        )
    }

    /// Play 3D positional audio
    pub fn play_3d(&self, handle: Handle<GodotResource>, position: Vec3) -> PlayAudioCommand<T> {
        PlayAudioCommand::new(
            self.channel_id,
            handle,
            AudioPlayerType::Spatial3D { position },
            self,
        )
    }

    /// Stop all sounds in this channel
    pub fn stop(&self) {
        self.queue_command(AudioCommand::Stop(self.channel_id, None));
    }

    /// Stop all sounds with fade-out
    pub fn stop_with_fade(&self, fade_out: AudioTween) {
        self.queue_command(AudioCommand::Stop(self.channel_id, Some(fade_out)));
    }

    /// Pause all sounds in this channel
    pub fn pause(&self) {
        self.queue_command(AudioCommand::Pause(self.channel_id, None));
    }

    /// Resume all sounds in this channel
    pub fn resume(&self) {
        self.queue_command(AudioCommand::Resume(self.channel_id, None));
    }

    /// Set volume for all sounds in this channel
    pub fn set_volume(&self, volume: f32) {
        self.queue_command(AudioCommand::SetVolume(
            self.channel_id,
            volume.clamp(0.0, 1.0),
            None,
        ));
    }

    /// Set volume with fade transition
    pub fn set_volume_with_fade(&self, volume: f32, tween: AudioTween) {
        self.queue_command(AudioCommand::SetVolume(
            self.channel_id,
            volume.clamp(0.0, 1.0),
            Some(tween),
        ));
    }

    /// Set pitch for all sounds in this channel
    pub fn set_pitch(&self, pitch: f32) {
        self.queue_command(AudioCommand::SetPitch(
            self.channel_id,
            pitch.clamp(0.1, 4.0),
            None,
        ));
    }

    /// Set panning for all sounds in this channel (non-positional only)
    pub fn set_panning(&self, panning: f32) {
        self.queue_command(AudioCommand::SetPanning(
            self.channel_id,
            panning.clamp(-1.0, 1.0),
            None,
        ));
    }
}

/// Fluent builder for playing audio with configurable settings
pub struct PlayAudioCommand<'a, T: AudioChannelMarker> {
    channel_id: ChannelId,
    handle: Handle<GodotResource>,
    player_type: AudioPlayerType,
    settings: AudioSettings,
    sound_id: SoundId,
    channel: &'a AudioChannel<T>,
}

impl<'a, T: AudioChannelMarker> PlayAudioCommand<'a, T> {
    pub(crate) fn new(
        channel_id: ChannelId,
        handle: Handle<GodotResource>,
        player_type: AudioPlayerType,
        channel: &'a AudioChannel<T>,
    ) -> Self {
        let sound_id = SoundId::next();

        Self {
            channel_id,
            handle,
            player_type,
            settings: AudioSettings::default(),
            sound_id,
            channel,
        }
    }

    /// Set the volume (0.0 to 1.0)
    pub fn volume(mut self, volume: f32) -> Self {
        self.settings.volume = volume.clamp(0.0, 1.0);
        self
    }

    /// Set the pitch/playback rate (0.1 to 4.0)
    pub fn pitch(mut self, pitch: f32) -> Self {
        self.settings.pitch = pitch.clamp(0.1, 4.0);
        self
    }

    /// Enable looping
    pub fn looped(mut self) -> Self {
        self.settings.looping = true;
        self
    }

    /// Set fade-in duration with linear easing
    pub fn fade_in(mut self, duration: std::time::Duration) -> Self {
        self.settings.fade_in = Some(AudioTween::linear(duration));
        self
    }

    /// Set fade-in with custom easing
    pub fn fade_in_with_easing(mut self, tween: AudioTween) -> Self {
        self.settings.fade_in = Some(tween);
        self
    }

    /// Start playback from specific position (in seconds)
    pub fn start_from(mut self, position: f32) -> Self {
        self.settings.start_position = position.max(0.0);
        self
    }

    /// Set panning for non-positional audio (-1.0 left, 0.0 center, 1.0 right)
    pub fn panning(mut self, panning: f32) -> Self {
        self.settings.panning = Some(panning.clamp(-1.0, 1.0));
        self
    }
}

// Auto-queue the command when the builder is dropped
impl<T: AudioChannelMarker> Drop for PlayAudioCommand<'_, T> {
    fn drop(&mut self) {
        let command = AudioCommand::Play(PlayCommand {
            channel_id: self.channel_id,
            handle: self.handle.clone(),
            player_type: self.player_type.clone(),
            settings: self.settings.clone(),
            sound_id: self.sound_id,
        });

        self.channel.queue_command(command);
    }
}

/// Default main audio track  
#[derive(Resource)]
pub struct MainAudioTrack;

impl AudioChannelMarker for MainAudioTrack {
    const CHANNEL_NAME: &'static str = "main";
}

/// Audio parameter validation utilities.
///
/// These functions provide testable implementations of audio parameter
/// validation and processing used throughout the audio system.
pub mod validation {
    /// Audio parameter bounds
    pub mod bounds {
        pub const VOLUME_MIN: f32 = 0.0;
        pub const VOLUME_MAX: f32 = 1.0;
        pub const PITCH_MIN: f32 = 0.1;
        pub const PITCH_MAX: f32 = 4.0;
        pub const PANNING_MIN: f32 = -1.0;
        pub const PANNING_MAX: f32 = 1.0;
    }

    /// Validate and clamp volume to valid range [0.0, 1.0]
    pub fn clamp_volume(volume: f32) -> f32 {
        volume.clamp(bounds::VOLUME_MIN, bounds::VOLUME_MAX)
    }

    /// Validate and clamp pitch to valid range [0.1, 4.0]
    pub fn clamp_pitch(pitch: f32) -> f32 {
        pitch.clamp(bounds::PITCH_MIN, bounds::PITCH_MAX)
    }

    /// Validate and clamp panning to valid range [-1.0, 1.0]
    pub fn clamp_panning(panning: f32) -> f32 {
        panning.clamp(bounds::PANNING_MIN, bounds::PANNING_MAX)
    }

    /// Check if volume is within valid range
    pub fn is_valid_volume(volume: f32) -> bool {
        volume.is_finite() && (bounds::VOLUME_MIN..=bounds::VOLUME_MAX).contains(&volume)
    }

    /// Check if pitch is within valid range
    pub fn is_valid_pitch(pitch: f32) -> bool {
        pitch.is_finite() && (bounds::PITCH_MIN..=bounds::PITCH_MAX).contains(&pitch)
    }

    /// Check if panning is within valid range
    pub fn is_valid_panning(panning: f32) -> bool {
        panning.is_finite() && (bounds::PANNING_MIN..=bounds::PANNING_MAX).contains(&panning)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn test_clamp_volume() {
            assert_eq!(clamp_volume(-0.5), 0.0);
            assert_eq!(clamp_volume(0.5), 0.5);
            assert_eq!(clamp_volume(1.5), 1.0);
            assert_eq!(clamp_volume(0.0), 0.0);
            assert_eq!(clamp_volume(1.0), 1.0);
        }

        #[test]
        fn test_clamp_pitch() {
            assert_eq!(clamp_pitch(0.05), 0.1);
            assert_eq!(clamp_pitch(2.0), 2.0);
            assert_eq!(clamp_pitch(5.0), 4.0);
            assert_eq!(clamp_pitch(0.1), 0.1);
            assert_eq!(clamp_pitch(4.0), 4.0);
        }

        #[test]
        fn test_clamp_panning() {
            assert_eq!(clamp_panning(-2.0), -1.0);
            assert_eq!(clamp_panning(0.0), 0.0);
            assert_eq!(clamp_panning(2.0), 1.0);
            assert_eq!(clamp_panning(-1.0), -1.0);
            assert_eq!(clamp_panning(1.0), 1.0);
        }

        #[test]
        fn test_validation_functions() {
            assert!(is_valid_volume(0.5));
            assert!(!is_valid_volume(-0.1));
            assert!(!is_valid_volume(1.1));
            assert!(!is_valid_volume(f32::NAN));

            assert!(is_valid_pitch(2.0));
            assert!(!is_valid_pitch(0.05));
            assert!(!is_valid_pitch(5.0));

            assert!(is_valid_panning(0.0));
            assert!(!is_valid_panning(-1.5));
            assert!(!is_valid_panning(1.5));
        }
    }
}
