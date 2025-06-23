use bevy::prelude::*;
use godot_bevy::prelude::godot_prelude::godot_print;

pub struct BevyInputTestPlugin;

impl Plugin for BevyInputTestPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, test_bevy_input_resources);
    }
}

fn test_bevy_input_resources(
    keyboard_input: Res<bevy::input::ButtonInput<bevy::input::keyboard::KeyCode>>,
    mouse_input: Res<bevy::input::ButtonInput<bevy::input::mouse::MouseButton>>,
    gamepad_input: Res<bevy::input::ButtonInput<bevy::input::gamepad::GamepadButton>>,
    gamepad_axes: Res<bevy::input::Axis<bevy::input::gamepad::GamepadAxis>>,
) {
    use bevy::input::gamepad::{GamepadAxis, GamepadButton};
    use bevy::input::keyboard::KeyCode;
    use bevy::input::mouse::MouseButton;

    // Debug info
    if keyboard_input.just_pressed(KeyCode::KeyG) {
        godot_print!("[BEVY] 🎮 DEBUG: Testing with gamepad ID 0");
    }

    // Test some common keys using Bevy's standard input resources
    if keyboard_input.just_pressed(KeyCode::KeyT) {
        godot_print!("[BEVY] 🧪 'T' key detected via Bevy's ButtonInput<KeyCode>!");
    }

    if keyboard_input.just_pressed(KeyCode::KeyB) {
        godot_print!("[BEVY] 🧪 'B' key detected via Bevy's ButtonInput<KeyCode>!");
    }

    if mouse_input.just_pressed(MouseButton::Left) {
        godot_print!("[BEVY] 🧪 Left mouse button detected via Bevy's ButtonInput<MouseButton>!");
    }

    if mouse_input.just_pressed(MouseButton::Right) {
        godot_print!("[BEVY] 🧪 Right mouse button detected via Bevy's ButtonInput<MouseButton>!");
    }

    if keyboard_input.pressed(KeyCode::Space) {
        godot_print!("[BEVY] 🧪 Space held via Bevy input system!");
    }

    // Test gamepad inputs via Bevy's standard resources
    if gamepad_input.just_pressed(GamepadButton::South) {
        godot_print!("[BEVY] 🧪 Gamepad A button detected via Bevy's ButtonInput<GamepadButton>!");
    }

    if gamepad_input.just_pressed(GamepadButton::East) {
        godot_print!("[BEVY] 🧪 Gamepad B button detected via Bevy's ButtonInput<GamepadButton>!");
    }

    // Test gamepad axes
    let left_stick_x = gamepad_axes.get(GamepadAxis::LeftStickX).unwrap_or(0.0);
    let left_stick_y = gamepad_axes.get(GamepadAxis::LeftStickY).unwrap_or(0.0);

    if left_stick_x.abs() > 0.5 || left_stick_y.abs() > 0.5 {
        godot_print!(
            "[BEVY] 🧪 Left stick via Bevy Axis: ({:.2}, {:.2})",
            left_stick_x,
            left_stick_y
        );
    }
}
