use bevy::app::{App, Plugin, Startup, Update};
use bevy::ecs::system::{Res, ResMut};
use bevy::prelude::*;
use bevy::state::condition::in_state;
use bevy::state::state::{OnEnter, OnExit};
use godot_bevy::prelude::{AudioHandle, AudioManager, GodotResourceLoader, SoundId, SoundSettings};

use crate::GameState;

/// Plugin that manages background music and sound effects for the game.
pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<GameAudio>()
            .add_systems(Startup, load_audio_assets)
            .add_systems(OnEnter(GameState::InGame), start_background_music)
            .add_systems(OnEnter(GameState::GameOver), play_game_over_sound)
            .add_systems(OnExit(GameState::InGame), stop_background_music)
            .add_systems(Update, check_music_loop.run_if(in_state(GameState::InGame)));
    }
}

/// Resource similar to Kira's pattern - preloaded audio handles
#[derive(Resource, Default)]
pub struct GameAudio {
    pub background_music: Option<AudioHandle>,
    pub game_over_sound: Option<AudioHandle>,
    pub background_music_instance: Option<SoundId>,
}

/// System that loads and caches audio assets - similar to Kira's pattern
fn load_audio_assets(
    mut audio: ResMut<AudioManager>,
    mut game_audio: ResMut<GameAudio>,
    godot_loader: Res<GodotResourceLoader>,
) {
    info!("Loading audio assets...");

    // Preload background music for efficient reuse
    match audio.load("audio/House In a Forest Loop.ogg", &godot_loader) {
        Ok(handle) => {
            game_audio.background_music = Some(handle);
            info!("Loaded background music");
        }
        Err(e) => warn!("Failed to load background music: {}", e),
    }

    // Preload game over sound
    match audio.load("audio/gameover.wav", &godot_loader) {
        Ok(handle) => {
            game_audio.game_over_sound = Some(handle);
            info!("Loaded game over sound");
        }
        Err(e) => warn!("Failed to load game over sound: {}", e),
    }

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
fn play_game_over_sound(mut audio: ResMut<AudioManager>, game_audio: Res<GameAudio>) {
    if let Some(ref sound_handle) = game_audio.game_over_sound {
        // Play preloaded asset - no loading overhead!
        match audio.play_handle_with_settings(sound_handle, SoundSettings::new().volume(0.7)) {
            Ok(_) => info!("Played game over sound from preloaded asset"),
            Err(e) => warn!("Failed to play game over sound: {}", e),
        }
    } else {
        // Fallback: direct loading (less efficient for repeated sounds)
        match audio.play_with_settings("audio/gameover.wav", SoundSettings::new().volume(0.7)) {
            Ok(_) => info!("Played game over sound with direct loading"),
            Err(e) => warn!("Failed to play game over sound: {}", e),
        }
    }
}

/// System that ensures music keeps looping
fn check_music_loop(mut audio: ResMut<AudioManager>, game_audio: Res<GameAudio>) {
    if let Some(sound_id) = game_audio.background_music_instance {
        // Check if the background music is still playing
        if !audio.is_playing(sound_id) {
            warn!("Background music stopped unexpectedly");
            // Could restart it here if needed
        }
    }
}
