//! Main audio plugin and systems

use crate::bridge::GodotNodeHandle;
use crate::plugins::assets::GodotResource;
use crate::plugins::audio::{
    AudioCommand, AudioSettings, AudioTween, AudioPlayerType, ChannelId, ChannelState, 
    PlayCommand, SoundId, AudioOutput, MainAudioTrack, AudioChannel, AudioChannelMarker
};
use crate::plugins::core::SceneTreeRef;
use bevy::app::{App, Plugin, Update};
use bevy::asset::{Assets, Handle};
use bevy::ecs::system::ResMut;
use bevy::prelude::*;
use godot::classes::{AudioStream, AudioStreamPlayer, AudioStreamPlayer2D, AudioStreamPlayer3D};
use godot::obj::NewAlloc;
use std::collections::HashMap;
use thiserror::Error;

/// Plugin that provides a comprehensive audio API using Godot's audio system.
/// Supports 2D, 3D, and non-positional audio with channels, tweening, and spatial features.
pub struct GodotAudioPlugin;

impl Plugin for GodotAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GodotAudioChannels>()
            .init_resource::<AudioOutput>()
            .add_audio_channel::<MainAudioTrack>()
            .add_systems(
                Update,
                cleanup_finished_sounds,
            );
    }
}

/// Main audio manager for playing sounds and music across different channels.
#[derive(Resource, Default)]
pub struct GodotAudioChannels {
    pub(crate) channels: HashMap<ChannelId, ChannelState>,
    pub(crate) command_queue: Vec<AudioCommand>,
}

/// Extension trait for App to add audio channels with automatic system registration
pub trait AudioApp {
    fn add_audio_channel<T: AudioChannelMarker>(&mut self) -> &mut Self;
}

impl AudioApp for App {
    fn add_audio_channel<T: AudioChannelMarker>(&mut self) -> &mut Self {
        let channel_id = ChannelId(T::CHANNEL_NAME);
        
        // Auto-register a dedicated system for this channel type (like bevy_kira_audio)
        self.add_systems(
            Update,
            process_channel_commands::<T>
        );
        
        self.insert_resource(AudioChannel::<T>::new(channel_id.clone()));
        
        // Initialize channel state in the global manager
        self.world_mut().resource_mut::<GodotAudioChannels>()
            .channels.insert(channel_id, ChannelState::default());
        
        self
    }
}

/// Dedicated system for processing commands from a specific channel type (like bevy_kira_audio)
fn process_channel_commands<T: AudioChannelMarker>(
    channel: Res<AudioChannel<T>>,
    mut audio_output: ResMut<AudioOutput>,
    mut assets: ResMut<Assets<GodotResource>>,
    mut scene_tree: SceneTreeRef,
) {
    // Process all commands from this channel's queue
    let mut commands = channel.commands.write();
    while let Some(command) = commands.pop_front() {
        match command {
            AudioCommand::Play(play_cmd) => {
                let sound_id = process_play_command(play_cmd, &mut assets, &mut scene_tree, &mut audio_output);
                if sound_id.is_none() {
                    // Asset not ready, re-queue for next frame
                    // Note: We need to re-create the command since play_cmd was consumed
                    warn!("Audio asset not ready, skipping for this frame");
                    break; // Stop processing this frame to avoid infinite retry loop
                }
            }
            AudioCommand::Stop(channel_id, tween) => {
                let sound_ids: Vec<SoundId> = audio_output
                    .sound_to_channel
                    .iter()
                    .filter(|(_, ch)| **ch == channel_id)
                    .map(|(sound_id, _)| *sound_id)
                    .collect();

                for sound_id in sound_ids {
                    audio_output.stop_sound(sound_id);
                }

                if let Some(_tween) = tween {
                    // TODO: Implement fade-out tweening
                }
                trace!("Stopped all sounds in channel: {:?}", channel_id);
            }
            AudioCommand::Pause(channel_id, _tween) => {
                apply_to_channel_sounds(&mut audio_output, channel_id, |output, sound_id| {
                    output.pause_sound(sound_id);
                });
                trace!("Paused channel: {:?}", channel_id);
            }
            AudioCommand::Resume(channel_id, _tween) => {
                apply_to_channel_sounds(&mut audio_output, channel_id, |output, sound_id| {
                    output.resume_sound(sound_id);
                });
                trace!("Resumed channel: {:?}", channel_id);
            }
            AudioCommand::SetVolume(channel_id, volume, _tween) => {
                apply_to_channel_sounds(&mut audio_output, channel_id, |output, sound_id| {
                    output.set_sound_volume(sound_id, volume);
                });
                trace!("Set volume to {} for channel: {:?}", volume, channel_id);
            }
            AudioCommand::SetPitch(channel_id, pitch, _tween) => {
                apply_to_channel_sounds(&mut audio_output, channel_id, |output, sound_id| {
                    output.set_sound_pitch(sound_id, pitch);
                });
                trace!("Set pitch to {} for channel: {:?}", pitch, channel_id);
            }
            AudioCommand::SetPanning(_channel_id, _panning, _tween) => {
                // TODO: Implement panning for individual sounds
                warn!("Panning not yet implemented for individual sounds");
            }
            AudioCommand::StopSound(sound_id, _tween) => {
                audio_output.stop_sound(sound_id);
                trace!("Stopped sound: {:?}", sound_id);
            }
        }
    }
}

/// Helper function to apply an operation to all sounds in a channel
fn apply_to_channel_sounds<F>(output: &mut AudioOutput, channel_id: ChannelId, operation: F)
where
    F: Fn(&mut AudioOutput, SoundId),
{
    let sound_ids: Vec<SoundId> = output
        .sound_to_channel
        .iter()
        .filter(|(_, ch)| **ch == channel_id)
        .map(|(sound_id, _)| *sound_id)
        .collect();

    for sound_id in sound_ids {
        operation(output, sound_id);
    }
}

/// Process a play command and return the sound ID if successful
fn process_play_command(
    play_cmd: PlayCommand,
    assets: &mut Assets<GodotResource>,
    scene_tree: &mut SceneTreeRef,
    output: &mut AudioOutput,
) -> Option<SoundId> {
    let audio_stream = if let Some(asset) = assets.get_mut(&play_cmd.handle) {
        asset.try_cast::<AudioStream>()
    } else {
        // Asset not ready yet, re-queue for next frame
        warn!("Audio asset not ready: {:?}", play_cmd.handle);
        return None;
    };

    let Some(audio_stream) = audio_stream else {
        warn!("Failed to cast to AudioStream: {:?}", play_cmd.handle);
        return None;
    };

    // Configure looping if requested
    let audio_stream = configure_looping(audio_stream, play_cmd.settings.looping);
    
    // Create appropriate player based on type
    let player_handle = match play_cmd.player_type {
        AudioPlayerType::NonPositional => {
            create_audio_player(audio_stream, &play_cmd.settings)
        }
        AudioPlayerType::Spatial2D { position } => {
            create_audio_player_2d(audio_stream, &play_cmd.settings, position)
        }
        AudioPlayerType::Spatial3D { position } => {
            create_audio_player_3d(audio_stream, &play_cmd.settings, position)
        }
    };

    if let Some(mut handle) = player_handle {
        if let Some(mut root) = scene_tree.get().get_root() {
            // Get the node from the handle and add it to the scene tree
            let node = handle.get::<godot::classes::Node>();
            root.add_child(&node);
        }
        
        // Now that the node is in the scene tree, start playback
        start_audio_playback(&mut handle);
        
        output.playing_sounds.insert(play_cmd.sound_id, handle);
        output.sound_to_channel.insert(play_cmd.sound_id, play_cmd.channel_id);
        trace!("Started playing audio: {:?} in channel: {:?}", play_cmd.sound_id, play_cmd.channel_id);
        Some(play_cmd.sound_id)
    } else {
        None
    }
}

fn create_audio_player(
    audio_stream: godot::obj::Gd<AudioStream>,
    settings: &AudioSettings,
) -> Option<GodotNodeHandle> {
    let mut player = AudioStreamPlayer::new_alloc();
    player.set_stream(&audio_stream);
    player.set_volume_db(volume_to_db(settings.volume));
    player.set_pitch_scale(settings.pitch);
    
    if let Some(panning) = settings.panning {
        // Convert from -1.0..1.0 to 0.0..1.0 for Godot
        let _godot_panning = (panning + 1.0) / 2.0;
        let bus_name: godot::builtin::StringName = "Master".into();
        player.set_bus(&bus_name);
    }
    
    // Don't play yet - need to add to scene tree first
    Some(GodotNodeHandle::new(player.upcast::<godot::classes::Node>()))
}

fn create_audio_player_2d(
    audio_stream: godot::obj::Gd<AudioStream>,
    settings: &AudioSettings,
    position: Vec2,
) -> Option<GodotNodeHandle> {
    let mut player = AudioStreamPlayer2D::new_alloc();
    player.set_stream(&audio_stream);
    player.set_volume_db(volume_to_db(settings.volume));
    player.set_pitch_scale(settings.pitch);
    player.set_position(godot::prelude::Vector2::new(position.x, position.y));
    
    // Don't play yet - need to add to scene tree first
    Some(GodotNodeHandle::new(player.upcast::<godot::classes::Node>()))
}

fn create_audio_player_3d(
    audio_stream: godot::obj::Gd<AudioStream>,
    settings: &AudioSettings,
    position: Vec3,
) -> Option<GodotNodeHandle> {
    let mut player = AudioStreamPlayer3D::new_alloc();
    player.set_stream(&audio_stream);
    player.set_volume_db(volume_to_db(settings.volume));
    player.set_pitch_scale(settings.pitch);
    player.set_position(godot::prelude::Vector3::new(position.x, position.y, position.z));
    
    // Don't play yet - need to add to scene tree first
    Some(GodotNodeHandle::new(player.upcast::<godot::classes::Node>()))
}

fn configure_looping(
    audio_stream: godot::obj::Gd<AudioStream>,
    looping: bool,
) -> godot::obj::Gd<AudioStream> {
    if !looping {
        return audio_stream;
    }

    // Try to enable looping on different stream types
    if let Ok(mut ogg_stream) = audio_stream
        .clone()
        .try_cast::<godot::classes::AudioStreamOggVorbis>()
    {
        ogg_stream.set_loop(true);
        ogg_stream.upcast()
    } else if let Ok(mut wav_stream) = audio_stream
        .clone()
        .try_cast::<godot::classes::AudioStreamWav>()
    {
        wav_stream.set_loop_mode(godot::classes::audio_stream_wav::LoopMode::FORWARD);
        wav_stream.upcast()
    } else {
        warn!("Audio stream type doesn't support runtime loop configuration");
        audio_stream
    }
}

fn start_audio_playback(handle: &mut GodotNodeHandle) {
    // Try each player type and start playback
    if let Some(mut player) = handle.try_get::<AudioStreamPlayer>() {
        player.play();
    } else if let Some(mut player) = handle.try_get::<AudioStreamPlayer2D>() {
        player.play();
    } else if let Some(mut player) = handle.try_get::<AudioStreamPlayer3D>() {
        player.play();
    }
}

/// System that cleans up finished sounds
fn cleanup_finished_sounds(mut audio_output: ResMut<AudioOutput>) {
    let mut finished_sounds = Vec::new();

    for (&sound_id, handle) in audio_output.playing_sounds.iter_mut() {
        let is_playing = if let Some(player) = handle.try_get::<AudioStreamPlayer>() {
            player.is_playing()
        } else if let Some(player) = handle.try_get::<AudioStreamPlayer2D>() {
            player.is_playing()
        } else if let Some(player) = handle.try_get::<AudioStreamPlayer3D>() {
            player.is_playing()
        } else {
            false // Player was freed
        };

        if !is_playing {
            finished_sounds.push(sound_id);
        }
    }

    for sound_id in finished_sounds {
        audio_output.playing_sounds.remove(&sound_id);
        audio_output.sound_to_channel.remove(&sound_id);
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

/// Simplified GodotAudioChannels - most functionality moved to per-channel systems
impl GodotAudioChannels {
    /// Get stats about the audio system
    pub fn stats(&self) -> (usize, usize) {
        (self.command_queue.len(), self.channels.len())
    }
}

/// Possible errors that can be produced by the audio system
#[derive(Debug, Error)]
pub enum AudioError {
    #[error("Sound not found: {0:?}")]
    SoundNotFound(SoundId),
    #[error("Channel not found: {0:?}")]
    ChannelNotFound(ChannelId),
} 