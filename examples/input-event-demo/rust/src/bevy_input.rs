use bevy::{
    input::mouse::{AccumulatedMouseMotion, AccumulatedMouseScroll},
    prelude::*,
};
use godot::global::godot_print;

pub struct BevyInputTestPlugin;

impl Plugin for BevyInputTestPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (mouse_click_system, mouse_move_system, gamepad_system),
        );
    }
}

// This system prints messages when you press or release the left mouse button:
fn mouse_click_system(mouse_button_input: Res<ButtonInput<MouseButton>>) {
    if mouse_button_input.pressed(MouseButton::Left) {
        godot_print!("[BEVY] left mouse currently pressed");
    }

    if mouse_button_input.just_pressed(MouseButton::Left) {
        godot_print!("[BEVY] left mouse just pressed");
    }

    if mouse_button_input.just_released(MouseButton::Left) {
        godot_print!("[BEVY] left mouse just released");
    }
}

// This system prints messages when you finish dragging or scrolling with your mouse
fn mouse_move_system(
    accumulated_mouse_motion: Res<AccumulatedMouseMotion>,
    accumulated_mouse_scroll: Res<AccumulatedMouseScroll>,
) {
    if accumulated_mouse_motion.delta != Vec2::ZERO {
        let delta = accumulated_mouse_motion.delta;
        godot_print!("[BEVY] mouse moved ({}, {})", delta.x, delta.y);
    }
    if accumulated_mouse_scroll.delta != Vec2::ZERO {
        let delta = accumulated_mouse_scroll.delta;
        godot_print!("[BEVY] mouse scrolled ({}, {})", delta.x, delta.y);
    }
}

fn gamepad_system(gamepads: Query<(Entity, &Gamepad)>) {
    for (entity, gamepad) in &gamepads {
        if gamepad.just_pressed(GamepadButton::South) {
            godot_print!("[BEVY] {} just pressed South", entity);
        } else if gamepad.just_released(GamepadButton::South) {
            godot_print!("[BEVY] {} just released South", entity);
        }

        let right_trigger = gamepad.get(GamepadButton::RightTrigger2).unwrap();
        if right_trigger.abs() > 0.01 {
            godot_print!("[BEVY] {} RightTrigger2 value is {}", entity, right_trigger);
        }

        let left_stick_x = gamepad.get(GamepadAxis::LeftStickX).unwrap();
        if left_stick_x.abs() > 0.01 {
            godot_print!("[BEVY] {} LeftStickX value is {}", entity, left_stick_x);
        }
    }
}
