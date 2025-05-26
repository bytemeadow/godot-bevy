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
        InputEvent as GodotInputEvent, InputEventKey, InputEventMouseButton, InputEventMouseMotion,
        InputEventScreenTouch,
    },
    global::Key,
    obj::Gd,
};

pub struct GodotInputEventPlugin;

impl Plugin for GodotInputEventPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(First, write_input_events.before(event_update_system))
            .add_event::<KeyboardInput>()
            .add_event::<MouseButtonInput>()
            .add_event::<MouseMotion>()
            .add_event::<TouchInput>()
            .add_event::<ActionInput>();
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

fn write_input_events(
    events: NonSendMut<InputEventReader>,
    mut keyboard_events: EventWriter<KeyboardInput>,
    mut mouse_button_events: EventWriter<MouseButtonInput>,
    mut mouse_motion_events: EventWriter<MouseMotion>,
    mut touch_events: EventWriter<TouchInput>,
    mut action_events: EventWriter<ActionInput>,
) {
    for (event_type, input_event) in events.0.try_iter() {
        match event_type {
            InputEventType::Normal | InputEventType::Unhandled => {
                // Extract data from Godot input event and create thread-safe events
                extract_input_events(
                    input_event,
                    &mut keyboard_events,
                    &mut mouse_button_events,
                    &mut mouse_motion_events,
                    &mut touch_events,
                    &mut action_events,
                );
            }
        }
    }
}

fn extract_input_events(
    input_event: Gd<GodotInputEvent>,
    keyboard_events: &mut EventWriter<KeyboardInput>,
    mouse_button_events: &mut EventWriter<MouseButtonInput>,
    mouse_motion_events: &mut EventWriter<MouseMotion>,
    touch_events: &mut EventWriter<TouchInput>,
    action_events: &mut EventWriter<ActionInput>,
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

    // Action input - Check if this event matches any actions
    // Note: InputEventAction is not emitted by the engine, so we need to check manually
    check_action_events(&input_event, action_events);
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
