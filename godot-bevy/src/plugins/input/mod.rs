pub mod actions;
pub mod events;
pub mod input_bridge;

// Re-export the main plugins
pub use events::GodotInputEventPlugin;
pub use input_bridge::BevyInputBridgePlugin;

// Re-export actions API
pub use actions::{Action, GodotActions, GodotActionsPlugin, GodotInputSet};

// Re-export event types for convenience
pub use events::{
    ActionInput, GamepadAxisInput, GamepadButtonInput, GodotKeyboardInput, GodotMouseButton,
    GodotMouseButtonInput, GodotMouseMotion, PanGestureInput, TouchInput,
};

// Re-export input reader types
pub use events::{InputEventReader, InputEventType};
