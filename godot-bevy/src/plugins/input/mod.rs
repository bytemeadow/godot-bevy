pub mod events;
pub mod input_bridge;

// Re-export the main plugins
pub use events::GodotInputEventPlugin;
pub use input_bridge::BevyInputBridgePlugin;

// Re-export event types for convenience
pub use events::{
    ActionInput, GamepadAxisInput, GamepadButtonInput, KeyboardInput, MouseButton,
    MouseButtonInput, MouseMotion, TouchInput,
};

// Re-export input reader types
pub use events::{InputEventReader, InputEventType};
