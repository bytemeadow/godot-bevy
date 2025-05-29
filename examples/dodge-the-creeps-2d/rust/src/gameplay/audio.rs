use bevy::app::{App, Plugin, Startup};
use bevy::asset::AssetServer;
use bevy::ecs::system::{Res, ResMut};
use bevy::prelude::*;
use bevy::state::state::{OnEnter, OnExit};
use godot_bevy::prelude::{AudioHandle, AudioManager, SoundId, SoundSettings};

use crate::GameState;

/// Plugin that manages background music and sound effects for the game.
pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameAudio>()
            .add_systems(Startup, load_audio_assets)
            .add_systems(OnEnter(GameState::InGame), start_background_music)
            .add_systems(OnEnter(GameState::GameOver), play_game_over_sound)
            .add_systems(OnExit(GameState::InGame), stop_background_music);
    }
}

/// Resource similar to Kira's pattern - preloaded audio handles
#[derive(Resource, Default)]
pub struct GameAudio {
    pub background_music: Option<AudioHandle>,
    pub game_over_sound: Option<AudioHandle>,
    pub background_music_instance: Option<SoundId>,
}

/// System that loads and caches audio assets using Bevy's asset system
fn load_audio_assets(
    mut audio: ResMut<AudioManager>,
    mut game_audio: ResMut<GameAudio>,
    asset_server: Res<AssetServer>,
) {
    info!("Loading audio assets...");

    // Preload background music for efficient reuse using Bevy's asset system
    let background_handle = audio.load("audio/House In a Forest Loop.ogg", &asset_server);
    game_audio.background_music = Some(background_handle);
    info!("Loaded background music");

    // Preload game over sound
    let gameover_handle = audio.load("audio/gameover.wav", &asset_server);
    game_audio.game_over_sound = Some(gameover_handle);
    info!("Loaded game over sound");

    info!("Audio assets loaded");
}

/// System that starts background music using preloaded assets
fn start_background_music(mut audio: ResMut<AudioManager>, mut game_audio: ResMut<GameAudio>) {
    if let Some(ref music_handle) = game_audio.background_music {
        // Play preloaded asset with settings - no loading overhead!
        match audio
            .play_handle_with_settings(music_handle, SoundSettings::new().volume(0.5).looped())
        {
            Ok(sound_id) => {
                game_audio.background_music_instance = Some(sound_id);
                info!("Started background music from preloaded asset");
            }
            Err(e) => {
                warn!("Failed to start background music: {}", e);
            }
        }
    } else {
        warn!("Background music not loaded");
    }
}

/// System that stops background music
fn stop_background_music(mut audio: ResMut<AudioManager>, mut game_audio: ResMut<GameAudio>) {
    if let Some(sound_id) = game_audio.background_music_instance.take() {
        if let Err(e) = audio.stop(sound_id) {
            warn!("Failed to stop background music: {}", e);
        } else {
            info!("Stopped background music");
        }
    }
}

/// System that plays game over sound using preloaded asset
fn play_game_over_sound(
    mut audio: ResMut<AudioManager>, 
    game_audio: Res<GameAudio>,
    asset_server: Res<AssetServer>,
) {
    if let Some(ref sound_handle) = game_audio.game_over_sound {
        // Play preloaded asset - no loading overhead!
        match audio.play_handle_with_settings(sound_handle, SoundSettings::new().volume(0.7)) {
            Ok(_) => info!("Played game over sound from preloaded asset"),
            Err(e) => warn!("Failed to play game over sound: {}", e),
        }
    } else {
        // Fallback: direct loading using asset server (async)
        match audio.play_with_settings("audio/gameover.wav", SoundSettings::new().volume(0.7), &asset_server) {
            Ok(_) => info!("Played game over sound with async loading"),
            Err(e) => warn!("Failed to play game over sound: {}", e),
        }
    }
}
