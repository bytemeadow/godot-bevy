//! Comprehensive audio system for godot-bevy
//!
//! This module provides a powerful audio API that integrates Godot's audio system
//! with Bevy's ECS, offering channels, spatial audio, and smooth transitions.
//!
//! # Example
//! ```no_run
//! use bevy_asset::AssetServer;
//! use bevy_ecs::prelude::{Res, Resource};
//! use bevy_math::Vec2;
//! use godot_bevy::prelude::*;
//!
//! fn setup_audio(
//!     background_music: Res<AudioChannel<BackgroundMusic>>,
//!     sound_effects: Res<AudioChannel<SoundEffects>>,
//!     asset_server: Res<AssetServer>,
//! ) {
//!     background_music
//!         .play(asset_server.load("music/background.ogg"))
//!         .volume(0.8)
//!         .looped();
//!
//!     sound_effects
//!         .play_2d(
//!             asset_server.load("sounds/jump.wav"),
//!             Vec2::new(100.0, 50.0),
//!         )
//!         .volume(0.6)
//!         .pitch(1.2);
//! }
//!
//! #[derive(Resource)]
//! struct BackgroundMusic;
//! impl AudioChannelMarker for BackgroundMusic {
//!     const CHANNEL_NAME: &'static str = "background_music";
//! }
//!
//! #[derive(Resource)]
//! struct SoundEffects;
//! impl AudioChannelMarker for SoundEffects {
//!     const CHANNEL_NAME: &'static str = "sound_effects";
//! }
//! ```

pub mod channel;
pub mod command;
pub mod output;
pub mod player;
pub mod plugin;
pub mod settings;
pub mod tween;

pub use channel::{AudioChannel, AudioChannelMarker, ChannelId, MainAudioTrack, PlayAudioCommand};
pub use command::{AudioCommand, PlayCommand};
pub use output::{ActiveTween, AudioOutput, SoundId, TweenType};
pub use player::AudioPlayerType;
pub use plugin::{AudioApp, AudioError, GodotAudioChannels, GodotAudioPlugin};
pub use settings::AudioSettings;
pub use tween::{AudioEasing, AudioTween};

pub(crate) use channel::ChannelState;

/// Main audio channel type alias for convenience
pub type Audio = AudioChannel<MainAudioTrack>;
