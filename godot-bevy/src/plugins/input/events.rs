use bevy::{
    app::{App, First, Plugin},
    ecs::{
        event::{Event, EventWriter, event_update_system},
        schedule::IntoScheduleConfigs,
        system::NonSendMut,
    },
    math::Vec2,
};
use godot::{
    classes::{
        InputEvent as GodotInputEvent, InputEventJoypadButton, InputEventJoypadMotion,
        InputEventKey, InputEventMouseButton, InputEventMouseMotion, InputEventScreenTouch,
    },
    global::Key,
    obj::{EngineEnum, Gd},
};
use tracing::trace;

/// Plugin that handles Godot input events and converts them to Bevy events.
/// This is the base input plugin that provides raw input event types.
///
/// For higher-level input handling, consider using:
/// - `BevyInputBridgePlugin` for Bevy's standard input resources
/// - Custom input handling systems that read these events
#[derive(Default)]
pub struct GodotInputEventPlugin;

/// Alias for backwards compatibility
#[deprecated(note = "Use GodotInputEventPlugin instead")]
pub type GodotInputPlugin = GodotInputEventPlugin;

impl Plugin for GodotInputEventPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(First, write_input_events.before(event_update_system))
            .add_event::<KeyboardInput>()
            .add_event::<MouseButtonInput>()
            .add_event::<MouseMotion>()
            .add_event::<TouchInput>()
            .add_event::<ActionInput>()
            .add_event::<GamepadButtonInput>()
            .add_event::<GamepadAxisInput>();
    }
}

/// Keyboard key press/release event
#[derive(Debug, Event, Clone)]
pub struct KeyboardInput {
    pub keycode: Key,
    pub physical_keycode: Option<Key>,
    pub pressed: bool,
    pub echo: bool,
}

/// Mouse button press/release event
#[derive(Debug, Event, Clone)]
pub struct MouseButtonInput {
    pub button: MouseButton,
    pub pressed: bool,
    pub position: Vec2,
}

/// Mouse motion event
#[derive(Debug, Event, Clone)]
pub struct MouseMotion {
    pub delta: Vec2,
    pub position: Vec2,
}

/// Touch input event (for mobile/touchscreen)
#[derive(Debug, Event, Clone)]
pub struct TouchInput {
    pub finger_id: i32,
    pub position: Vec2,
    pub pressed: bool,
}

/// Godot action input event (for input map actions)
#[derive(Debug, Event, Clone)]
pub struct ActionInput {
    pub action: String,
    pub pressed: bool,
    pub strength: f32,
}

/// Gamepad button input event (from Godot InputEventJoypadButton)
#[derive(Debug, Event, Clone)]
pub struct GamepadButtonInput {
    pub device: i32,
    pub button_index: i32,
    pub pressed: bool,
    pub pressure: f32,
}

/// Gamepad axis input event (from Godot InputEventJoypadMotion)
#[derive(Debug, Event, Clone)]
pub struct GamepadAxisInput {
    pub device: i32,
    pub axis: i32,
    pub value: f32,
}

/// Mouse button types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MouseButton {
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

impl From<godot::global::MouseButton> for MouseButton {
    fn from(button: godot::global::MouseButton) -> Self {
        match button {
            godot::global::MouseButton::LEFT => MouseButton::Left,
            godot::global::MouseButton::RIGHT => MouseButton::Right,
            godot::global::MouseButton::MIDDLE => MouseButton::Middle,
            godot::global::MouseButton::WHEEL_UP => MouseButton::WheelUp,
            godot::global::MouseButton::WHEEL_DOWN => MouseButton::WheelDown,
            godot::global::MouseButton::WHEEL_LEFT => MouseButton::WheelLeft,
            godot::global::MouseButton::WHEEL_RIGHT => MouseButton::WheelRight,
            godot::global::MouseButton::XBUTTON1 => MouseButton::Extra1,
            godot::global::MouseButton::XBUTTON2 => MouseButton::Extra2,
            _ => MouseButton::Left, // fallback
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn write_input_events(
    events: NonSendMut<InputEventReader>,
    mut keyboard_events: EventWriter<KeyboardInput>,
    mut mouse_button_events: EventWriter<MouseButtonInput>,
    mut mouse_motion_events: EventWriter<MouseMotion>,
    mut touch_events: EventWriter<TouchInput>,
    mut action_events: EventWriter<ActionInput>,
    mut gamepad_button_events: EventWriter<GamepadButtonInput>,
    mut gamepad_axis_events: EventWriter<GamepadAxisInput>,
) {
    for (event_type, input_event) in events.0.try_iter() {
        trace!("Processing {:?} input event", event_type);

        match event_type {
            InputEventType::Normal => {
                // Only process ActionInput events from normal input (mapped keys/actions)
                extract_action_events_only(input_event, &mut action_events);
            }
            InputEventType::Unhandled => {
                // Process raw input events from unhandled input (unmapped keys, mouse, etc.)
                extract_input_events_no_actions(
                    input_event,
                    &mut keyboard_events,
                    &mut mouse_button_events,
                    &mut mouse_motion_events,
                    &mut touch_events,
                    &mut gamepad_button_events,
                    &mut gamepad_axis_events,
                );
            }
        }
    }
}

fn extract_action_events_only(
    input_event: Gd<GodotInputEvent>,
    action_events: &mut EventWriter<ActionInput>,
) {
    // Only process ActionInput events from normal input (mapped keys/actions)
    // Note: InputEventAction is not emitted by the engine, so we need to check manually
    check_action_events(&input_event, action_events);
}

fn extract_input_events_no_actions(
    input_event: Gd<GodotInputEvent>,
    keyboard_events: &mut EventWriter<KeyboardInput>,
    mouse_button_events: &mut EventWriter<MouseButtonInput>,
    mouse_motion_events: &mut EventWriter<MouseMotion>,
    touch_events: &mut EventWriter<TouchInput>,
    gamepad_button_events: &mut EventWriter<GamepadButtonInput>,
    gamepad_axis_events: &mut EventWriter<GamepadAxisInput>,
) {
    extract_basic_input_events(
        input_event,
        keyboard_events,
        mouse_button_events,
        mouse_motion_events,
        touch_events,
        gamepad_button_events,
        gamepad_axis_events,
    );
}

fn extract_basic_input_events(
    input_event: Gd<GodotInputEvent>,
    keyboard_events: &mut EventWriter<KeyboardInput>,
    mouse_button_events: &mut EventWriter<MouseButtonInput>,
    mouse_motion_events: &mut EventWriter<MouseMotion>,
    touch_events: &mut EventWriter<TouchInput>,
    gamepad_button_events: &mut EventWriter<GamepadButtonInput>,
    gamepad_axis_events: &mut EventWriter<GamepadAxisInput>,
) {
    // Try to cast to specific input event types and extract data

    // Keyboard input
    if let Ok(key_event) = input_event.clone().try_cast::<InputEventKey>() {
        keyboard_events.write(KeyboardInput {
            keycode: key_event.get_keycode(),
            physical_keycode: Some(key_event.get_physical_keycode()),
            pressed: key_event.is_pressed(),
            echo: key_event.is_echo(),
        });
    }
    // Mouse button input
    else if let Ok(mouse_button_event) = input_event.clone().try_cast::<InputEventMouseButton>() {
        let position = mouse_button_event.get_position();
        mouse_button_events.write(MouseButtonInput {
            button: mouse_button_event.get_button_index().into(),
            pressed: mouse_button_event.is_pressed(),
            position: Vec2::new(position.x, position.y),
        });
    }
    // Mouse motion
    else if let Ok(mouse_motion_event) = input_event.clone().try_cast::<InputEventMouseMotion>() {
        let position = mouse_motion_event.get_position();
        let relative = mouse_motion_event.get_relative();
        mouse_motion_events.write(MouseMotion {
            delta: Vec2::new(relative.x, relative.y),
            position: Vec2::new(position.x, position.y),
        });
    }
    // Touch input
    else if let Ok(touch_event) = input_event.clone().try_cast::<InputEventScreenTouch>() {
        let position = touch_event.get_position();
        touch_events.write(TouchInput {
            finger_id: touch_event.get_index(),
            position: Vec2::new(position.x, position.y),
            pressed: touch_event.is_pressed(),
        });
    }
    // Gamepad button input
    else if let Ok(gamepad_button_event) =
        input_event.clone().try_cast::<InputEventJoypadButton>()
    {
        gamepad_button_events.write(GamepadButtonInput {
            device: gamepad_button_event.get_device(),
            button_index: gamepad_button_event.get_button_index().ord(),
            pressed: gamepad_button_event.is_pressed(),
            pressure: gamepad_button_event.get_pressure(),
        });
    }
    // Gamepad axis input
    else if let Ok(gamepad_motion_event) =
        input_event.clone().try_cast::<InputEventJoypadMotion>()
    {
        gamepad_axis_events.write(GamepadAxisInput {
            device: gamepad_motion_event.get_device(),
            axis: gamepad_motion_event.get_axis().ord(),
            value: gamepad_motion_event.get_axis_value(),
        });
    }
}

fn check_action_events(
    input_event: &Gd<GodotInputEvent>,
    action_events: &mut EventWriter<ActionInput>,
) {
    use godot::builtin::StringName;
    use godot::classes::InputMap;

    // Get all actions from the InputMap
    let mut input_map = InputMap::singleton();
    let actions = input_map.get_actions();

    // Check each action to see if this input event matches it
    for action_variant in actions.iter_shared() {
        let action_name = action_variant.to_string();
        let action_string_name: StringName = action_name.as_str().into();

        // Check if this input event matches the action
        if input_event.is_action(&action_string_name) {
            let pressed = input_event.is_action_pressed(&action_string_name);
            let strength = input_event.get_action_strength(&action_string_name);

            trace!(
                "Generated ActionInput: '{}' {} (strength: {:.2})",
                action_name,
                if pressed { "pressed" } else { "released" },
                strength
            );

            action_events.write(ActionInput {
                action: action_name,
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
pub struct InputEventReader(pub std::sync::mpsc::Receiver<(InputEventType, Gd<GodotInputEvent>)>);
