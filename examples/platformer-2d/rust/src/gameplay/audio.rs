//! Audio system for the platformer game with optimized parallelization
//!
//! Audio systems are organized into parallel system sets:
//! - `BackgroundMusic`: Handles level music (uses `GameMusicChannel`)
//! - `SoundEffects`: Handles sound effects (uses `GameSfxChannel`)
//!
//! These sets can run in parallel since they use separate audio channels
//! and have no shared mutable state, improving audio responsiveness.

use crate::GameState;
use crate::level_manager::{LevelId, LevelLoadedMessage};
use bevy::prelude::*;
use bevy::state::state::OnExit;
use bevy_asset_loader::asset_collection::AssetCollection;
use godot_bevy::prelude::{AudioApp, AudioChannel, AudioChannelMarker, GodotResource};

/// Plugin that manages background music and sound effects.
pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_channel::<GameMusicChannel>()
            .add_audio_channel::<GameSfxChannel>()
            .add_observer(on_play_sfx)
            .add_observer(on_level_loaded_play_music)
            .add_systems(OnExit(GameState::InGame), stop_background_music);
    }
}

/// Audio channel for game music
#[derive(Resource)]
pub struct GameMusicChannel;

impl AudioChannelMarker for GameMusicChannel {
    const CHANNEL_NAME: &'static str = "game_music";
}

/// Audio channel for game sound effects
#[derive(Resource)]
pub struct GameSfxChannel;

impl AudioChannelMarker for GameSfxChannel {
    const CHANNEL_NAME: &'static str = "game_sfx";
}

/// Audio assets loaded via bevy_asset_loader
#[derive(AssetCollection, Resource, Debug)]
pub struct GameAudio {
    #[asset(path = "assets/audio/actiontheme-v3.ogg")]
    pub action_theme: Handle<GodotResource>,

    #[asset(path = "assets/audio/annoyingwaltz.wav")]
    pub waltz_theme: Handle<GodotResource>,

    #[asset(path = "assets/audio/jump.wav")]
    pub jump_sound: Handle<GodotResource>,

    #[asset(path = "assets/audio/gem.wav")]
    pub gem_sound: Handle<GodotResource>,
}

/// Event to trigger sound effects
#[derive(Event, Debug, Clone)]
pub enum PlaySfxMessage {
    PlayerJump,
    GemCollected,
}

/// Observer that handles level music changes
fn on_level_loaded_play_music(
    trigger: On<LevelLoadedMessage>,
    music_channel: Res<AudioChannel<GameMusicChannel>>,
    game_audio: Res<GameAudio>,
) {
    let event = trigger.event();

    // Stop current music
    music_channel.stop();

    // Play appropriate music for the level
    let music_handle = match event.level_id {
        LevelId::Level1 | LevelId::Level3 => &game_audio.action_theme,
        LevelId::Level2 => &game_audio.waltz_theme,
    };

    music_channel
        .play(music_handle.clone())
        .volume(0.6)
        .looped()
        .fade_in(std::time::Duration::from_secs(2));

    info!("Started background music for level: {:?}", event.level_id);
}

/// Observer that handles playing sound effects
fn on_play_sfx(
    trigger: On<PlaySfxMessage>,
    sfx_channel: Res<AudioChannel<GameSfxChannel>>,
    game_audio: Res<GameAudio>,
) {
    match trigger.event() {
        PlaySfxMessage::PlayerJump => {
            sfx_channel.play(game_audio.jump_sound.clone()).volume(0.8);
            debug!("Played jump sound effect");
        }
        PlaySfxMessage::GemCollected => {
            sfx_channel.play(game_audio.gem_sound.clone()).volume(0.9);
            debug!("Played gem collection sound effect");
        }
    }
}

/// System that stops background music when exiting the game
fn stop_background_music(music_channel: Res<AudioChannel<GameMusicChannel>>) {
    music_channel.stop();
    info!("Stopped background music");
}
