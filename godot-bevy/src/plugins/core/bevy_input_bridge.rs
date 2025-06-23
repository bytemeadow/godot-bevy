use bevy::{
    app::{App, Last, Plugin, PreUpdate},
    ecs::{
        event::{EventReader, EventWriter},
        system::ResMut,
    },
    input::{
        Axis, ButtonInput, ButtonState,
        gamepad::{GamepadAxis, GamepadButton},
        keyboard::KeyCode,
        mouse::{
            AccumulatedMouseMotion, AccumulatedMouseScroll, MouseButton as BevyMouseButton,
            MouseButtonInput as BevyMouseButtonInput, MouseMotion as BevyMouseMotion,
            mouse_button_input_system,
        },
    },
    math::Vec2,
    prelude::Entity,
};

use crate::plugins::core::input_event::{
    GamepadAxisInput as GodotGamepadAxisInput, GamepadButtonInput as GodotGamepadButtonInput,
    KeyboardInput as GodotKeyboardInput, MouseButton as GodotMouseButton,
    MouseButtonInput as GodotMouseButtonInput, MouseMotion as GodotMouseMotion,
};

/// Plugin that bridges godot-bevy's input events to Bevy's standard input resources.
pub struct BevyInputBridgePlugin;

impl Plugin for BevyInputBridgePlugin {
    fn build(&self, app: &mut App) {
        // Add Bevy's standard input resources and events
        app.init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<ButtonInput<BevyMouseButton>>()
            .init_resource::<AccumulatedMouseMotion>()
            .init_resource::<AccumulatedMouseScroll>()
            .init_resource::<ButtonInput<GamepadButton>>()
            .init_resource::<Axis<GamepadAxis>>()
            .add_event::<BevyMouseMotion>()
            .add_event::<BevyMouseButtonInput>()
            .add_systems(
                PreUpdate,
                (
                    bridge_keyboard_input,
                    bridge_mouse_button_input,
                    bridge_mouse_motion,
                    bridge_mouse_scroll,
                    bridge_gamepad_button_input,
                    bridge_gamepad_axis_input,
                    // Add Bevy's mouse_button_input_system to process MouseButtonInput events
                    mouse_button_input_system,
                ),
            )
            .add_systems(Last, update_input_resources);
    }
}

fn bridge_keyboard_input(
    mut keyboard_events: EventReader<GodotKeyboardInput>,
    mut key_code_input: ResMut<ButtonInput<KeyCode>>,
) {
    for event in keyboard_events.read() {
        // Convert Godot Key to Bevy KeyCode
        if let Some(bevy_key_code) = godot_key_to_bevy_keycode(event.keycode) {
            if event.pressed {
                key_code_input.press(bevy_key_code);
            } else {
                key_code_input.release(bevy_key_code);
            }
        }
    }
}

fn bridge_mouse_button_input(
    mut mouse_events: EventReader<GodotMouseButtonInput>,
    mut bevy_mouse_button_events: EventWriter<BevyMouseButtonInput>,
) {
    for event in mouse_events.read() {
        // Skip wheel events - they're handled separately in bridge_mouse_scroll
        match event.button {
            GodotMouseButton::WheelUp
            | GodotMouseButton::WheelDown
            | GodotMouseButton::WheelLeft
            | GodotMouseButton::WheelRight => continue,
            _ => {}
        }

        let bevy_button = godot_mouse_to_bevy_mouse(event.button);
        let state = if event.pressed {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        };

        // Send MouseButtonInput event that Bevy's mouse_button_input_system will process
        bevy_mouse_button_events.send(BevyMouseButtonInput {
            button: bevy_button,
            state,
            window: Entity::PLACEHOLDER,
        });
    }
}

fn bridge_mouse_motion(
    mut mouse_motion_events: EventReader<GodotMouseMotion>,
    mut bevy_mouse_motion_events: EventWriter<BevyMouseMotion>,
    mut accumulated_motion: ResMut<AccumulatedMouseMotion>,
) {
    // Reset accumulated motion at the start of the frame (like Bevy does)
    accumulated_motion.delta = Vec2::ZERO;

    // Send individual Bevy MouseMotion events AND accumulate for the frame
    for event in mouse_motion_events.read() {
        // Send individual MouseMotion event (for libraries that prefer events)
        bevy_mouse_motion_events.send(BevyMouseMotion { delta: event.delta });

        // Accumulate delta for the AccumulatedMouseMotion resource
        accumulated_motion.delta += event.delta;
    }
}

fn bridge_mouse_scroll(
    mut mouse_button_events: EventReader<GodotMouseButtonInput>,
    mut accumulated_scroll: ResMut<AccumulatedMouseScroll>,
) {
    // Reset accumulated scroll at the start of the frame (like Bevy does)
    accumulated_scroll.delta = Vec2::ZERO;

    // Convert wheel button events to scroll accumulation for this frame
    for event in mouse_button_events.read() {
        if event.pressed {
            match event.button {
                GodotMouseButton::WheelUp => {
                    accumulated_scroll.delta.y += 1.0;
                }
                GodotMouseButton::WheelDown => {
                    accumulated_scroll.delta.y -= 1.0;
                }
                GodotMouseButton::WheelLeft => {
                    accumulated_scroll.delta.x -= 1.0;
                }
                GodotMouseButton::WheelRight => {
                    accumulated_scroll.delta.x += 1.0;
                }
                _ => {} // Ignore non-wheel buttons
            }
        }
    }
}

fn bridge_gamepad_button_input(
    mut gamepad_button_events: EventReader<GodotGamepadButtonInput>,
    mut gamepad_button_input: ResMut<ButtonInput<GamepadButton>>,
) {
    for event in gamepad_button_events.read() {
        if let Some(bevy_button) = godot_button_to_bevy_button(event.button_index) {
            if event.pressed {
                gamepad_button_input.press(bevy_button);
            } else {
                gamepad_button_input.release(bevy_button);
            }
        }
    }
}

fn bridge_gamepad_axis_input(
    mut gamepad_axis_events: EventReader<GodotGamepadAxisInput>,
    mut gamepad_axis_input: ResMut<Axis<GamepadAxis>>,
) {
    for event in gamepad_axis_events.read() {
        if let Some(bevy_axis) = godot_axis_to_bevy_axis(event.axis) {
            gamepad_axis_input.set(bevy_axis, event.value);
        }
    }
}

fn update_input_resources(
    mut keyboard_input: ResMut<ButtonInput<KeyCode>>,
    mut gamepad_button_input: ResMut<ButtonInput<GamepadButton>>,
) {
    // Clear just_pressed/just_released states at the end of each frame
    // This is what Bevy's InputPlugin normally does
    keyboard_input.clear();
    gamepad_button_input.clear();
    // Note: Mouse input is handled by Bevy's mouse_button_input_system
    // Note: AccumulatedMouseMotion and AccumulatedMouseScroll are reset
    // at the beginning of each frame in their respective bridge systems
    // Note: GamepadAxis doesn't need clearing as it's state-based, not event-based
}

// Conversion functions
fn godot_key_to_bevy_keycode(godot_key: godot::global::Key) -> Option<KeyCode> {
    use KeyCode as BK;
    use godot::global::Key as GK;

    match godot_key {
        GK::A => Some(BK::KeyA),
        GK::B => Some(BK::KeyB),
        GK::C => Some(BK::KeyC),
        GK::D => Some(BK::KeyD),
        GK::E => Some(BK::KeyE),
        GK::F => Some(BK::KeyF),
        GK::G => Some(BK::KeyG),
        GK::H => Some(BK::KeyH),
        GK::I => Some(BK::KeyI),
        GK::J => Some(BK::KeyJ),
        GK::K => Some(BK::KeyK),
        GK::L => Some(BK::KeyL),
        GK::M => Some(BK::KeyM),
        GK::N => Some(BK::KeyN),
        GK::O => Some(BK::KeyO),
        GK::P => Some(BK::KeyP),
        GK::Q => Some(BK::KeyQ),
        GK::R => Some(BK::KeyR),
        GK::S => Some(BK::KeyS),
        GK::T => Some(BK::KeyT),
        GK::U => Some(BK::KeyU),
        GK::V => Some(BK::KeyV),
        GK::W => Some(BK::KeyW),
        GK::X => Some(BK::KeyX),
        GK::Y => Some(BK::KeyY),
        GK::Z => Some(BK::KeyZ),

        GK::KEY_0 => Some(BK::Digit0),
        GK::KEY_1 => Some(BK::Digit1),
        GK::KEY_2 => Some(BK::Digit2),
        GK::KEY_3 => Some(BK::Digit3),
        GK::KEY_4 => Some(BK::Digit4),
        GK::KEY_5 => Some(BK::Digit5),
        GK::KEY_6 => Some(BK::Digit6),
        GK::KEY_7 => Some(BK::Digit7),
        GK::KEY_8 => Some(BK::Digit8),
        GK::KEY_9 => Some(BK::Digit9),

        GK::SPACE => Some(BK::Space),
        GK::ENTER => Some(BK::Enter),
        GK::ESCAPE => Some(BK::Escape),
        GK::BACKSPACE => Some(BK::Backspace),
        GK::TAB => Some(BK::Tab),
        GK::SHIFT => Some(BK::ShiftLeft),
        GK::CTRL => Some(BK::ControlLeft),
        GK::ALT => Some(BK::AltLeft),

        GK::LEFT => Some(BK::ArrowLeft),
        GK::RIGHT => Some(BK::ArrowRight),
        GK::UP => Some(BK::ArrowUp),
        GK::DOWN => Some(BK::ArrowDown),

        GK::F1 => Some(BK::F1),
        GK::F2 => Some(BK::F2),
        GK::F3 => Some(BK::F3),
        GK::F4 => Some(BK::F4),
        GK::F5 => Some(BK::F5),
        GK::F6 => Some(BK::F6),
        GK::F7 => Some(BK::F7),
        GK::F8 => Some(BK::F8),
        GK::F9 => Some(BK::F9),
        GK::F10 => Some(BK::F10),
        GK::F11 => Some(BK::F11),
        GK::F12 => Some(BK::F12),

        _ => None, // Many keys don't have direct equivalents
    }
}

fn godot_mouse_to_bevy_mouse(godot_button: GodotMouseButton) -> BevyMouseButton {
    match godot_button {
        GodotMouseButton::Left => BevyMouseButton::Left,
        GodotMouseButton::Right => BevyMouseButton::Right,
        GodotMouseButton::Middle => BevyMouseButton::Middle,
        GodotMouseButton::Extra1 => BevyMouseButton::Back,
        GodotMouseButton::Extra2 => BevyMouseButton::Forward,
        // Note: Bevy doesn't have wheel events as buttons
        _ => BevyMouseButton::Other(255),
    }
}

fn godot_button_to_bevy_button(button_index: i32) -> Option<GamepadButton> {
    // Map Godot's JoyButton enum to Bevy's GamepadButton
    // Reference: https://docs.godotengine.org/en/stable/classes/class_%40globalscope.html#enum-globalscope-joybutton
    match button_index {
        0 => Some(GamepadButton::South), // JOY_BUTTON_A / Bottom face button
        1 => Some(GamepadButton::East),  // JOY_BUTTON_B / Right face button
        2 => Some(GamepadButton::West),  // JOY_BUTTON_X / Left face button
        3 => Some(GamepadButton::North), // JOY_BUTTON_Y / Top face button
        4 => Some(GamepadButton::LeftTrigger), // JOY_BUTTON_LEFT_SHOULDER
        5 => Some(GamepadButton::RightTrigger), // JOY_BUTTON_RIGHT_SHOULDER
        6 => Some(GamepadButton::LeftTrigger2), // JOY_BUTTON_LEFT_TRIGGER
        7 => Some(GamepadButton::RightTrigger2), // JOY_BUTTON_RIGHT_TRIGGER
        8 => Some(GamepadButton::Select), // JOY_BUTTON_LEFT_STICK
        9 => Some(GamepadButton::Start), // JOY_BUTTON_RIGHT_STICK
        10 => Some(GamepadButton::LeftThumb), // JOY_BUTTON_LEFT_STICK
        11 => Some(GamepadButton::RightThumb), // JOY_BUTTON_RIGHT_STICK
        12 => Some(GamepadButton::DPadUp), // JOY_BUTTON_DPAD_UP
        13 => Some(GamepadButton::DPadDown), // JOY_BUTTON_DPAD_DOWN
        14 => Some(GamepadButton::DPadLeft), // JOY_BUTTON_DPAD_LEFT
        15 => Some(GamepadButton::DPadRight), // JOY_BUTTON_DPAD_RIGHT
        16 => Some(GamepadButton::Mode), // JOY_BUTTON_MISC1 (Guide/Home)
        _ => Some(GamepadButton::Other(button_index as u8)), // Non-standard buttons
    }
}

fn godot_axis_to_bevy_axis(axis: i32) -> Option<GamepadAxis> {
    // Map Godot's JoyAxis enum to Bevy's GamepadAxis
    // Reference: https://docs.godotengine.org/en/stable/classes/class_%40globalscope.html#enum-globalscope-joyaxis
    match axis {
        0 => Some(GamepadAxis::LeftStickX),        // JOY_AXIS_LEFT_X
        1 => Some(GamepadAxis::LeftStickY),        // JOY_AXIS_LEFT_Y
        2 => Some(GamepadAxis::RightStickX),       // JOY_AXIS_RIGHT_X
        3 => Some(GamepadAxis::RightStickY),       // JOY_AXIS_RIGHT_Y
        4 => Some(GamepadAxis::LeftZ),             // JOY_AXIS_TRIGGER_LEFT
        5 => Some(GamepadAxis::RightZ),            // JOY_AXIS_TRIGGER_RIGHT
        _ => Some(GamepadAxis::Other(axis as u8)), // Non-standard axes
    }
}
