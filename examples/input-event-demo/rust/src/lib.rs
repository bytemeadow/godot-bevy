#![allow(clippy::type_complexity)]

use bevy::prelude::*;
use godot::global::Key;
use godot_bevy::prelude::{
    godot_prelude::{gdextension, godot_print, ExtensionLibrary},
    *,
};

// Import input event types directly to avoid naming conflicts
use godot_bevy::plugins::core::input_event::{
    ActionInput, KeyboardInput, MouseButton, MouseButtonInput, MouseMotion, TouchInput,
};

// This example demonstrates godot-bevy's input event system.
//
// Key behavior:
// - Keys mapped in Godot's Input Map (like arrow keys → "ui_down", "move_down")
//   generate ActionInput events only (no duplicate raw keyboard events)
// - Unmapped keys (like random letters) generate KeyboardInput events only
// - This prevents duplicate events and follows Godot's intended input flow

#[bevy_app]
fn build_app(app: &mut App) {
    app.add_plugins(InputEventPlugin);
}

struct InputEventPlugin;

impl Plugin for InputEventPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_keyboard_input,
                handle_mouse_button_input,
                handle_mouse_motion,
                handle_touch_input,
                handle_action_input,
            ),
        );
    }
}

fn handle_keyboard_input(mut keyboard_events: EventReader<KeyboardInput>) {
    for event in keyboard_events.read() {
        let key_name = format!("{:?}", event.keycode);
        let state = if event.pressed { "pressed" } else { "released" };
        let echo_info = if event.echo { " (echo)" } else { "" };

        godot_print!(
            "🎹 Keyboard: {} {} (physical: {:?}){}",
            key_name,
            state,
            event.physical_keycode,
            echo_info
        );

        // Special handling for common keys
        match event.keycode {
            Key::SPACE if event.pressed => {
                godot_print!("🚀 Space bar pressed - Jump!");
            }
            Key::ESCAPE if event.pressed => {
                godot_print!("🚪 Escape pressed - Pause menu!");
            }
            Key::ENTER if event.pressed => {
                godot_print!("✅ Enter pressed - Confirm!");
            }
            _ => {}
        }
    }
}

fn handle_mouse_button_input(mut mouse_button_events: EventReader<MouseButtonInput>) {
    for event in mouse_button_events.read() {
        let button_name = format!("{:?}", event.button);
        let state = if event.pressed { "pressed" } else { "released" };

        godot_print!(
            "🖱️  Mouse: {} {} at ({:.1}, {:.1})",
            button_name,
            state,
            event.position.x,
            event.position.y
        );

        // Special handling for different buttons
        match event.button {
            MouseButton::Left if event.pressed => {
                godot_print!("👆 Left click - Select/Attack!");
            }
            MouseButton::Right if event.pressed => {
                godot_print!("👉 Right click - Context menu!");
            }
            MouseButton::WheelUp => {
                godot_print!("🔼 Scroll up - Zoom in!");
            }
            MouseButton::WheelDown => {
                godot_print!("🔽 Scroll down - Zoom out!");
            }
            _ => {}
        }
    }
}

fn handle_mouse_motion(mut mouse_motion_events: EventReader<MouseMotion>) {
    for event in mouse_motion_events.read() {
        // Only log significant mouse movements to avoid spam
        if event.delta.length() > 5.0 {
            godot_print!(
                "🖱️  Mouse moved: delta({:.1}, {:.1}) position({:.1}, {:.1})",
                event.delta.x,
                event.delta.y,
                event.position.x,
                event.position.y
            );
        }
    }
}

fn handle_touch_input(mut touch_events: EventReader<TouchInput>) {
    for event in touch_events.read() {
        let state = if event.pressed { "touched" } else { "released" };

        godot_print!(
            "👆 Touch: finger {} {} at ({:.1}, {:.1})",
            event.finger_id,
            state,
            event.position.x,
            event.position.y
        );

        if event.pressed {
            godot_print!("📱 Touch started - finger {}", event.finger_id);
        } else {
            godot_print!("📱 Touch ended - finger {}", event.finger_id);
        }
    }
}

fn handle_action_input(mut action_events: EventReader<ActionInput>) {
    for event in action_events.read() {
        let state = if event.pressed { "pressed" } else { "released" };

        godot_print!(
            "🎮 Action: '{}' {} (strength: {:.2})",
            event.action,
            state,
            event.strength
        );

        // Handle common action names
        match event.action.as_str() {
            "ui_accept" if event.pressed => {
                godot_print!("✅ UI Accept action triggered!");
            }
            "ui_cancel" if event.pressed => {
                godot_print!("❌ UI Cancel action triggered!");
            }
            "move_left" | "move_right" | "move_up" | "move_down" => {
                if event.pressed {
                    godot_print!("🏃 Movement action: {}", event.action);
                }
            }
            "jump" => {
                godot_print!("🦘 Jump action: {}", state);
            }
            _ => {}
        }
    }
}
