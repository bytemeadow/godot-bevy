use bevy::app::{App, Plugin};
use bevy::prelude::*;
use bevy::state::state::{OnEnter, OnExit};
use bevy_asset_loader::asset_collection::AssetCollection;
use godot_bevy::prelude::{AudioApp, AudioChannel, AudioChannelMarker, GodotResource};

use crate::GameState;

/// Plugin that manages background music and sound effects for the game.
pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_audio_channel::<GameMusicChannel>()
            .add_audio_channel::<GameSfxChannel>()
            .add_systems(OnEnter(GameState::InGame), start_background_music)
            .add_systems(OnEnter(GameState::GameOver), play_game_over_sound)
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
    #[asset(path = "audio/House In a Forest Loop.ogg")]
    pub background_music: Handle<GodotResource>,

    #[asset(path = "audio/gameover.wav")]
    pub game_over_sound: Handle<GodotResource>,
}

/// System that starts background music
fn start_background_music(
    music_channel: Res<AudioChannel<GameMusicChannel>>,
    game_audio: Res<GameAudio>,
) {
    music_channel
        .play(game_audio.background_music.clone())
        .volume(0.5)
        .looped()
        .fade_in(std::time::Duration::from_secs(3));

    info!("Started background music with 3-second fade-in!");
}

/// System that stops background music
fn stop_background_music(music_channel: Res<AudioChannel<GameMusicChannel>>) {
    music_channel.stop();
    info!("Stopped background music");
}

/// System that plays game over sound
fn play_game_over_sound(
    sfx_channel: Res<AudioChannel<GameSfxChannel>>,
    game_audio: Res<GameAudio>,
) {
    sfx_channel
        .play(game_audio.game_over_sound.clone())
        .volume(0.7);

    info!("Played game over sound using new clean API");
}
