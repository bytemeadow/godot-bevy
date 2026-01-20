//! Main audio plugin and systems
use crate::interop::{GodotAccess, GodotNodeHandle};
use crate::plugins::assets::GodotResource;
use crate::plugins::audio::output::{
    AudioPlayer, stop_and_free_audio_player, try_get_audio_player,
};
use crate::plugins::audio::{
    ActiveTween, AudioChannel, AudioChannelMarker, AudioCommand, AudioOutput, AudioPlayerType,
    AudioSettings, ChannelId, ChannelState, MainAudioTrack, PlayCommand, SoundId, TweenType,
};
use crate::plugins::scene_tree::SceneTreeRef;
use bevy_app::{App, Plugin, Update};
use bevy_asset::Assets;
use bevy_ecs::prelude::Resource;
use bevy_ecs::schedule::{IntoScheduleConfigs, SystemSet};
use bevy_ecs::system::{Res, ResMut};
use bevy_math::{Vec2, Vec3};
use bevy_time::Time;
use godot::classes::{AudioStream, AudioStreamPlayer, AudioStreamPlayer2D, AudioStreamPlayer3D};
use godot::obj::NewAlloc;
use std::collections::{HashMap, VecDeque};
use thiserror::Error;
use tracing::{trace, warn};

/// Plugin that provides a comprehensive audio API using Godot's audio system.
/// Supports 2D, 3D, and non-positional audio with channels, tweening, and spatial features.
#[derive(Default)]
pub struct GodotAudioPlugin;

#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum AudioSystemSet {
    CollectCommands,
    ProcessCommands,
}

impl Plugin for GodotAudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GodotAudioChannels>()
            .init_resource::<AudioOutput>()
            .add_audio_channel::<MainAudioTrack>()
            .configure_sets(
                Update,
                AudioSystemSet::ProcessCommands.after(AudioSystemSet::CollectCommands),
            )
            .add_systems(
                Update,
                audio_main_thread.in_set(AudioSystemSet::ProcessCommands),
            );
    }
}

/// Main audio manager for playing sounds and music across different channels.
#[derive(Resource, Default)]
pub struct GodotAudioChannels {
    pub(crate) channels: HashMap<ChannelId, ChannelState>,
    pub(crate) command_queue: VecDeque<AudioCommand>,
}

/// Extension trait for App to add audio channels with automatic system registration
pub trait AudioApp {
    fn add_audio_channel<T: AudioChannelMarker>(&mut self) -> &mut Self;
}

impl AudioApp for App {
    fn add_audio_channel<T: AudioChannelMarker>(&mut self) -> &mut Self {
        let channel_id = ChannelId(T::CHANNEL_NAME);

        // Auto-register a dedicated system for this channel type
        self.add_systems(
            Update,
            process_channel_commands::<T>.in_set(AudioSystemSet::CollectCommands),
        );

        self.insert_resource(AudioChannel::<T>::new(channel_id));

        // Initialize channel state in the global manager
        self.world_mut()
            .resource_mut::<GodotAudioChannels>()
            .channels
            .insert(channel_id, ChannelState::default());

        self
    }
}

/// Dedicated system for processing commands from a specific channel type
fn process_channel_commands<T: AudioChannelMarker>(
    channel: Res<AudioChannel<T>>,
    mut audio_channels: ResMut<GodotAudioChannels>,
) {
    // Collect commands into the global queue so a single system can apply Godot calls.
    let mut commands = channel.commands.write();
    while let Some(command) = commands.pop_front() {
        audio_channels.command_queue.push_back(command);
    }
}

#[derive(Default)]
struct PendingSoundOps {
    volume: Option<f32>,
    pitch: Option<f32>,
    paused: Option<bool>,
}

/// System that applies queued audio commands using Godot APIs.
fn audio_main_thread(
    mut audio_channels: ResMut<GodotAudioChannels>,
    mut audio_output: ResMut<AudioOutput>,
    mut assets: ResMut<Assets<GodotResource>>,
    mut scene_tree: SceneTreeRef,
    time: Res<Time>,
    mut godot: GodotAccess,
) {
    let mut pending_ops: HashMap<SoundId, PendingSoundOps> = HashMap::new();
    let mut pending_stops: Vec<(SoundId, GodotNodeHandle)> = Vec::new();

    while let Some(command) = audio_channels.command_queue.pop_front() {
        match command {
            AudioCommand::Play(play_cmd) => {
                if process_play_command(
                    &play_cmd,
                    &mut assets,
                    &mut scene_tree,
                    &mut audio_output,
                    &mut godot,
                )
                .is_none()
                {
                    // Asset not ready, re-queue for next frame
                    audio_channels
                        .command_queue
                        .push_front(AudioCommand::Play(play_cmd));
                    warn!("Audio asset not ready, re-queued for next frame");
                    break; // Stop processing this frame to avoid infinite retry loop
                }
            }
            AudioCommand::Stop(channel_id, tween) => {
                let sound_ids = collect_channel_sound_ids(&audio_output, channel_id);

                if let Some(tween) = tween {
                    // Implement fade-out tweening with real current volumes
                    for sound_id in sound_ids {
                        // Get the actual current volume instead of assuming 1.0
                        let current_volume = audio_output
                            .current_volumes
                            .get(&sound_id)
                            .copied()
                            .unwrap_or(1.0);
                        let fade_out_tween =
                            ActiveTween::new_fade_out(current_volume, tween.clone());
                        audio_output.active_tweens.insert(sound_id, fade_out_tween);
                        trace!(
                            "Started fade-out from volume {} for sound: {:?}",
                            current_volume, sound_id
                        );
                    }
                } else {
                    // Immediate stop
                    for sound_id in sound_ids {
                        schedule_stop_sound(
                            &mut audio_output,
                            &mut pending_ops,
                            &mut pending_stops,
                            sound_id,
                        );
                    }
                }
                trace!("Processed stop command for channel: {:?}", channel_id);
            }
            AudioCommand::Pause(channel_id, _tween) => {
                let sound_ids = collect_channel_sound_ids(&audio_output, channel_id);
                for sound_id in sound_ids {
                    pending_ops.entry(sound_id).or_default().paused = Some(true);
                }
                trace!("Paused channel: {:?}", channel_id);
            }
            AudioCommand::Resume(channel_id, _tween) => {
                let sound_ids = collect_channel_sound_ids(&audio_output, channel_id);
                for sound_id in sound_ids {
                    pending_ops.entry(sound_id).or_default().paused = Some(false);
                }
                trace!("Resumed channel: {:?}", channel_id);
            }
            AudioCommand::SetVolume(channel_id, volume, _tween) => {
                let sound_ids = collect_channel_sound_ids(&audio_output, channel_id);
                for sound_id in sound_ids {
                    audio_output.current_volumes.insert(sound_id, volume);
                    pending_ops.entry(sound_id).or_default().volume = Some(volume);
                }
                trace!("Set volume to {} for channel: {:?}", volume, channel_id);
            }
            AudioCommand::SetPitch(channel_id, pitch, _tween) => {
                let sound_ids = collect_channel_sound_ids(&audio_output, channel_id);
                for sound_id in sound_ids {
                    pending_ops.entry(sound_id).or_default().pitch = Some(pitch);
                }
                trace!("Set pitch to {} for channel: {:?}", pitch, channel_id);
            }
            AudioCommand::SetPanning(_channel_id, _panning, _tween) => {
                // TODO: Implement panning for individual sounds
                warn!("Panning not yet implemented for individual sounds");
            }
            AudioCommand::StopSound(sound_id, _tween) => {
                schedule_stop_sound(
                    &mut audio_output,
                    &mut pending_ops,
                    &mut pending_stops,
                    sound_id,
                );
                trace!("Stopped sound: {:?}", sound_id);
            }
        }
    }

    let delta = time.delta();
    let mut completed_tweens = Vec::new();
    let mut sounds_to_stop = Vec::new();
    let mut volume_updates = Vec::new();
    let mut pitch_updates = Vec::new();

    // First pass: update tweens and collect parameter changes
    for (&sound_id, tween) in audio_output.active_tweens.iter_mut() {
        let current_value = tween.update(delta);

        match tween.tween_type {
            TweenType::Volume | TweenType::FadeOut => {
                volume_updates.push((sound_id, current_value));
            }
            TweenType::Pitch => {
                pitch_updates.push((sound_id, current_value));
            }
        }

        if tween.is_complete() {
            completed_tweens.push(sound_id);

            // If this was a fade-out, mark sound for removal
            if matches!(tween.tween_type, TweenType::FadeOut) {
                sounds_to_stop.push(sound_id);
            }
        }
    }

    for (sound_id, volume) in volume_updates {
        if audio_output.playing_sounds.contains_key(&sound_id) {
            audio_output.current_volumes.insert(sound_id, volume);
            pending_ops.entry(sound_id).or_default().volume = Some(volume);
        }
    }

    for (sound_id, pitch) in pitch_updates {
        if audio_output.playing_sounds.contains_key(&sound_id) {
            pending_ops.entry(sound_id).or_default().pitch = Some(pitch);
        }
    }

    for sound_id in completed_tweens {
        audio_output.active_tweens.remove(&sound_id);
        trace!("Completed tween for sound: {:?}", sound_id);
    }

    for sound_id in sounds_to_stop {
        schedule_stop_sound(
            &mut audio_output,
            &mut pending_ops,
            &mut pending_stops,
            sound_id,
        );
        trace!("Stopped sound after fade-out: {:?}", sound_id);
    }

    for (sound_id, handle) in pending_stops {
        stop_and_free_audio_player(&mut godot, handle);
        trace!("Stopped sound: {:?}", sound_id);
    }

    let playing_sounds: Vec<(SoundId, GodotNodeHandle)> = audio_output
        .playing_sounds
        .iter()
        .map(|(sound_id, handle)| (*sound_id, *handle))
        .collect();

    let mut finished_sounds = Vec::new();
    for (sound_id, handle) in playing_sounds {
        let Some(mut player) = try_get_audio_player(&mut godot, handle) else {
            finished_sounds.push(sound_id);
            continue;
        };

        if let Some(ops) = pending_ops.get(&sound_id) {
            apply_pending_ops(&mut player, ops);
        }

        let is_playing = player.is_playing();
        if !is_playing {
            let mut node = player.into_node();
            if let Some(mut parent) = node.get_parent() {
                parent.remove_child(&node);
            }
            node.queue_free();
            finished_sounds.push(sound_id);
        }
    }

    for sound_id in finished_sounds {
        audio_output.playing_sounds.remove(&sound_id);
        audio_output.sound_to_channel.remove(&sound_id);
        audio_output.active_tweens.remove(&sound_id);
        audio_output.current_volumes.remove(&sound_id);
        trace!("Cleaned up finished sound: {:?}", sound_id);
    }
}

fn collect_channel_sound_ids(output: &AudioOutput, channel_id: ChannelId) -> Vec<SoundId> {
    output
        .sound_to_channel
        .iter()
        .filter(|(_, ch)| **ch == channel_id)
        .map(|(sound_id, _)| *sound_id)
        .collect()
}

fn schedule_stop_sound(
    output: &mut AudioOutput,
    pending_ops: &mut HashMap<SoundId, PendingSoundOps>,
    pending_stops: &mut Vec<(SoundId, GodotNodeHandle)>,
    sound_id: SoundId,
) {
    if let Some(handle) = output.playing_sounds.remove(&sound_id) {
        output.sound_to_channel.remove(&sound_id);
        output.current_volumes.remove(&sound_id);
        pending_ops.remove(&sound_id);
        pending_stops.push((sound_id, handle));
    }
}

fn apply_pending_ops(player: &mut AudioPlayer, ops: &PendingSoundOps) {
    if let Some(volume) = ops.volume {
        player.set_volume_db(volume_to_db(volume));
    }
    if let Some(pitch) = ops.pitch {
        player.set_pitch_scale(pitch);
    }
    if let Some(paused) = ops.paused {
        player.set_stream_paused(paused);
    }
}

/// Process a play command and return the sound ID if successful
fn process_play_command(
    play_cmd: &PlayCommand,
    assets: &mut Assets<GodotResource>,
    scene_tree: &mut SceneTreeRef,
    output: &mut AudioOutput,
    godot: &mut GodotAccess,
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

    // Check if fade-in is needed
    let (initial_volume, fade_in_tween) = if let Some(fade_in) = &play_cmd.settings.fade_in {
        (0.0, Some((play_cmd.settings.volume, fade_in.clone())))
    } else {
        (play_cmd.settings.volume, None)
    };

    // Create settings with initial volume for fade-in
    let mut initial_settings = play_cmd.settings.clone();
    initial_settings.volume = initial_volume;

    // Create appropriate player based on type
    let player_handle = match &play_cmd.player_type {
        AudioPlayerType::NonPositional => create_audio_player(audio_stream, &initial_settings),
        AudioPlayerType::Spatial2D { position } => {
            create_audio_player_2d(audio_stream, &initial_settings, *position)
        }
        AudioPlayerType::Spatial3D { position } => {
            create_audio_player_3d(audio_stream, &initial_settings, *position)
        }
    };

    if let Some(handle) = player_handle {
        if let Some(mut root) = scene_tree.get().get_root() {
            // Get the node from the handle and add it to the scene tree
            let node = godot.get::<godot::classes::Node>(handle);
            root.add_child(&node);
        }

        // Now that the node is in the scene tree, start playback
        start_audio_playback(godot, handle);

        output.playing_sounds.insert(play_cmd.sound_id, handle);
        output
            .sound_to_channel
            .insert(play_cmd.sound_id, play_cmd.channel_id);

        // Track initial volume (either fade-in start volume or target volume)
        let initial_volume = if fade_in_tween.is_some() {
            0.0
        } else {
            initial_settings.volume
        };
        output
            .current_volumes
            .insert(play_cmd.sound_id, initial_volume);

        // Set up fade-in tween if needed
        if let Some((target_volume, fade_in)) = fade_in_tween {
            let tween = ActiveTween::new_fade_in(target_volume, fade_in);
            output.active_tweens.insert(play_cmd.sound_id, tween);
            trace!("Started fade-in for sound: {:?}", play_cmd.sound_id);
        }

        trace!(
            "Started playing audio: {:?} in channel: {:?}",
            play_cmd.sound_id, play_cmd.channel_id
        );
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
    Some(GodotNodeHandle::new(
        player.upcast::<godot::classes::Node>(),
    ))
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
    Some(GodotNodeHandle::new(
        player.upcast::<godot::classes::Node>(),
    ))
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
    player.set_position(godot::prelude::Vector3::new(
        position.x, position.y, position.z,
    ));

    // Don't play yet - need to add to scene tree first
    Some(GodotNodeHandle::new(
        player.upcast::<godot::classes::Node>(),
    ))
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

fn start_audio_playback(godot: &mut GodotAccess, handle: GodotNodeHandle) {
    // Try each player type and start playback
    if let Some(mut player) = godot.try_get::<AudioStreamPlayer>(handle) {
        player.play();
    } else if let Some(mut player) = godot.try_get::<AudioStreamPlayer2D>(handle) {
        player.play();
    } else if let Some(mut player) = godot.try_get::<AudioStreamPlayer3D>(handle) {
        player.play();
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
