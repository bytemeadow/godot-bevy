//! Audio command system for deferred execution

use crate::plugins::assets::GodotResource;
use crate::plugins::audio::{AudioPlayerType, AudioSettings, AudioTween, ChannelId, SoundId};
use bevy::asset::Handle;
use std::time::Duration;

/// Internal command for the audio system (channel-wide operations only)
#[derive(Debug)]
pub enum AudioCommand {
    Play(PlayCommand),
    Stop(ChannelId, Option<AudioTween>),
    Pause(ChannelId, Option<AudioTween>),
    Resume(ChannelId, Option<AudioTween>),
    SetVolume(ChannelId, f32, Option<AudioTween>),
    SetPitch(ChannelId, f32, Option<AudioTween>),
    SetPanning(ChannelId, f32, Option<AudioTween>),
    StopSound(SoundId, Option<AudioTween>),
}

/// Command to play audio with specific settings
#[derive(Debug)]
pub struct PlayCommand {
    pub channel_id: ChannelId,
    pub handle: Handle<GodotResource>,
    pub player_type: AudioPlayerType,
    pub settings: AudioSettings,
    pub sound_id: SoundId,
}

/// Fluent builder for playing audio with configurable settings
pub struct PlayAudioCommand<'a> {
    channel_id: ChannelId,
    handle: Handle<GodotResource>,
    player_type: AudioPlayerType,
    settings: AudioSettings,
    audio_manager: &'a mut super::GodotAudioChannels,
    output: &'a mut super::AudioOutput,
}

impl<'a> PlayAudioCommand<'a> {
    pub(crate) fn new(
        channel_id: ChannelId,
        handle: Handle<GodotResource>,
        player_type: AudioPlayerType,
        audio_manager: &'a mut super::GodotAudioChannels,
        output: &'a mut super::AudioOutput,
    ) -> Self {
        Self {
            channel_id,
            handle,
            player_type,
            settings: AudioSettings::default(),
            audio_manager,
            output,
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
    pub fn fade_in(mut self, duration: Duration) -> Self {
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

    /// Execute the play command and return a sound ID for later control
    pub fn play(self) -> SoundId {
        let sound_id = SoundId(self.output.next_sound_id);
        self.output.next_sound_id += 1;

        let command = AudioCommand::Play(PlayCommand {
            channel_id: self.channel_id,
            handle: self.handle,
            player_type: self.player_type,
            settings: self.settings,
            sound_id,
        });

        self.audio_manager.command_queue.push(command);
        sound_id
    }
}
