//! Audio tweening and easing for smooth transitions

use std::time::Duration;

/// Tweening/easing configuration for smooth audio transitions
#[derive(Debug, Clone)]
pub struct AudioTween {
    pub duration: Duration,
    pub easing: AudioEasing,
}

/// Audio easing types for smooth transitions
#[derive(Debug, Clone, Copy)]
pub enum AudioEasing {
    Linear,
    EaseIn,
    EaseOut,
    EaseInOut,
}

impl AudioTween {
    /// Create a new tween with the given duration and easing
    pub fn new(duration: Duration, easing: AudioEasing) -> Self {
        Self { duration, easing }
    }

    /// Create a new linear tween with the given duration
    pub fn linear(duration: Duration) -> Self {
        Self::new(duration, AudioEasing::Linear)
    }

    /// Set an easing for the tween
    pub fn with_easing(mut self, easing: AudioEasing) -> Self {
        self.easing = easing;
        self
    }
}

impl Default for AudioTween {
    fn default() -> Self {
        Self::new(Duration::from_millis(10), AudioEasing::Linear)
    }
}
