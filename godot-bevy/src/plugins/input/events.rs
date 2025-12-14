use bevy_app::{App, First, Plugin};
use bevy_ecs::{
    message::{Message, MessageWriter, message_update_system},
    schedule::IntoScheduleConfigs,
    system::NonSendMut,
};
use bevy_math::Vec2;
use bevy_reflect::Reflect;
use godot::{
    builtin::VarDictionary,
    classes::{
        Engine, InputEvent as GodotInputEvent, InputEventJoypadButton, InputEventJoypadMotion,
        InputEventKey, InputEventMouseButton, InputEventMouseMotion, InputEventPanGesture,
        InputEventScreenTouch, Object, SceneTree,
    },
    global::Key,
    obj::{EngineEnum, Gd, Singleton},
    prelude::ToGodot,
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
            .add_message::<KeyboardInput>()
            .add_message::<MouseButtonInput>()
            .add_message::<MouseMotion>()
            .add_message::<TouchInput>()
            .add_message::<ActionInput>()
            .add_message::<GamepadButtonInput>()
            .add_message::<GamepadAxisInput>()
            .add_message::<PanGestureInput>();
    }
}

/// Keyboard key press/release event
#[derive(Debug, Message, Clone)]
pub struct KeyboardInput {
    pub keycode: Key,
    pub physical_keycode: Option<Key>,
    pub pressed: bool,
    pub echo: bool,
}

/// Mouse button press/release event
#[derive(Debug, Message, Clone)]
pub struct MouseButtonInput {
    pub button: MouseButton,
    pub pressed: bool,
    pub position: Vec2,
    pub factor: f32,
    pub canceled: bool,
    pub is_double_click: bool,
}

/// Mouse motion event
#[derive(Debug, Message, Clone)]
pub struct MouseMotion {
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

/// Mouse button types
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Reflect)]
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
fn write_input_messages(
    events: NonSendMut<InputEventReader>,
    mut keyboard_events: MessageWriter<KeyboardInput>,
    mut mouse_button_events: MessageWriter<MouseButtonInput>,
    mut mouse_motion_events: MessageWriter<MouseMotion>,
    mut touch_events: MessageWriter<TouchInput>,
    mut action_events: MessageWriter<ActionInput>,
    mut gamepad_button_events: MessageWriter<GamepadButtonInput>,
    mut gamepad_axis_events: MessageWriter<GamepadAxisInput>,
    mut pan_gesture_events: MessageWriter<PanGestureInput>,
) {
    for (event_type, input_event) in events.0.try_iter() {
        trace!("Processing {:?} input event", event_type);

        match event_type {
            InputEventType::Normal => {
                // Only process ActionInput events from normal input (mapped keys/actions)
                extract_action_events_only(&input_event, &mut action_events);
                extract_input_mouse_motion_events(&input_event, &mut mouse_motion_events);
            }
            InputEventType::Unhandled => {
                // Process raw input events from unhandled input (unmapped keys, mouse, etc.)
                extract_input_events_no_actions(
                    &input_event,
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

fn extract_action_events_only(
    input_event: &Gd<GodotInputEvent>,
    action_messages: &mut MessageWriter<ActionInput>,
) {
    // Only process ActionInput events from normal input (mapped keys/actions)
    // Note: InputEventAction is not emitted by the engine, so we need to check manually
    check_action_events(&input_event, action_messages);
}

#[allow(clippy::too_many_arguments)]
fn extract_input_mouse_motion_events(
    input_event: &Gd<GodotInputEvent>,
    mouse_motion_events: &mut MessageWriter<MouseMotion>,
) {
    extract_mouse_motion_events(input_event, mouse_motion_events);
}

#[allow(clippy::too_many_arguments)]
fn extract_input_events_no_actions(
    input_event: &Gd<GodotInputEvent>,
    keyboard_events: &mut MessageWriter<KeyboardInput>,
    mouse_button_events: &mut MessageWriter<MouseButtonInput>,
    touch_events: &mut MessageWriter<TouchInput>,
    gamepad_button_events: &mut MessageWriter<GamepadButtonInput>,
    gamepad_axis_events: &mut MessageWriter<GamepadAxisInput>,
    pan_gesture_events: &mut MessageWriter<PanGestureInput>,
) {
    extract_basic_input_events(
        input_event,
        keyboard_events,
        mouse_button_events,
        touch_events,
        gamepad_button_events,
        gamepad_axis_events,
        pan_gesture_events,
    );
}

#[allow(clippy::too_many_arguments)]
fn extract_mouse_motion_events(
    input_event: &Gd<GodotInputEvent>,
    mouse_motion_events: &mut MessageWriter<MouseMotion>,
) {
    // Try to cast to specific input event types and extract data

    // Mouse motion
    if let Ok(mouse_motion_event) = input_event.clone().try_cast::<InputEventMouseMotion>() {
        let position = mouse_motion_event.get_position();
        let relative = mouse_motion_event.get_relative();
        mouse_motion_events.write(MouseMotion {
            delta: Vec2::new(relative.x, relative.y),
            position: Vec2::new(position.x, position.y),
        });
    }
}

#[allow(clippy::too_many_arguments)]
fn extract_basic_input_events(
    input_event: &Gd<GodotInputEvent>,
    keyboard_events: &mut MessageWriter<KeyboardInput>,
    mouse_button_events: &mut MessageWriter<MouseButtonInput>,
    touch_events: &mut MessageWriter<TouchInput>,
    gamepad_button_events: &mut MessageWriter<GamepadButtonInput>,
    gamepad_axis_events: &mut MessageWriter<GamepadAxisInput>,
    pan_gesture_events: &mut MessageWriter<PanGestureInput>,
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
            factor: mouse_button_event.get_factor(),
            canceled: mouse_button_event.is_canceled(),
            is_double_click: mouse_button_event.is_double_click(),
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
    // Two-finger pan gesture
    else if let Ok(pan_gesture_event) = input_event.clone().try_cast::<InputEventPanGesture>() {
        let delta = pan_gesture_event.get_delta();
        pan_gesture_events.write(PanGestureInput {
            delta: Vec2::new(delta.x, delta.y),
        });
    }
}

fn check_action_events(
    input_event: &Gd<GodotInputEvent>,
    action_events: &mut MessageWriter<ActionInput>,
) {
    // Try to get the OptimizedBulkOperations node for bulk optimization
    let bulk_ops = (|| {
        let engine = Engine::singleton();
        let scene_tree = engine
            .get_main_loop()
            .and_then(|main_loop| main_loop.try_cast::<SceneTree>().ok())?;
        let root = scene_tree.get_root()?;
        root.get_node_or_null("BevyAppSingleton/OptimizedBulkOperations")
            .or_else(|| root.get_node_or_null("/root/BevyAppSingleton/OptimizedBulkOperations"))
            .map(|n: godot::prelude::Gd<godot::classes::Node>| n.upcast::<Object>())
    })();

    if let Some(bulk_ops_node) = bulk_ops {
        check_action_events_bulk(input_event, action_events, bulk_ops_node);
        return;
    }

    // Fallback to individual FFI calls
    check_action_events_individual(input_event, action_events);
}

fn check_action_events_bulk(
    input_event: &Gd<GodotInputEvent>,
    action_events: &mut MessageWriter<ActionInput>,
    mut batch_singleton: Gd<Object>,
) {
    let result = batch_singleton
        .call("bulk_check_actions", &[input_event.to_variant()])
        .to::<VarDictionary>();

    if let (Some(actions), Some(pressed_arr), Some(strengths)) = (
        result
            .get("actions")
            .map(|v| v.to::<godot::builtin::PackedStringArray>()),
        result
            .get("pressed")
            .map(|v| v.to::<godot::builtin::Array<bool>>()),
        result
            .get("strengths")
            .map(|v| v.to::<godot::builtin::PackedFloat32Array>()),
    ) {
        for i in 0..actions.len() {
            if let (Some(action), Some(pressed), Some(strength)) =
                (actions.get(i), pressed_arr.get(i), strengths.get(i))
            {
                let action_str = action.to_string();

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
}

fn check_action_events_individual(
    input_event: &Gd<GodotInputEvent>,
    action_events: &mut MessageWriter<ActionInput>,
) {
    use godot::classes::InputMap;

    // Get all actions from the InputMap
    let mut input_map = InputMap::singleton();
    let actions = input_map.get_actions();

    // Check each action to see if this input event matches it
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
pub struct InputEventReader(pub std::sync::mpsc::Receiver<(InputEventType, Gd<GodotInputEvent>)>);
