use bevy::app::{App, Plugin};
use bevy::ecs::system::ResMut;
use bevy::prelude::*;
use bevy::state::state::{OnEnter, OnExit};
use bevy_asset_loader::asset_collection::AssetCollection;
use godot_bevy::prelude::{AudioManager, GodotResource, SoundId, SoundSettings};

use crate::GameState;

/// Plugin that manages background music and sound effects for the game.
pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(OnEnter(GameState::InGame), start_background_music)
            .add_systems(OnEnter(GameState::GameOver), play_game_over_sound)
            .add_systems(OnExit(GameState::InGame), stop_background_music);
    }
}

/// Audio assets loaded via bevy_asset_loader
#[derive(AssetCollection, Resource, Debug)]
pub struct GameAudio {
    #[asset(path = "audio/House In a Forest Loop.ogg")]
    pub background_music: Handle<GodotResource>,

    #[asset(path = "audio/gameover.wav")]
    pub game_over_sound: Handle<GodotResource>,
}

/// Resource to track playing instances
#[derive(Resource, Default)]
pub struct GameAudioState {
    pub background_music_instance: Option<SoundId>,
}

/// System that starts background music using AssetCollection
fn start_background_music(
    mut audio: ResMut<AudioManager>,
    mut audio_state: ResMut<GameAudioState>,
    game_audio: Res<GameAudio>,
) {
    // Play background music with settings - clean and simple!
    let sound_id = audio.play_with_settings(
        game_audio.background_music.clone(),
        SoundSettings::new().volume(0.5).looped(),
    );

    audio_state.background_music_instance = Some(sound_id);
    info!("Started background music from AssetCollection");
}

/// System that stops background music
fn stop_background_music(mut audio: ResMut<AudioManager>, mut audio_state: ResMut<GameAudioState>) {
    if let Some(sound_id) = audio_state.background_music_instance.take() {
        if let Err(e) = audio.stop(sound_id) {
            warn!("Failed to stop background music: {}", e);
        } else {
            info!("Stopped background music");
        }
    }
}

/// System that plays game over sound using AssetCollection
fn play_game_over_sound(mut audio: ResMut<AudioManager>, game_audio: Res<GameAudio>) {
    // Play game over sound - direct and clean!
    let _sound_id = audio.play_with_settings(
        game_audio.game_over_sound.clone(),
        SoundSettings::new().volume(0.7),
    );

    info!("Played game over sound from AssetCollection");
}
