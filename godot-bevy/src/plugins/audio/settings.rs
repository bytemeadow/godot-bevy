//! Audio settings and configuration

use crate::plugins::audio::AudioTween;

/// Settings for playing audio
#[derive(Debug, Clone)]
pub struct AudioSettings {
    pub volume: f32,
    pub pitch: f32,
    pub looping: bool,
    pub fade_in: Option<AudioTween>,
    pub start_position: f32,
    pub panning: Option<f32>, // Only for non-positional audio
}

impl Default for AudioSettings {
    fn default() -> Self {
        Self {
            volume: 1.0,
            pitch: 1.0,
            looping: false,
            fade_in: None,
            start_position: 0.0,
            panning: None,
        }
    }
}
