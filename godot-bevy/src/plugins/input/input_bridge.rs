use bevy::{
    app::{App, Last, Plugin, PreUpdate},
    ecs::{
        entity::Entity,
        event::{EventReader, EventWriter},
        system::ResMut,
    },
    input::{
        ButtonInput, ButtonState, InputPlugin,
        keyboard::KeyCode,
        mouse::{
            AccumulatedMouseMotion, AccumulatedMouseScroll, MouseButton as BevyMouseButton,
            MouseButtonInput as BevyMouseButtonInput, MouseMotion as BevyMouseMotion,
        },
    },
    math::Vec2,
};

use crate::plugins::input::events::{
    KeyboardInput as GodotKeyboardInput, MouseButton as GodotMouseButton,
    MouseButtonInput as GodotMouseButtonInput, MouseMotion as GodotMouseMotion,
};

/// Plugin that bridges godot-bevy's input events to Bevy's standard input resources.
/// This plugin automatically includes GodotInputEventPlugin as a dependency.
#[derive(Default)]
pub struct BevyInputBridgePlugin;

impl Plugin for BevyInputBridgePlugin {
    fn build(&self, app: &mut App) {
        // Add the dependency - we need Godot input events to bridge them
        app.add_plugins(super::events::GodotInputEventPlugin)
            .add_plugins(InputPlugin)
            .add_systems(
                PreUpdate,
                (
                    bridge_keyboard_input,
                    bridge_mouse_button_input,
                    bridge_mouse_motion,
                    bridge_mouse_scroll,
                ),
            )
            .add_systems(Last, clear_keyboard_input);
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
        bevy_mouse_button_events.write(BevyMouseButtonInput {
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
        bevy_mouse_motion_events.write(BevyMouseMotion { delta: event.delta });

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

fn clear_keyboard_input(mut keyboard_input: ResMut<ButtonInput<KeyCode>>) {
    // Clear just_pressed/just_released states at the end of each frame
    // This is what Bevy's InputPlugin normally does for gamepads, but we handle keyboard manually
    keyboard_input.clear();
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

        // Numpad keys
        GK::KP_0 => Some(BK::Numpad0),
        GK::KP_1 => Some(BK::Numpad1),
        GK::KP_2 => Some(BK::Numpad2),
        GK::KP_3 => Some(BK::Numpad3),
        GK::KP_4 => Some(BK::Numpad4),
        GK::KP_5 => Some(BK::Numpad5),
        GK::KP_6 => Some(BK::Numpad6),
        GK::KP_7 => Some(BK::Numpad7),
        GK::KP_8 => Some(BK::Numpad8),
        GK::KP_9 => Some(BK::Numpad9),
        GK::KP_ADD => Some(BK::NumpadAdd),
        GK::KP_SUBTRACT => Some(BK::NumpadSubtract),
        GK::KP_MULTIPLY => Some(BK::NumpadMultiply),
        GK::KP_DIVIDE => Some(BK::NumpadDivide),
        GK::KP_PERIOD => Some(BK::NumpadDecimal),
        GK::KP_ENTER => Some(BK::NumpadEnter),

        // Additional common keys
        GK::DELETE => Some(BK::Delete),
        GK::INSERT => Some(BK::Insert),
        GK::HOME => Some(BK::Home),
        GK::END => Some(BK::End),
        GK::PAGEUP => Some(BK::PageUp),
        GK::PAGEDOWN => Some(BK::PageDown),
        GK::CAPSLOCK => Some(BK::CapsLock),
        GK::NUMLOCK => Some(BK::NumLock),
        GK::SCROLLLOCK => Some(BK::ScrollLock),
        GK::PAUSE => Some(BK::Pause),
        GK::PRINT => Some(BK::PrintScreen),

        // Punctuation and symbols
        GK::COMMA => Some(BK::Comma),
        GK::PERIOD => Some(BK::Period),
        GK::SLASH => Some(BK::Slash),
        GK::SEMICOLON => Some(BK::Semicolon),
        GK::APOSTROPHE => Some(BK::Quote),
        GK::BRACKETLEFT => Some(BK::BracketLeft),
        GK::BRACKETRIGHT => Some(BK::BracketRight),
        GK::BACKSLASH => Some(BK::Backslash),
        GK::QUOTELEFT => Some(BK::Backquote),
        GK::MINUS => Some(BK::Minus),
        GK::EQUAL => Some(BK::Equal),

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
