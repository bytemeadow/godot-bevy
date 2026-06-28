use bevy_app::{App, First, Plugin};
use bevy_ecs::{
    message::{Message, MessageWriter, message_update_system},
    schedule::IntoScheduleConfigs,
    system::NonSendMut,
};
use bevy_math::Vec2;
use bevy_reflect::Reflect;
use godot::{
    builtin::{Array, StringName},
    classes::{
        InputEvent as GodotInputEvent, InputEventJoypadButton, InputEventJoypadMotion,
        InputEventKey, InputEventMouseButton, InputEventMouseMotion, InputEventPanGesture,
        InputEventScreenTouch, InputMap,
    },
    global::Key,
    obj::{EngineEnum, Gd, Singleton},
};
use tracing::trace;

/// Plugin that handles Godot input events and converts them to Bevy messages.
/// This is the base input plugin that provides raw input message types.
///
/// For higher-level input handling, consider using:
/// - `BevyInputBridgePlugin` for Bevy's standard input resources
/// - Custom input handling systems that read these messages
#[derive(Default)]
pub struct GodotInputEventPlugin;

/// Alias for backwards compatibility
#[deprecated(note = "Use GodotInputEventPlugin instead")]
pub type GodotInputPlugin = GodotInputEventPlugin;

impl Plugin for GodotInputEventPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(First, write_input_messages.before(message_update_system))
            .add_message::<GodotKeyboardInput>()
            .add_message::<GodotMouseButtonInput>()
            .add_message::<GodotMouseMotion>()
            .add_message::<TouchInput>()
            .add_message::<ActionInput>()
            .add_message::<GamepadButtonInput>()
            .add_message::<GamepadAxisInput>()
            .add_message::<PanGestureInput>();
    }
}

/// Keyboard key press/release event.
#[derive(Debug, Message, Clone)]
pub struct GodotKeyboardInput {
    pub keycode: Key,
    pub physical_keycode: Option<Key>,
    pub pressed: bool,
    pub echo: bool,
    pub unicode: u32,
}

/// Mouse button press/release event.
#[derive(Debug, Message, Clone)]
pub struct GodotMouseButtonInput {
    pub button: GodotMouseButton,
    pub pressed: bool,
    pub position: Vec2,
    pub factor: f32,
    pub canceled: bool,
    pub is_double_click: bool,
}

/// Mouse motion event.
#[derive(Debug, Message, Clone)]
pub struct GodotMouseMotion {
    pub delta: Vec2,
    pub position: Vec2,
}

/// Touch input event (for mobile/touchscreen)
#[derive(Debug, Message, Clone)]
pub struct TouchInput {
    pub finger_id: i32,
    pub position: Vec2,
    pub pressed: bool,
}

/// Godot action input event (for input map actions)
#[derive(Debug, Message, Clone)]
pub struct ActionInput {
    pub action: String,
    pub pressed: bool,
    pub strength: f32,
}

/// Gamepad button input event (from Godot InputEventJoypadButton)
#[derive(Debug, Message, Clone)]
pub struct GamepadButtonInput {
    pub device: i32,
    pub button_index: i32,
    pub pressed: bool,
    pub pressure: f32,
}

/// Gamepad axis input event (from Godot InputEventJoypadMotion)
#[derive(Debug, Message, Clone)]
pub struct GamepadAxisInput {
    pub device: i32,
    pub axis: i32,
    pub value: f32,
}

/// Two-finger pan gesture input event (from Godot InputEventPanGesture)
#[derive(Debug, Message, Clone)]
pub struct PanGestureInput {
    pub delta: Vec2,
}

/// Mouse button types.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
pub enum GodotMouseButton {
    Left,
    Right,
    Middle,
    WheelUp,
    WheelDown,
    WheelLeft,
    WheelRight,
    Extra1,
    Extra2,
}

impl From<godot::global::MouseButton> for GodotMouseButton {
    fn from(button: godot::global::MouseButton) -> Self {
        match button {
            godot::global::MouseButton::LEFT => GodotMouseButton::Left,
            godot::global::MouseButton::RIGHT => GodotMouseButton::Right,
            godot::global::MouseButton::MIDDLE => GodotMouseButton::Middle,
            godot::global::MouseButton::WHEEL_UP => GodotMouseButton::WheelUp,
            godot::global::MouseButton::WHEEL_DOWN => GodotMouseButton::WheelDown,
            godot::global::MouseButton::WHEEL_LEFT => GodotMouseButton::WheelLeft,
            godot::global::MouseButton::WHEEL_RIGHT => GodotMouseButton::WheelRight,
            godot::global::MouseButton::XBUTTON1 => GodotMouseButton::Extra1,
            godot::global::MouseButton::XBUTTON2 => GodotMouseButton::Extra2,
            _ => GodotMouseButton::Left, // fallback
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn write_input_messages(
    events: NonSendMut<InputEventReader>,
    mut keyboard_events: MessageWriter<GodotKeyboardInput>,
    mut mouse_button_events: MessageWriter<GodotMouseButtonInput>,
    mut mouse_motion_events: MessageWriter<GodotMouseMotion>,
    mut touch_events: MessageWriter<TouchInput>,
    mut action_events: MessageWriter<ActionInput>,
    mut gamepad_button_events: MessageWriter<GamepadButtonInput>,
    mut gamepad_axis_events: MessageWriter<GamepadAxisInput>,
    mut pan_gesture_events: MessageWriter<PanGestureInput>,
) {
    // Fetch once per frame, not per event: get_actions() is an allocating FFI
    // call and the action set is stable within a frame.
    let actions = InputMap::singleton().get_actions();

    for (event_type, input_event) in events.0.try_iter() {
        trace!("Processing {:?} input event", event_type);

        match event_type {
            InputEventType::Normal => {
                check_action_events(&input_event, &mut action_events, &actions);
                extract_mouse_motion_events(input_event, &mut mouse_motion_events);
            }
            InputEventType::Unhandled => {
                extract_basic_input_events(
                    input_event,
                    &mut keyboard_events,
                    &mut mouse_button_events,
                    &mut touch_events,
                    &mut gamepad_button_events,
                    &mut gamepad_axis_events,
                    &mut pan_gesture_events,
                );
            }
        }
    }
}

fn extract_mouse_motion_events(
    input_event: Gd<GodotInputEvent>,
    mouse_motion_events: &mut MessageWriter<GodotMouseMotion>,
) {
    if let Ok(mouse_motion_event) = input_event.try_cast::<InputEventMouseMotion>() {
        let position = mouse_motion_event.get_position();
        let relative = mouse_motion_event.get_relative();
        mouse_motion_events.write(GodotMouseMotion {
            delta: Vec2::new(relative.x, relative.y),
            position: Vec2::new(position.x, position.y),
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn extract_basic_input_events(
    input_event: Gd<GodotInputEvent>,
    keyboard_events: &mut MessageWriter<GodotKeyboardInput>,
    mouse_button_events: &mut MessageWriter<GodotMouseButtonInput>,
    touch_events: &mut MessageWriter<TouchInput>,
    gamepad_button_events: &mut MessageWriter<GamepadButtonInput>,
    gamepad_axis_events: &mut MessageWriter<GamepadAxisInput>,
    pan_gesture_events: &mut MessageWriter<PanGestureInput>,
) {
    // Keyboard input
    let input_event = match input_event.try_cast::<InputEventKey>() {
        Ok(key_event) => {
            keyboard_events.write(GodotKeyboardInput {
                keycode: key_event.get_keycode(),
                physical_keycode: Some(key_event.get_physical_keycode()),
                pressed: key_event.is_pressed(),
                echo: key_event.is_echo(),
                unicode: key_event.get_unicode(),
            });
            return;
        }
        Err(original) => original,
    };

    // Mouse button input
    let input_event = match input_event.try_cast::<InputEventMouseButton>() {
        Ok(mouse_button_event) => {
            let position = mouse_button_event.get_position();
            mouse_button_events.write(GodotMouseButtonInput {
                button: mouse_button_event.get_button_index().into(),
                pressed: mouse_button_event.is_pressed(),
                position: Vec2::new(position.x, position.y),
                factor: mouse_button_event.get_factor(),
                canceled: mouse_button_event.is_canceled(),
                is_double_click: mouse_button_event.is_double_click(),
            });
            return;
        }
        Err(original) => original,
    };

    // Touch input
    let input_event = match input_event.try_cast::<InputEventScreenTouch>() {
        Ok(touch_event) => {
            let position = touch_event.get_position();
            touch_events.write(TouchInput {
                finger_id: touch_event.get_index(),
                position: Vec2::new(position.x, position.y),
                pressed: touch_event.is_pressed(),
            });
            return;
        }
        Err(original) => original,
    };

    // Gamepad button input
    let input_event = match input_event.try_cast::<InputEventJoypadButton>() {
        Ok(gamepad_button_event) => {
            gamepad_button_events.write(GamepadButtonInput {
                device: gamepad_button_event.get_device(),
                button_index: gamepad_button_event.get_button_index().ord(),
                pressed: gamepad_button_event.is_pressed(),
                pressure: gamepad_button_event.get_pressure(),
            });
            return;
        }
        Err(original) => original,
    };

    // Gamepad axis input
    let input_event = match input_event.try_cast::<InputEventJoypadMotion>() {
        Ok(gamepad_motion_event) => {
            gamepad_axis_events.write(GamepadAxisInput {
                device: gamepad_motion_event.get_device(),
                axis: gamepad_motion_event.get_axis().ord(),
                value: gamepad_motion_event.get_axis_value(),
            });
            return;
        }
        Err(original) => original,
    };

    // Two-finger pan gesture
    if let Ok(pan_gesture_event) = input_event.try_cast::<InputEventPanGesture>() {
        let delta = pan_gesture_event.get_delta();
        pan_gesture_events.write(PanGestureInput {
            delta: Vec2::new(delta.x, delta.y),
        });
    }
}

fn check_action_events(
    input_event: &Gd<GodotInputEvent>,
    action_events: &mut MessageWriter<ActionInput>,
    actions: &Array<StringName>,
) {
    for action_name in actions.iter_shared() {
        if input_event.is_action(&action_name) {
            let pressed = input_event.is_action_pressed(&action_name);
            let strength = input_event.get_action_strength(&action_name);
            let action_str = action_name.to_string();

            trace!(
                "Generated ActionInput: '{}' {} (strength: {:.2})",
                action_str,
                if pressed { "pressed" } else { "released" },
                strength
            );

            action_events.write(ActionInput {
                action: action_str,
                pressed,
                strength,
            });
        }
    }
}

#[doc(hidden)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum InputEventType {
    Normal,
    Unhandled,
}

#[doc(hidden)]
pub struct InputEventReader(pub crossbeam_channel::Receiver<(InputEventType, Gd<GodotInputEvent>)>);
