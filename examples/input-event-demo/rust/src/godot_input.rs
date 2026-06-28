use bevy::prelude::*;
use godot::global::Key;
use godot_bevy::prelude::godot_prelude::godot_print;

use godot_bevy::plugins::input::{
    ActionInput, GamepadAxisInput, GamepadButtonInput, GodotKeyboardInput, GodotMouseButton,
    GodotMouseButtonInput, GodotMouseMotion, TouchInput,
};

pub struct GodotInputPlugin;

impl Plugin for GodotInputPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (
                handle_keyboard_input,
                handle_mouse_button_input,
                handle_mouse_motion,
                handle_touch_input,
                handle_action_input,
                handle_gamepad_button_input,
                handle_gamepad_axis_input,
            ),
        );
    }
}

fn handle_keyboard_input(mut keyboard_events: MessageReader<GodotKeyboardInput>) {
    for event in keyboard_events.read() {
        let key_name = format!("{:?}", event.keycode);
        let state = if event.pressed { "pressed" } else { "released" };
        let echo_info = if event.echo { " (echo)" } else { "" };

        godot_print!(
            "[GODOT] 🎹 Keyboard: {} {} (physical: {:?}){}",
            key_name,
            state,
            event.physical_keycode,
            echo_info
        );

        // Special handling for common keys
        match event.keycode {
            Key::SPACE if event.pressed => {
                godot_print!("[GODOT] 🚀 Space bar pressed - Jump!");
            }
            Key::ESCAPE if event.pressed => {
                godot_print!("[GODOT] 🚪 Escape pressed - Pause menu!");
            }
            Key::ENTER if event.pressed => {
                godot_print!("[GODOT] ✅ Enter pressed - Confirm!");
            }
            _ => {}
        }
    }
}

fn handle_mouse_button_input(mut mouse_button_events: MessageReader<GodotMouseButtonInput>) {
    for event in mouse_button_events.read() {
        let button_name = format!("{:?}", event.button);
        let state = if event.pressed { "pressed" } else { "released" };

        godot_print!(
            "[GODOT] 🖱️  Mouse: {} {} at ({:.1}, {:.1})",
            button_name,
            state,
            event.position.x,
            event.position.y
        );

        // Special handling for different buttons
        match event.button {
            GodotMouseButton::Left if event.pressed => {
                godot_print!("[GODOT] 👆 Left click - Select/Attack!");
            }
            GodotMouseButton::Right if event.pressed => {
                godot_print!("[GODOT] 👉 Right click - Context menu!");
            }
            GodotMouseButton::WheelUp => {
                godot_print!("[GODOT] 🔼 Scroll up - Zoom in!");
            }
            GodotMouseButton::WheelDown => {
                godot_print!("[GODOT] 🔽 Scroll down - Zoom out!");
            }
            _ => {}
        }
    }
}

fn handle_mouse_motion(mut mouse_motion_events: MessageReader<GodotMouseMotion>) {
    for event in mouse_motion_events.read() {
        // Only log significant mouse movements to avoid spam
        if event.delta.length() > 5.0 {
            godot_print!(
                "[GODOT] 🖱️  Mouse moved: delta({:.1}, {:.1}) position({:.1}, {:.1})",
                event.delta.x,
                event.delta.y,
                event.position.x,
                event.position.y
            );
        }
    }
}

fn handle_touch_input(mut touch_events: MessageReader<TouchInput>) {
    for event in touch_events.read() {
        let state = if event.pressed { "touched" } else { "released" };

        godot_print!(
            "[GODOT] 👆 Touch: finger {} {} at ({:.1}, {:.1})",
            event.finger_id,
            state,
            event.position.x,
            event.position.y
        );

        if event.pressed {
            godot_print!("[GODOT] 📱 Touch started - finger {}", event.finger_id);
        } else {
            godot_print!("[GODOT] 📱 Touch ended - finger {}", event.finger_id);
        }
    }
}

fn handle_action_input(mut action_events: MessageReader<ActionInput>) {
    for event in action_events.read() {
        let state = if event.pressed { "pressed" } else { "released" };

        godot_print!(
            "[GODOT] 🎮 Action: '{}' {} (strength: {:.2})",
            event.action,
            state,
            event.strength
        );

        // Handle common action names
        match event.action.as_str() {
            "ui_accept" if event.pressed => {
                godot_print!("[GODOT] ✅ UI Accept action triggered!");
            }
            "ui_cancel" if event.pressed => {
                godot_print!("[GODOT] ❌ UI Cancel action triggered!");
            }
            "move_left" | "move_right" | "move_up" | "move_down" if event.pressed => {
                godot_print!("[GODOT] 🏃 Movement action: {}", event.action);
            }
            "jump" => {
                godot_print!("[GODOT] 🦘 Jump action: {}", state);
            }
            _ => {}
        }
    }
}

fn handle_gamepad_button_input(mut gamepad_button_events: MessageReader<GamepadButtonInput>) {
    for event in gamepad_button_events.read() {
        let state = if event.pressed { "pressed" } else { "released" };

        godot_print!(
            "[GODOT] 🎮 Gamepad {}: Button {} {} (pressure: {:.2})",
            event.device,
            event.button_index,
            state,
            event.pressure
        );

        // Handle common buttons
        match event.button_index {
            0 if event.pressed => {
                // A button (South)
                godot_print!("[GODOT] 🔴 A button pressed - Jump/Confirm!");
            }
            1 if event.pressed => {
                // B button (East)
                godot_print!("[GODOT] 🔵 B button pressed - Back/Cancel!");
            }
            2 if event.pressed => {
                // X button (West)
                godot_print!("[GODOT] 🟩 X button pressed - Action!");
            }
            3 if event.pressed => {
                // Y button (North)
                godot_print!("[GODOT] 🟨 Y button pressed - Menu!");
            }
            _ => {}
        }
    }
}

fn handle_gamepad_axis_input(mut gamepad_axis_events: MessageReader<GamepadAxisInput>) {
    for event in gamepad_axis_events.read() {
        // Only log significant axis movements to avoid spam
        if event.value.abs() > 0.1 {
            godot_print!(
                "[GODOT] 🕹️ Gamepad {}: Axis {} = {:.2}",
                event.device,
                event.axis,
                event.value
            );

            // Handle common axes
            match event.axis {
                0 => godot_print!("[GODOT] ⬅️➡️ Left stick X: {:.2}", event.value),
                1 => godot_print!("[GODOT] ⬆️⬇️ Left stick Y: {:.2}", event.value),
                2 => godot_print!("[GODOT] ⬅️➡️ Right stick X: {:.2}", event.value),
                3 => godot_print!("[GODOT] ⬆️⬇️ Right stick Y: {:.2}", event.value),
                4 => godot_print!("[GODOT] 🎯 Left trigger: {:.2}", event.value),
                5 => godot_print!("[GODOT] 🎯 Right trigger: {:.2}", event.value),
                _ => {}
            }
        }
    }
}
