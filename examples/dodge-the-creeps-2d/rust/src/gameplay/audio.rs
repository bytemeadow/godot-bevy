use bevy::prelude::*;
use bevy_kira_audio::prelude::*;

use crate::GameState;

pub struct AudioPlugin;

impl Plugin for AudioPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy_kira_audio::AudioPlugin)
            .init_resource::<GameAudio>()
            .add_systems(Startup, load_audio_assets)
            .add_systems(OnEnter(GameState::Countdown), start_background_music)
            .add_systems(OnExit(GameState::InGame), stop_background_music)
            .add_systems(OnEnter(GameState::GameOver), play_game_over_sound);
    }
}

#[derive(Resource, Default)]
pub struct GameAudio {
    pub background_music: Option<Handle<AudioSource>>,
    pub game_over_sound: Option<Handle<AudioSource>>,
    pub background_music_instance: Option<Handle<AudioInstance>>,
}

fn load_audio_assets(mut commands: Commands, asset_server: Res<AssetServer>) {
    let game_audio = GameAudio {
        background_music: Some(asset_server.load("sounds/House In a Forest Loop.ogg")),
        game_over_sound: Some(asset_server.load("sounds/gameover.wav")),
        background_music_instance: None,
    };

    commands.insert_resource(game_audio);
}

fn start_background_music(mut game_audio: ResMut<GameAudio>, audio: Res<Audio>) {
    if let Some(background_music) = &game_audio.background_music {
        let instance = audio
            .play(background_music.clone())
            .looped()
            .with_volume(0.5)
            .handle();

        game_audio.background_music_instance = Some(instance);
    }
}

fn stop_background_music(
    game_audio: Res<GameAudio>,
    mut audio_instances: ResMut<Assets<AudioInstance>>,
) {
    if let Some(instance_handle) = &game_audio.background_music_instance {
        if let Some(instance) = audio_instances.get_mut(instance_handle) {
            instance.stop(AudioTween::default());
        }
    }
}

fn play_game_over_sound(game_audio: Res<GameAudio>, audio: Res<Audio>) {
    if let Some(game_over_sound) = &game_audio.game_over_sound {
        audio.play(game_over_sound.clone()).with_volume(0.7);
    }
}
