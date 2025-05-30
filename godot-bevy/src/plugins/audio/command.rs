//! Audio command system for deferred execution

use crate::plugins::assets::GodotResource;
use crate::plugins::audio::{AudioPlayerType, AudioSettings, AudioTween, ChannelId, SoundId};
use bevy::asset::Handle;

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
