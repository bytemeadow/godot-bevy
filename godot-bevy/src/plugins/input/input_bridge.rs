use bevy_app::{App, First, Plugin};
use bevy_ecs::{
    entity::Entity,
    message::{MessageReader, MessageWriter},
    schedule::IntoScheduleConfigs,
};
use bevy_input::{
    ButtonState, InputPlugin,
    gestures::PanGesture as BevyPanGesture,
    keyboard::{Key, KeyCode, KeyboardInput as BevyKeyboardInput, NativeKey, NativeKeyCode},
    mouse::{
        MouseButton as BevyMouseButton, MouseButtonInput as BevyMouseButtonInput,
        MouseMotion as BevyMouseMotion, MouseScrollUnit, MouseWheel as BevyMouseWheel,
    },
    touch::TouchPhase,
};

use crate::plugins::input::events::{
    GodotKeyboardInput, GodotMouseButton, GodotMouseButtonInput, GodotMouseMotion,
    PanGestureInput as GodotPanGestureInput,
};

/// Plugin that bridges godot-bevy's input messages to Bevy's standard input resources.
/// Automatically includes `GodotInputEventPlugin`.
#[derive(Default)]
pub struct BevyInputBridgePlugin;

impl Plugin for BevyInputBridgePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(super::events::GodotInputEventPlugin)
            .add_plugins(InputPlugin)
            .add_systems(
                First,
                (
                    bridge_keyboard_input,
                    bridge_mouse_button_input,
                    bridge_mouse_motion,
                    bridge_mouse_scroll,
                    bridge_pan_gesture,
                )
                    // Godot input messages are written and buffer-swapped within
                    // First; without this ordering the bridge can run outside the
                    // one-frame window where they are readable and drop input.
                    .after(super::events::write_input_messages),
            );
    }
}

fn bridge_keyboard_input(
    mut keyboard_messages: MessageReader<GodotKeyboardInput>,
    mut bevy_keyboard_events: MessageWriter<BevyKeyboardInput>,
) {
    for msg in keyboard_messages.read() {
        let key_code = godot_key_to_bevy_keycode(msg.keycode)
            .unwrap_or(KeyCode::Unidentified(NativeKeyCode::Unidentified));

        let (logical_key, text) = if msg.unicode != 0 {
            if let Some(ch) = char::from_u32(msg.unicode) {
                let s = ch.to_string();
                (Key::Character(s.clone().into()), Some(s.into()))
            } else {
                (godot_key_to_bevy_key(msg.keycode), None)
            }
        } else {
            (godot_key_to_bevy_key(msg.keycode), None)
        };

        let state = if msg.pressed {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        };

        bevy_keyboard_events.write(BevyKeyboardInput {
            key_code,
            logical_key,
            state,
            text,
            repeat: msg.echo,
            window: Entity::PLACEHOLDER,
        });
    }
}

fn bridge_mouse_button_input(
    mut mouse_messages: MessageReader<GodotMouseButtonInput>,
    mut bevy_mouse_button_messages: MessageWriter<BevyMouseButtonInput>,
) {
    for message in mouse_messages.read() {
        // Skip wheel events - they're handled separately in bridge_mouse_scroll
        if matches!(
            message.button,
            GodotMouseButton::WheelUp
                | GodotMouseButton::WheelDown
                | GodotMouseButton::WheelLeft
                | GodotMouseButton::WheelRight
        ) {
            continue;
        }

        let bevy_button = godot_mouse_to_bevy_mouse(message.button);
        let state = if message.pressed {
            ButtonState::Pressed
        } else {
            ButtonState::Released
        };

        bevy_mouse_button_messages.write(BevyMouseButtonInput {
            button: bevy_button,
            state,
            window: Entity::PLACEHOLDER,
        });
    }
}

fn bridge_mouse_motion(
    mut mouse_motion_messages: MessageReader<GodotMouseMotion>,
    mut bevy_mouse_motion_messages: MessageWriter<BevyMouseMotion>,
) {
    for event in mouse_motion_messages.read() {
        bevy_mouse_motion_messages.write(BevyMouseMotion { delta: event.delta });
    }
}

fn bridge_mouse_scroll(
    mut mouse_button_messages: MessageReader<GodotMouseButtonInput>,
    mut bevy_mouse_scroll_messages: MessageWriter<BevyMouseWheel>,
) {
    for message in mouse_button_messages.read() {
        match message.button {
            GodotMouseButton::WheelUp => {
                bevy_mouse_scroll_messages.write(BevyMouseWheel {
                    x: 0.0,
                    y: message.factor,
                    unit: MouseScrollUnit::Line,
                    window: Entity::PLACEHOLDER,
                    // Godot delivers discrete wheel ticks; mouse wheels always report Moved.
                    phase: TouchPhase::Moved,
                });
            }
            GodotMouseButton::WheelDown => {
                bevy_mouse_scroll_messages.write(BevyMouseWheel {
                    x: 0.0,
                    y: -message.factor,
                    unit: MouseScrollUnit::Line,
                    window: Entity::PLACEHOLDER,
                    // Godot delivers discrete wheel ticks; mouse wheels always report Moved.
                    phase: TouchPhase::Moved,
                });
            }
            GodotMouseButton::WheelLeft => {
                bevy_mouse_scroll_messages.write(BevyMouseWheel {
                    x: -message.factor,
                    y: 0.0,
                    unit: MouseScrollUnit::Line,
                    window: Entity::PLACEHOLDER,
                    // Godot delivers discrete wheel ticks; mouse wheels always report Moved.
                    phase: TouchPhase::Moved,
                });
            }
            GodotMouseButton::WheelRight => {
                bevy_mouse_scroll_messages.write(BevyMouseWheel {
                    x: message.factor,
                    y: 0.0,
                    unit: MouseScrollUnit::Line,
                    window: Entity::PLACEHOLDER,
                    // Godot delivers discrete wheel ticks; mouse wheels always report Moved.
                    phase: TouchPhase::Moved,
                });
            }
            _ => {}
        }
    }
}

fn bridge_pan_gesture(
    mut pan_messages: MessageReader<GodotPanGestureInput>,
    mut bevy_pan_messages: MessageWriter<BevyPanGesture>,
) {
    for event in pan_messages.read() {
        bevy_pan_messages.write(BevyPanGesture(event.delta));
    }
}

// Conversion functions
fn godot_key_to_bevy_key(godot_key: godot::global::Key) -> Key {
    use godot::global::Key as GK;

    match godot_key {
        GK::SPACE => Key::Space,
        GK::ENTER | GK::KP_ENTER => Key::Enter,
        GK::ESCAPE => Key::Escape,
        GK::BACKSPACE => Key::Backspace,
        GK::TAB => Key::Tab,
        GK::SHIFT => Key::Shift,
        GK::CTRL => Key::Control,
        GK::ALT => Key::Alt,
        GK::LEFT => Key::ArrowLeft,
        GK::RIGHT => Key::ArrowRight,
        GK::UP => Key::ArrowUp,
        GK::DOWN => Key::ArrowDown,
        GK::F1 => Key::F1,
        GK::F2 => Key::F2,
        GK::F3 => Key::F3,
        GK::F4 => Key::F4,
        GK::F5 => Key::F5,
        GK::F6 => Key::F6,
        GK::F7 => Key::F7,
        GK::F8 => Key::F8,
        GK::F9 => Key::F9,
        GK::F10 => Key::F10,
        GK::F11 => Key::F11,
        GK::F12 => Key::F12,
        GK::DELETE => Key::Delete,
        GK::INSERT => Key::Insert,
        GK::HOME => Key::Home,
        GK::END => Key::End,
        GK::PAGEUP => Key::PageUp,
        GK::PAGEDOWN => Key::PageDown,
        GK::CAPSLOCK => Key::CapsLock,
        GK::NUMLOCK => Key::NumLock,
        GK::SCROLLLOCK => Key::ScrollLock,
        GK::PAUSE => Key::Pause,
        GK::PRINT => Key::PrintScreen,
        _ => Key::Unidentified(NativeKey::Unidentified),
    }
}

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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::input::events::GodotKeyboardInput;
    use bevy_app::{App, First, Update};
    use bevy_ecs::{
        message::{MessageReader, Messages},
        resource::Resource,
        system::ResMut,
    };
    use bevy_input::ButtonInput;

    #[derive(Resource, Default)]
    struct Collected(Vec<BevyKeyboardInput>);

    fn collect(mut reader: MessageReader<BevyKeyboardInput>, mut out: ResMut<Collected>) {
        for ev in reader.read() {
            out.0.push(ev.clone());
        }
    }

    fn make_app() -> App {
        let mut app = App::new();
        app.add_message::<GodotKeyboardInput>()
            .add_message::<BevyKeyboardInput>()
            .init_resource::<ButtonInput<KeyCode>>()
            .init_resource::<Collected>()
            .add_systems(First, bridge_keyboard_input)
            .add_systems(Update, collect);
        app
    }

    fn send(app: &mut App, msg: GodotKeyboardInput) {
        app.world_mut()
            .resource_mut::<Messages<GodotKeyboardInput>>()
            .write(msg);
    }

    fn drain(app: &mut App) -> Vec<BevyKeyboardInput> {
        core::mem::take(&mut app.world_mut().resource_mut::<Collected>().0)
    }

    fn godot_key_msg(
        keycode: godot::global::Key,
        unicode: u32,
        pressed: bool,
        echo: bool,
    ) -> GodotKeyboardInput {
        GodotKeyboardInput {
            keycode,
            physical_keycode: None,
            pressed,
            echo,
            unicode,
        }
    }

    // (a) printable: Key::A + unicode 'a' -> KeyCode::KeyA + Key::Character("a") + text Some("a") + Pressed
    #[test]
    fn printable_key_maps_to_character() {
        let mut app = make_app();
        send(
            &mut app,
            godot_key_msg(godot::global::Key::A, 'a' as u32, true, false),
        );
        app.update();
        let events = drain(&mut app);
        assert_eq!(events.len(), 1);
        let ev = &events[0];
        assert_eq!(ev.key_code, KeyCode::KeyA);
        assert_eq!(ev.logical_key, Key::Character("a".to_string().into()));
        assert_eq!(ev.text, Some("a".to_string().into()));
        assert_eq!(ev.state, ButtonState::Pressed);
        assert!(!ev.repeat);
    }

    // (b) named non-printable: Key::ESCAPE + unicode 0 -> KeyCode::Escape + Key::Escape + text None
    #[test]
    fn non_printable_key_maps_to_named_variant() {
        let mut app = make_app();
        send(
            &mut app,
            godot_key_msg(godot::global::Key::ESCAPE, 0, true, false),
        );
        app.update();
        let events = drain(&mut app);
        assert_eq!(events.len(), 1);
        let ev = &events[0];
        assert_eq!(ev.key_code, KeyCode::Escape);
        assert_eq!(ev.logical_key, Key::Escape);
        assert_eq!(ev.text, None);
        assert_eq!(ev.state, ButtonState::Pressed);
    }

    // (c) unmapped key + unicode 0 -> both Unidentified, still emitted
    #[test]
    fn unmapped_key_emits_unidentified() {
        let mut app = make_app();
        // Key::NONE has no keycode mapping and no unicode
        send(
            &mut app,
            godot_key_msg(godot::global::Key::NONE, 0, true, false),
        );
        app.update();
        let events = drain(&mut app);
        assert_eq!(events.len(), 1, "unmapped key must still emit an event");
        let ev = &events[0];
        assert!(
            matches!(ev.key_code, KeyCode::Unidentified(_)),
            "key_code should be Unidentified"
        );
        assert!(
            matches!(ev.logical_key, Key::Unidentified(_)),
            "logical_key should be Unidentified"
        );
    }

    // (d) echo=true -> repeat true
    #[test]
    fn echo_maps_to_repeat() {
        let mut app = make_app();
        send(
            &mut app,
            godot_key_msg(godot::global::Key::A, 'a' as u32, true, true),
        );
        app.update();
        let events = drain(&mut app);
        assert_eq!(events.len(), 1);
        assert!(events[0].repeat);
    }

    // (e) regression guard: bridge alone must not touch ButtonInput<KeyCode>
    #[test]
    fn bridge_does_not_press_button_input() {
        let mut app = make_app();
        send(
            &mut app,
            godot_key_msg(godot::global::Key::A, 'a' as u32, true, false),
        );
        app.update();
        let input = app.world().resource::<ButtonInput<KeyCode>>();
        assert!(
            input.get_just_pressed().count() == 0 && input.get_pressed().count() == 0,
            "bridge must only write events, never press ButtonInput<KeyCode> directly"
        );
    }
}
