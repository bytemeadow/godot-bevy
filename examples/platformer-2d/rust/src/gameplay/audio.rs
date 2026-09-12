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

fn on_level_loaded_play_music(
    trigger: On<LevelLoadedMessage>,
    music_channel: Res<AudioChannel<GameMusicChannel>>,
    game_audio: Res<GameAudio>,
) {
    #[cfg(feature = "capture-audio")]
    if crate::capture_audio::is_active() {
        return;
    }

    let event = trigger.event();

    music_channel.stop();

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

fn on_play_sfx(
    trigger: On<PlaySfxMessage>,
    sfx_channel: Res<AudioChannel<GameSfxChannel>>,
    game_audio: Res<GameAudio>,
) {
    #[cfg(feature = "capture-audio")]
    if crate::capture_audio::is_active() {
        return;
    }

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

fn stop_background_music(music_channel: Res<AudioChannel<GameMusicChannel>>) {
    music_channel.stop();
    info!("Stopped background music");
}

#[cfg(feature = "capture-audio")]
pub(crate) fn reset_capture_audio(world: &World) {
    let music = world.resource::<AudioChannel<GameMusicChannel>>();
    music.stop();
    music.set_volume(0.0);
    let sfx = world.resource::<AudioChannel<GameSfxChannel>>();
    sfx.stop();
    sfx.resume();
    sfx.set_volume(1.0);
    sfx.set_pitch(1.0);
    sfx.set_panning(0.0);
}

#[cfg(feature = "capture-audio")]
pub(crate) fn play_capture_cue(world: &World, name: &str) -> Result<(), String> {
    let assets = world.resource::<GameAudio>();
    let handle = match name {
        "jump" => &assets.jump_sound,
        "gem" => &assets.gem_sound,
        _ => return Err(format!("unknown audio cue {name}")),
    };
    world
        .resource::<AudioChannel<GameSfxChannel>>()
        .play(handle.clone())
        .volume(0.8)
        .pitch(1.0)
        .panning(0.0);
    Ok(())
}
