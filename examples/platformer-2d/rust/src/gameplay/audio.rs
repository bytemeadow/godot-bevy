//! Audio system for the platformer game with optimized parallelization
//!
//! Audio systems are organized into parallel system sets:
//! - `BackgroundMusic`: Handles level music (uses `GameMusicChannel`)
//! - `SoundEffects`: Handles sound effects (uses `GameSfxChannel`)
//!
//! These sets can run in parallel since they use separate audio channels
//! and have no shared mutable state, improving audio responsiveness.

use bevy::prelude::*;
use bevy::state::condition::in_state;
use bevy::state::state::OnExit;
use bevy_asset_loader::asset_collection::AssetCollection;
use godot_bevy::prelude::{AudioApp, AudioChannel, AudioChannelMarker, GodotResource};

use crate::level_manager::{LevelId, LevelLoadedEvent};
use crate::GameState;

/// System sets for audio operations that can run in parallel
#[derive(SystemSet, Debug, Clone, PartialEq, Eq, Hash)]
pub enum AudioSystemSet {
    /// Background music management (independent of SFX)
    BackgroundMusic,
    /// Sound effects management (independent of music)
    SoundEffects,
}

/// Plugin that manages background music and sound effects with parallel system sets.
pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_channel::<GameMusicChannel>()
            .add_audio_channel::<GameSfxChannel>()
            .add_event::<PlaySfxEvent>()
            .add_systems(
                Update,
                (
                    // Music systems run independently of SFX systems
                    handle_level_music_change,
                )
                    .run_if(in_state(GameState::InGame))
                    .in_set(AudioSystemSet::BackgroundMusic),
            )
            .add_systems(
                Update,
                (
                    // SFX systems run independently of music systems
                    handle_sfx_events,
                )
                    .run_if(in_state(GameState::InGame))
                    .in_set(AudioSystemSet::SoundEffects),
            )
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
pub enum PlaySfxEvent {
    PlayerJump,
    GemCollected,
}

/// System that handles level music changes
///
/// Runs in `BackgroundMusic` set and can execute in parallel with SFX systems
/// since it uses a separate audio channel (`GameMusicChannel`).
fn handle_level_music_change(
    mut level_loaded_events: EventReader<LevelLoadedEvent>,
    music_channel: Res<AudioChannel<GameMusicChannel>>,
    game_audio: Res<GameAudio>,
) {
    for event in level_loaded_events.read() {
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
}

/// System that handles playing sound effects
///
/// Runs in `SoundEffects` set and can execute in parallel with music systems
/// since it uses a separate audio channel (`GameSfxChannel`).
fn handle_sfx_events(
    mut sfx_events: EventReader<PlaySfxEvent>,
    sfx_channel: Res<AudioChannel<GameSfxChannel>>,
    game_audio: Res<GameAudio>,
) {
    for event in sfx_events.read() {
        match event {
            PlaySfxEvent::PlayerJump => {
                sfx_channel.play(game_audio.jump_sound.clone()).volume(0.8);
                debug!("Played jump sound effect");
            }
            PlaySfxEvent::GemCollected => {
                sfx_channel.play(game_audio.gem_sound.clone()).volume(0.9);
                debug!("Played gem collection sound effect");
            }
        }
    }
}

/// System that stops background music when exiting the game
fn stop_background_music(music_channel: Res<AudioChannel<GameMusicChannel>>) {
    music_channel.stop();
    info!("Stopped background music");
}
